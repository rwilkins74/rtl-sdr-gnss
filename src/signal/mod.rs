pub mod correlator;
pub mod discriminator;
pub mod loop_filter;

use num_complex::Complex;
use rustfft::{FftPlanner, num_complex::Complex as FftComplex};

/// Correlate two signals using FFT-based circular correlation
pub fn fft_correlate(signal: &[Complex<f32>], code: &[f32]) -> Vec<f32> {
    let n = signal.len();
    assert_eq!(n, code.len(), "Signal and code must have same length");

    // Create FFT planner
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    let ifft = planner.plan_fft_inverse(n);

    // Convert to FFT complex type and compute FFT of signal
    let mut signal_fft: Vec<FftComplex<f32>> = signal
        .iter()
        .map(|&c| FftComplex::new(c.re, c.im))
        .collect();
    fft.process(&mut signal_fft);

    // Convert code to complex and compute FFT
    let mut code_fft: Vec<FftComplex<f32>> = code
        .iter()
        .map(|&c| FftComplex::new(c, 0.0))
        .collect();
    fft.process(&mut code_fft);

    // Multiply signal FFT by conjugate of code FFT
    let mut correlation_fft: Vec<FftComplex<f32>> = signal_fft
        .iter()
        .zip(code_fft.iter())
        .map(|(s, c)| s * c.conj())
        .collect();

    // IFFT to get correlation
    ifft.process(&mut correlation_fft);

    // Return magnitude
    correlation_fft
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt() / n as f32)
        .collect()
}

/// Apply carrier wipeoff to signal
pub fn carrier_wipeoff(
    signal: &[Complex<f32>],
    carrier_freq: f64,
    sample_rate: f64,
    phase_offset: f64,
) -> Vec<Complex<f32>> {
    signal
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let phase = 2.0 * std::f64::consts::PI * carrier_freq * (i as f64) / sample_rate
                + phase_offset;
            // Back to original e^(-jφ) for proper carrier removal
            let carrier = Complex::new(phase.cos() as f32, -phase.sin() as f32);
            s * carrier
        })
        .collect()
}

/// Compute power spectrum using FFT
pub fn power_spectrum(signal: &[Complex<f32>]) -> Vec<f32> {
    let n = signal.len();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    let mut spectrum: Vec<FftComplex<f32>> = signal
        .iter()
        .map(|&c| FftComplex::new(c.re, c.im))
        .collect();

    fft.process(&mut spectrum);

    spectrum
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im) / (n as f32))
        .collect()
}

/// Estimate carrier-to-noise ratio (C/N0) in dB-Hz
pub fn estimate_cn0(
    prompt_power: f64,
    noise_power: f64,
    integration_time_ms: f64,
) -> f64 {
    let snr = prompt_power / noise_power;
    let bandwidth_hz = 1000.0 / integration_time_ms;
    10.0 * (snr / bandwidth_hz).log10()
}

/// Circular cross-correlation (time-domain)
pub fn correlate(signal: &[Complex<f32>], code: &[f32], offset: usize) -> Complex<f32> {
    let n = signal.len();
    let mut sum = Complex::new(0.0, 0.0);

    for i in 0..n {
        let code_idx = (i + offset) % code.len();
        sum += signal[i] * code[code_idx];
    }

    sum / (n as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_correlate() {
        let signal = vec![Complex::new(1.0, 0.0); 1024];
        let code = vec![1.0; 1024];
        let result = fft_correlate(&signal, &code);
        assert_eq!(result.len(), 1024);
    }

    #[test]
    fn test_carrier_wipeoff() {
        let signal = vec![Complex::new(1.0, 0.0); 1024];
        let result = carrier_wipeoff(&signal, 1000.0, 2_048_000.0, 0.0);
        assert_eq!(result.len(), 1024);
    }
}
