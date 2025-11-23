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
        tracing::debug!("Acquiring PRN {}", prn);

        let samples_per_code = (self.sample_rate * GPS_CA_CODE_LENGTH as f64
            / GPS_CA_CHIPPING_RATE) as usize;

        // Generate the C/A code for this PRN at the sample rate
        let ca_code =
            CaCodeGenerator::generate_sampled(prn, self.sample_rate, GPS_CA_CHIPPING_RATE);

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
            tracing::debug!(
                "PRN {} not acquired: peak/mean={:.2} < {:.2}",
                prn,
                peak_to_mean,
                self.config.threshold
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
        let mut max_peak_to_mean = 0.0_f64;
        let mut best_prn = 0_u8;

        for &prn in prns {
            if let Ok(result) = self.acquire(signal, prn) {
                let peak_to_mean = result.peak_metric / result.mean_metric.max(1e-10);
                if peak_to_mean > max_peak_to_mean {
                    max_peak_to_mean = peak_to_mean;
                    best_prn = prn;
                }

                if result.acquired {
                    results.push(result);
                }
            }
        }

        if results.is_empty() && prns.len() > 0 {
            tracing::info!(
                "No satellites acquired (best: PRN {} with peak/mean={:.2}, need {:.2})",
                best_prn,
                max_peak_to_mean,
                self.config.threshold
            );
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
