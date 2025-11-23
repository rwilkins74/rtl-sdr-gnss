pub mod decoder;
pub mod corrections;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbasMessage {
    pub prn: u8,
    pub message_type: u8,
    pub data: Vec<u8>,
    pub time: f64,
}

#[derive(Debug, Clone)]
pub struct SbasCorrections {
    pub prn: u8,
    pub fast_corrections: Vec<f64>,
    pub long_term_corrections: Vec<f64>,
    pub iono_corrections: Vec<f64>,
}

impl SbasCorrections {
    pub fn new(prn: u8) -> Self {
        Self {
            prn,
            fast_corrections: Vec::new(),
            long_term_corrections: Vec::new(),
            iono_corrections: Vec::new(),
        }
    }

    pub fn apply_to_pseudorange(&self, satellite_prn: u8, pseudorange: f64) -> f64 {
        // Simplified SBAS correction application
        // In production, you'd look up the appropriate correction for this satellite
        let mut corrected = pseudorange;

        // Apply fast corrections (typically < 1 meter)
        if let Some(&fc) = self.fast_corrections.get(satellite_prn as usize) {
            corrected -= fc;
        }

        // Apply long-term corrections
        if let Some(&ltc) = self.long_term_corrections.get(satellite_prn as usize) {
            corrected -= ltc;
        }

        corrected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbas_corrections() {
        let corr = SbasCorrections::new(131);
        assert_eq!(corr.prn, 131);
        assert!(corr.fast_corrections.is_empty());
    }
}
