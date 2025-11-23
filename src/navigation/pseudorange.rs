use crate::constants::*;

#[derive(Debug, Clone)]
pub struct Pseudorange {
    pub prn: u8,
    pub pseudorange: f64,    // meters
    pub pseudorange_rate: f64, // meters/second
    pub carrier_phase: f64,  // cycles
    pub cn0_db_hz: f64,
}

impl Pseudorange {
    /// Calculate pseudorange from code phase and time
    pub fn from_code_phase(
        prn: u8,
        code_phase_chips: f64,
        carrier_freq_hz: f64,
        cn0_db_hz: f64,
        rx_time_s: f64,
        tx_time_s: f64,
    ) -> Self {
        // Pseudorange = (receive time - transmit time) * speed of light
        // This is a simplified calculation
        let time_of_flight = rx_time_s - tx_time_s;
        let pseudorange = time_of_flight * SPEED_OF_LIGHT;

        // Pseudorange rate from Doppler
        let doppler_hz = carrier_freq_hz - GPS_L1_FREQ_HZ;
        let pseudorange_rate = -doppler_hz * SPEED_OF_LIGHT / GPS_L1_FREQ_HZ;

        Self {
            prn,
            pseudorange,
            pseudorange_rate,
            carrier_phase: 0.0,
            cn0_db_hz,
        }
    }

    /// Calculate pseudorange from tracking loop measurements
    pub fn from_tracking(
        prn: u8,
        code_phase_chips: f64,
        code_freq_hz: f64,
        carrier_freq_hz: f64,
        carrier_phase_rad: f64,
        cn0_db_hz: f64,
        lock_time_ms: u64,
    ) -> Self {
        // Convert code phase to distance
        // 1 chip = (speed of light / chipping rate) meters
        let chip_length = SPEED_OF_LIGHT / GPS_CA_CHIPPING_RATE;

        // Total pseudorange includes:
        // 1. Complete milliseconds (from lock time and nav message)
        // 2. Fractional millisecond from code phase
        let ms_count = lock_time_ms as f64;
        let fractional_ms = code_phase_chips / GPS_CA_CODE_LENGTH as f64;
        let total_ms = ms_count + fractional_ms;

        // Pseudorange in meters
        let pseudorange = total_ms / 1000.0 * SPEED_OF_LIGHT;

        // Pseudorange rate from Doppler
        let doppler_hz = carrier_freq_hz;
        let pseudorange_rate = -doppler_hz * SPEED_OF_LIGHT / GPS_L1_FREQ_HZ;

        // Carrier phase in cycles
        let carrier_phase = carrier_phase_rad / (2.0 * std::f64::consts::PI);

        Self {
            prn,
            pseudorange,
            pseudorange_rate,
            carrier_phase,
            cn0_db_hz,
        }
    }

    /// Apply clock correction to pseudorange
    pub fn apply_clock_correction(&mut self, clock_correction_s: f64) {
        self.pseudorange -= clock_correction_s * SPEED_OF_LIGHT;
    }

    /// Apply ionospheric correction
    pub fn apply_iono_correction(&mut self, iono_delay_m: f64) {
        self.pseudorange -= iono_delay_m;
    }

    /// Apply tropospheric correction
    pub fn apply_tropo_correction(&mut self, tropo_delay_m: f64) {
        self.pseudorange -= tropo_delay_m;
    }

    /// Get weight for weighted least squares (based on CN0)
    pub fn weight(&self) -> f64 {
        // Higher CN0 = higher weight
        // Convert from dB-Hz to linear and normalize
        let cn0_linear = 10f64.powf(self.cn0_db_hz / 10.0);
        cn0_linear / 1e6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pseudorange_from_tracking() {
        let pr = Pseudorange::from_tracking(
            1,
            512.0,  // code phase
            GPS_CA_CHIPPING_RATE,
            1500.0, // Doppler
            0.0,
            45.0,   // CN0
            100,    // lock time ms
        );

        assert_eq!(pr.prn, 1);
        assert!(pr.pseudorange > 0.0);
        assert!(pr.cn0_db_hz == 45.0);
    }

    #[test]
    fn test_weight_calculation() {
        let pr = Pseudorange::from_tracking(1, 0.0, GPS_CA_CHIPPING_RATE, 0.0, 0.0, 40.0, 100);
        let weight = pr.weight();
        assert!(weight > 0.0);
    }
}
