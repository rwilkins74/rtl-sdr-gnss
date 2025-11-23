use num_complex::Complex;

/// DLL discriminator - Normalized Early Minus Late Power
pub fn dll_nemlp(early: Complex<f32>, late: Complex<f32>) -> f64 {
    let e_power = early.norm_sqr();
    let l_power = late.norm_sqr();

    let discriminator = (e_power - l_power) / (e_power + l_power);
    discriminator as f64
}

/// DLL discriminator - Early Minus Late Envelope
pub fn dll_emle(early: Complex<f32>, late: Complex<f32>) -> f64 {
    let e_mag = early.norm();
    let l_mag = late.norm();

    ((e_mag - l_mag) / (e_mag + l_mag)) as f64
}

/// PLL discriminator - Costas (for BPSK)
pub fn pll_costas(prompt: Complex<f32>) -> f64 {
    let discriminator = prompt.im.atan2(prompt.re);
    discriminator as f64
}

/// PLL discriminator - Decision-directed (for known data)
pub fn pll_decision_directed(prompt: Complex<f32>, data_bit: i8) -> f64 {
    let expected_phase = if data_bit > 0 { 0.0 } else { std::f32::consts::PI };
    let phase = prompt.im.atan2(prompt.re);
    let mut error = phase - expected_phase;

    // Wrap to [-pi, pi]
    while error > std::f32::consts::PI {
        error -= 2.0 * std::f32::consts::PI;
    }
    while error < -std::f32::consts::PI {
        error += 2.0 * std::f32::consts::PI;
    }

    error as f64
}

/// FLL discriminator - Four-quadrant arctangent
pub fn fll_atan2(prompt_prev: Complex<f32>, prompt_curr: Complex<f32>, t: f64) -> f64 {
    // Cross product
    let cross = prompt_curr.re * prompt_prev.im - prompt_curr.im * prompt_prev.re;
    // Dot product
    let dot = prompt_curr.re * prompt_prev.re + prompt_curr.im * prompt_prev.im;

    let freq_error = cross.atan2(dot) / (2.0 * std::f32::consts::PI * t as f32);
    freq_error as f64
}

/// Calculate CN0 using Narrowband-Wideband Power Ratio method
pub fn cn0_nbwbpr(
    prompt_power: f64,
    noise_power: f64,
    integration_time_s: f64,
) -> f64 {
    let snr = prompt_power / noise_power.max(1e-10);
    let cn0 = 10.0 * (snr / integration_time_s).log10();
    cn0.max(0.0) // Ensure non-negative
}

/// Calculate CN0 using Beaulieu's formula
pub fn cn0_beaulieu(
    i_values: &[f32],
    q_values: &[f32],
    integration_time_s: f64,
) -> f64 {
    let n = i_values.len() as f64;

    // Calculate mean power
    let mean_power: f64 = i_values
        .iter()
        .zip(q_values.iter())
        .map(|(&i, &q)| (i * i + q * q) as f64)
        .sum::<f64>()
        / n;

    // Calculate variance of power
    let power_variance: f64 = i_values
        .iter()
        .zip(q_values.iter())
        .map(|(&i, &q)| {
            let power = (i * i + q * q) as f64;
            (power - mean_power).powi(2)
        })
        .sum::<f64>()
        / n;

    // SNR estimate
    let snr = (mean_power * mean_power / power_variance - 1.0).max(0.0);

    // CN0 in dB-Hz
    10.0 * (snr / integration_time_s).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dll_nemlp() {
        let early = Complex::new(1.0, 0.0);
        let late = Complex::new(0.5, 0.0);
        let error = dll_nemlp(early, late);
        // Early is stronger, so error should be positive
        assert!(error > 0.0);
    }

    #[test]
    fn test_pll_costas() {
        let prompt = Complex::new(1.0, 0.1);
        let error = pll_costas(prompt);
        // Small imaginary part means small phase error
        assert!(error.abs() < 0.2);
    }

    #[test]
    fn test_cn0_calculation() {
        let cn0 = cn0_nbwbpr(100.0, 1.0, 0.001);
        // Should be a reasonable CN0 value (> 0 dB-Hz)
        assert!(cn0 > 40.0 && cn0 < 60.0);
    }
}
