use num_complex::Complex;

/// Three-point correlator (Early, Prompt, Late) for DLL
#[derive(Debug, Clone)]
pub struct Correlator {
    early_late_spacing: f64, // in chips
}

impl Correlator {
    pub fn new(early_late_spacing: f64) -> Self {
        Self {
            early_late_spacing,
        }
    }

    /// Correlate signal with E/P/L versions of the code
    pub fn correlate_epl(
        &self,
        signal: &[Complex<f32>],
        code: &[f32],
        code_phase: f64,
        samples_per_chip: f64,
    ) -> (Complex<f32>, Complex<f32>, Complex<f32>) {
        let early = self.correlate_at_offset(
            signal,
            code,
            code_phase - self.early_late_spacing,
            samples_per_chip,
        );

        let prompt = self.correlate_at_offset(signal, code, code_phase, samples_per_chip);

        let late = self.correlate_at_offset(
            signal,
            code,
            code_phase + self.early_late_spacing,
            samples_per_chip,
        );

        (early, prompt, late)
    }

    /// Correlate signal with code at a specific offset
    fn correlate_at_offset(
        &self,
        signal: &[Complex<f32>],
        code: &[f32],
        code_phase_chips: f64,
        samples_per_chip: f64,
    ) -> Complex<f32> {
        let mut sum = Complex::new(0.0, 0.0);
        let n = signal.len();
        let code_len = code.len();

        for i in 0..n {
            // Calculate which code chip this sample corresponds to
            let code_pos = (i as f64 / samples_per_chip + code_phase_chips) % (code_len as f64);
            let code_idx = code_pos as usize % code_len;

            sum += signal[i] * code[code_idx];
        }

        sum / (n as f32)
    }

    /// Five-point correlator for Very Early and Very Late (useful for multipath detection)
    pub fn correlate_vevl(
        &self,
        signal: &[Complex<f32>],
        code: &[f32],
        code_phase: f64,
        samples_per_chip: f64,
        ve_spacing: f64,
        vl_spacing: f64,
    ) -> (
        Complex<f32>,
        Complex<f32>,
        Complex<f32>,
        Complex<f32>,
        Complex<f32>,
    ) {
        let very_early = self.correlate_at_offset(signal, code, code_phase - ve_spacing, samples_per_chip);
        let early = self.correlate_at_offset(signal, code, code_phase - self.early_late_spacing, samples_per_chip);
        let prompt = self.correlate_at_offset(signal, code, code_phase, samples_per_chip);
        let late = self.correlate_at_offset(signal, code, code_phase + self.early_late_spacing, samples_per_chip);
        let very_late = self.correlate_at_offset(signal, code, code_phase + vl_spacing, samples_per_chip);

        (very_early, early, prompt, late, very_late)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlator_epl() {
        let correlator = Correlator::new(0.5);

        let signal = vec![Complex::new(1.0, 0.0); 1024];
        let code = vec![1.0; 1023];

        let (early, prompt, late) = correlator.correlate_epl(&signal, &code, 0.0, 2.0);

        // All should be non-zero
        assert!(prompt.norm() > 0.0);
        assert!(early.norm() > 0.0);
        assert!(late.norm() > 0.0);
    }
}
