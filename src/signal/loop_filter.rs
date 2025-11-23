/// Second-order loop filter for DLL/PLL
#[derive(Debug, Clone)]
pub struct LoopFilter {
    bandwidth_hz: f64,
    damping: f64,
    sample_time: f64,

    // Filter coefficients
    k1: f64,
    k2: f64,

    // State
    integrator: f64,
}

impl LoopFilter {
    pub fn new(bandwidth_hz: f64, damping: f64, sample_time: f64) -> Self {
        // Calculate filter coefficients for a second-order loop
        let wn = bandwidth_hz * 8.0 * damping / (4.0 * damping * damping + 1.0);
        let k1 = 2.0 * damping * wn * sample_time;
        let k2 = wn * wn * sample_time * sample_time;

        Self {
            bandwidth_hz,
            damping,
            sample_time,
            k1,
            k2,
            integrator: 0.0,
        }
    }

    /// Update the filter with a new error measurement
    pub fn update(&mut self, error: f64) -> f64 {
        // Proportional term
        let proportional = self.k1 * error;

        // Integral term
        self.integrator += self.k2 * error;

        proportional + self.integrator
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        self.integrator = 0.0;
    }

    /// Set a new bandwidth
    pub fn set_bandwidth(&mut self, bandwidth_hz: f64) {
        self.bandwidth_hz = bandwidth_hz;
        let wn = bandwidth_hz * 8.0 * self.damping / (4.0 * self.damping * self.damping + 1.0);
        self.k1 = 2.0 * self.damping * wn * self.sample_time;
        self.k2 = wn * wn * self.sample_time * self.sample_time;
    }
}

/// First-order loop filter (simpler, for FLL)
#[derive(Debug, Clone)]
pub struct FirstOrderLoopFilter {
    gain: f64,
}

impl FirstOrderLoopFilter {
    pub fn new(bandwidth_hz: f64, sample_time: f64) -> Self {
        Self {
            gain: bandwidth_hz * sample_time,
        }
    }

    pub fn update(&mut self, error: f64) -> f64 {
        self.gain * error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_filter() {
        let mut filter = LoopFilter::new(25.0, 0.707, 0.001);

        // Test with a constant error
        let correction1 = filter.update(1.0);
        let correction2 = filter.update(1.0);

        // Integrator should cause increasing corrections
        assert!(correction2 > correction1);
    }

    #[test]
    fn test_filter_reset() {
        let mut filter = LoopFilter::new(25.0, 0.707, 0.001);

        filter.update(1.0);
        filter.reset();

        let correction = filter.update(1.0);
        // After reset, should be back to initial state
        assert!(correction.abs() < 0.1);
    }
}
