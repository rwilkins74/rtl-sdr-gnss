use crate::constants::GPS_CA_CODE_LENGTH;

/// GPS C/A code generator using Gold codes
pub struct CaCodeGenerator;

impl CaCodeGenerator {
    /// Generate C/A code for a given PRN (1-32 for GPS, 120-158 for SBAS)
    pub fn generate(prn: u8) -> Vec<i8> {
        let (g2_delay1, g2_delay2) = Self::get_g2_delays(prn);
        let mut code = vec![0i8; GPS_CA_CODE_LENGTH];

        // G1 LFSR: x^10 + x^3 + 1
        let mut g1 = [1i8; 10];
        // G2 LFSR: x^10 + x^9 + x^8 + x^6 + x^3 + x^2 + 1
        let mut g2 = [1i8; 10];

        for i in 0..GPS_CA_CODE_LENGTH {
            // Output is XOR of G1 and selected G2 taps
            let g1_out = g1[9];
            let g2_out = g2[g2_delay1] ^ g2[g2_delay2];
            code[i] = if (g1_out ^ g2_out) == 1 { 1 } else { -1 };

            // Shift G1
            let g1_feedback = g1[2] ^ g1[9];
            for j in (1..10).rev() {
                g1[j] = g1[j - 1];
            }
            g1[0] = g1_feedback;

            // Shift G2
            let g2_feedback = g2[1] ^ g2[2] ^ g2[5] ^ g2[7] ^ g2[8] ^ g2[9];
            for j in (1..10).rev() {
                g2[j] = g2[j - 1];
            }
            g2[0] = g2_feedback;
        }

        code
    }

    /// Generate upsampled C/A code at given sampling rate
    pub fn generate_sampled(prn: u8, sample_rate: f64, code_freq: f64) -> Vec<f32> {
        let code = Self::generate(prn);
        let samples_per_chip = sample_rate / code_freq;
        let total_samples = (GPS_CA_CODE_LENGTH as f64 * samples_per_chip) as usize;

        let mut sampled_code = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let chip_index = ((i as f64) / samples_per_chip) as usize % GPS_CA_CODE_LENGTH;
            sampled_code.push(code[chip_index] as f32);
        }

        sampled_code
    }

    /// Get G2 register tap delays for each PRN
    /// Returns (delay1, delay2) as indices into the G2 shift register
    fn get_g2_delays(prn: u8) -> (usize, usize) {
        match prn {
            1 => (1, 5),
            2 => (2, 6),
            3 => (3, 7),
            4 => (4, 8),
            5 => (0, 8),
            6 => (1, 9),
            7 => (0, 7),
            8 => (1, 8),
            9 => (2, 9),
            10 => (1, 2),
            11 => (2, 3),
            12 => (4, 5),
            13 => (5, 6),
            14 => (6, 7),
            15 => (7, 8),
            16 => (8, 9),
            17 => (0, 3),
            18 => (1, 4),
            19 => (2, 5),
            20 => (3, 6),
            21 => (4, 7),
            22 => (5, 8),
            23 => (0, 2),
            24 => (3, 5),
            25 => (4, 6),
            26 => (5, 7),
            27 => (6, 8),
            28 => (7, 9),
            29 => (0, 5),
            30 => (1, 6),
            31 => (2, 7),
            32 => (3, 8),
            // SBAS satellites (PRN 120-158) use different tap delays
            120..=158 => {
                // Simplified SBAS code generation (same structure, different taps)
                // In production, you'd want the full SBAS code table
                let offset = (prn - 120) as usize;
                ((offset % 9), ((offset + 3) % 9 + 1))
            }
            _ => panic!("Invalid PRN: {}", prn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ca_code_length() {
        let code = CaCodeGenerator::generate(1);
        assert_eq!(code.len(), GPS_CA_CODE_LENGTH);
    }

    #[test]
    fn test_ca_code_values() {
        let code = CaCodeGenerator::generate(1);
        // All values should be either 1 or -1
        assert!(code.iter().all(|&v| v == 1 || v == -1));
    }

    #[test]
    fn test_different_prns_different_codes() {
        let code1 = CaCodeGenerator::generate(1);
        let code2 = CaCodeGenerator::generate(2);
        // Codes should be different
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_sampled_code() {
        let sampled = CaCodeGenerator::generate_sampled(1, 2_048_000.0, 1_023_000.0);
        assert!(sampled.len() > GPS_CA_CODE_LENGTH);
    }
}
