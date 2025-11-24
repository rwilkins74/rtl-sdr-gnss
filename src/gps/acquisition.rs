use crate::config::AcquisitionConfig;
use crate::constants::*;
use crate::gps::ca_code::CaCodeGenerator;
use crate::signal;
use crate::Result;
use num_complex::Complex;

#[derive(Debug, Clone)]
pub struct AcquisitionResult {
    pub prn: u8,
    pub doppler_hz: f64,
    pub code_phase: usize,
    pub peak_metric: f64,
    pub mean_metric: f64,
    pub acquired: bool,
}

pub struct Acquisition {
    config: AcquisitionConfig,
    sample_rate: f64,
}

impl Acquisition {
    pub fn new(config: AcquisitionConfig, sample_rate: f64) -> Self {
        Self {
            config,
            sample_rate,
        }
    }

    /// Acquire a satellite signal
    pub fn acquire(&self, signal: &[Complex<f32>], prn: u8) -> Result<AcquisitionResult> {
        tracing::debug!("Acquiring PRN {} with {}ms coherent integration",
                       prn, self.config.coherent_integration_ms);

        // On first PRN, show signal diagnostics
        if prn == 1 {
            let power: f32 = signal.iter().map(|s| s.norm_sqr()).sum::<f32>() / signal.len() as f32;
            let dc_i: f32 = signal.iter().map(|s| s.re).sum::<f32>() / signal.len() as f32;
            let dc_q: f32 = signal.iter().map(|s| s.im).sum::<f32>() / signal.len() as f32;
            let dc_offset = (dc_i * dc_i + dc_q * dc_q).sqrt();

            tracing::info!(
                "Signal diagnostics: power={:.2e}, DC_offset={:.2e} (I={:.2e}, Q={:.2e}), samples={}",
                power, dc_offset, dc_i, dc_q, signal.len()
            );
        }

        let samples_per_code_period = (self.sample_rate * GPS_CA_CODE_LENGTH as f64
            / GPS_CA_CHIPPING_RATE) as usize;

        // Use configured coherent integration time
        let samples_per_code = samples_per_code_period * self.config.coherent_integration_ms;

        // Generate the C/A code for this PRN at the sample rate, repeated for coherent integration
        let mut ca_code = Vec::with_capacity(samples_per_code);
        let single_code = CaCodeGenerator::generate_sampled(prn, self.sample_rate, GPS_CA_CHIPPING_RATE);
        for _ in 0..self.config.coherent_integration_ms {
            ca_code.extend_from_slice(&single_code);
        }

        // Ensure we have enough signal samples
        let signal = if signal.len() >= samples_per_code {
            &signal[..samples_per_code]
        } else {
            tracing::warn!(
                "Not enough samples for acquisition: {} < {}",
                signal.len(),
                samples_per_code
            );
            return Ok(AcquisitionResult {
                prn,
                doppler_hz: 0.0,
                code_phase: 0,
                peak_metric: 0.0,
                mean_metric: 0.0,
                acquired: false,
            });
        };

        let mut best_doppler = 0.0;
        let mut best_code_phase = 0;
        let mut best_metric = 0.0;
        let mut all_metrics = Vec::new();

        // Search over Doppler frequencies
        let doppler_bins =
            (2.0 * self.config.max_doppler_hz / self.config.doppler_step_hz) as i32 + 1;

        for doppler_bin in 0..doppler_bins {
            let doppler = -self.config.max_doppler_hz
                + (doppler_bin as f64) * self.config.doppler_step_hz;

            // Apply carrier wipeoff
            let signal_wiped = signal::carrier_wipeoff(signal, doppler, self.sample_rate, 0.0);

            // Correlate with C/A code using FFT
            let correlation = signal::fft_correlate(&signal_wiped, &ca_code[..samples_per_code]);

            // Find peak in correlation
            let (max_idx, max_val) = correlation
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            all_metrics.push(*max_val);

            if *max_val > best_metric {
                best_metric = *max_val;
                best_doppler = doppler;
                best_code_phase = max_idx;
            }
        }

        // Calculate mean of all metrics for threshold comparison
        let mean_metric = all_metrics.iter().sum::<f32>() / all_metrics.len() as f32;
        let peak_to_mean = best_metric / mean_metric;

        let acquired = peak_to_mean > self.config.threshold as f32;

        if acquired {
            tracing::info!(
                "PRN {} acquired: Doppler={:.1} Hz, Code Phase={}, Metric={:.2}",
                prn,
                best_doppler,
                best_code_phase,
                peak_to_mean
            );
        } else {
            tracing::info!(
                "PRN {} not acquired: peak/mean={:.2} < {:.2} (peak={:.3e}, mean={:.3e})",
                prn,
                peak_to_mean,
                self.config.threshold,
                best_metric,
                mean_metric
            );
        }

        Ok(AcquisitionResult {
            prn,
            doppler_hz: best_doppler,
            code_phase: best_code_phase,
            peak_metric: best_metric as f64,
            mean_metric: mean_metric as f64,
            acquired,
        })
    }

    /// Acquire all GPS satellites in parallel
    pub async fn acquire_all(
        &self,
        signal: &[Complex<f32>],
        prns: &[u8],
    ) -> Vec<AcquisitionResult> {
        let mut results = Vec::new();
        let mut all_results = Vec::new();

        for &prn in prns {
            if let Ok(result) = self.acquire(signal, prn) {
                let peak_to_mean = result.peak_metric / result.mean_metric.max(1e-10);
                all_results.push((prn, peak_to_mean, result.doppler_hz, result.acquired));

                if result.acquired {
                    results.push(result);
                }
            }
        }

        // Sort by peak-to-mean ratio and show top 5
        all_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        if results.is_empty() && !all_results.is_empty() {
            let top5: Vec<String> = all_results.iter()
                .take(5)
                .map(|(prn, ratio, doppler, _)| format!("PRN{}({:.2}@{:.0}Hz)", prn, ratio, doppler))
                .collect();

            tracing::info!(
                "No satellites acquired. Top 5 correlations: {} (need {:.2})",
                top5.join(", "),
                self.config.threshold
            );
        } else if !results.is_empty() {
            tracing::info!("Acquired {} satellite(s)", results.len());
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquisition_result() {
        let config = AcquisitionConfig {
            coherent_integration_ms: 1,
            max_doppler_hz: 10_000.0,
            doppler_step_hz: 500.0,
            threshold: 2.5,
        };

        let acq = Acquisition::new(config, 2_048_000.0);

        // Create a simple test signal (noise)
        let signal: Vec<Complex<f32>> = (0..4096)
            .map(|_| Complex::new(0.01, 0.01))
            .collect();

        let result = acq.acquire(&signal, 1).unwrap();
        assert_eq!(result.prn, 1);
        // Noise should not be acquired
        assert!(!result.acquired);
    }
}
