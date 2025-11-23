use super::SbasMessage;
use crate::{Error, Result};

/// SBAS Message Decoder
pub struct SbasDecoder {
    prn: u8,
    bit_buffer: Vec<u8>,
}

impl SbasDecoder {
    pub fn new(prn: u8) -> Self {
        Self {
            prn,
            bit_buffer: Vec::new(),
        }
    }

    /// Add a bit to the decoder
    pub fn add_bit(&mut self, bit: u8) {
        self.bit_buffer.push(bit);

        // SBAS messages are 250 bits long
        if self.bit_buffer.len() >= 250 {
            if let Ok(message) = self.try_decode_message() {
                tracing::debug!("SBAS PRN {} decoded message type {}", self.prn, message.message_type);
            }

            // Remove processed bits
            if self.bit_buffer.len() > 500 {
                self.bit_buffer.drain(0..250);
            }
        }
    }

    fn try_decode_message(&self) -> Result<SbasMessage> {
        // SBAS message structure:
        // - 8 bits preamble (0x53)
        // - 6 bits message type
        // - 212 bits data
        // - 24 bits CRC

        for offset in 0..self.bit_buffer.len().saturating_sub(250) {
            if self.is_preamble_at_offset(offset) {
                return self.decode_message_at_offset(offset);
            }
        }

        Err(Error::SbasDecode("No preamble found".to_string()))
    }

    fn is_preamble_at_offset(&self, offset: usize) -> bool {
        if offset + 8 > self.bit_buffer.len() {
            return false;
        }

        // SBAS preamble: 01010011 (0x53)
        let preamble = [0, 1, 0, 1, 0, 0, 1, 1];
        for i in 0..8 {
            if self.bit_buffer[offset + i] != preamble[i] {
                return false;
            }
        }

        true
    }

    fn decode_message_at_offset(&self, offset: usize) -> Result<SbasMessage> {
        if offset + 250 > self.bit_buffer.len() {
            return Err(Error::SbasDecode("Not enough bits".to_string()));
        }

        // Extract message type (6 bits after preamble)
        let mut message_type: u8 = 0;
        for i in 0..6 {
            message_type = (message_type << 1) | self.bit_buffer[offset + 8 + i];
        }

        // Extract data (212 bits)
        let mut data = Vec::new();
        for i in 0..(212 / 8) {
            let mut byte: u8 = 0;
            for j in 0..8 {
                let bit_idx = offset + 14 + i * 8 + j;
                if bit_idx < self.bit_buffer.len() {
                    byte = (byte << 1) | self.bit_buffer[bit_idx];
                }
            }
            data.push(byte);
        }

        // TODO: Verify CRC-24

        Ok(SbasMessage {
            prn: self.prn,
            message_type,
            data,
            time: 0.0,
        })
    }
}

/// Decode SBAS message type 1 (PRN masks)
pub fn decode_type1(message: &SbasMessage) -> Vec<u8> {
    // Extract PRN mask from message data
    // Simplified implementation
    Vec::new()
}

/// Decode SBAS message types 2-5 (Fast corrections)
pub fn decode_fast_corrections(message: &SbasMessage) -> Vec<f64> {
    // Extract fast corrections
    // Simplified implementation
    Vec::new()
}

/// Decode SBAS message types 24-25 (Long-term corrections)
pub fn decode_long_term_corrections(message: &SbasMessage) -> Vec<f64> {
    // Extract long-term satellite error corrections
    // Simplified implementation
    Vec::new()
}

/// Decode SBAS message type 26 (Ionospheric corrections)
pub fn decode_iono_corrections(message: &SbasMessage) -> Vec<f64> {
    // Extract ionospheric grid corrections
    // Simplified implementation
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbas_decoder_creation() {
        let decoder = SbasDecoder::new(131);
        assert_eq!(decoder.prn, 131);
    }

    #[test]
    fn test_preamble_detection() {
        let mut decoder = SbasDecoder::new(131);
        let preamble = vec![0, 1, 0, 1, 0, 0, 1, 1];

        for &bit in &preamble {
            decoder.add_bit(bit);
        }

        assert!(decoder.is_preamble_at_offset(0));
    }
}
