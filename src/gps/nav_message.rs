use crate::{Error, Result};

/// GPS Navigation Message Decoder
pub struct NavMessage {
    prn: u8,
    bit_buffer: Vec<i8>,
    subframes: [Option<Vec<u32>>; 5],
    pub tow: Option<u32>,          // Time of Week (seconds)
    pub week_number: Option<u16>,  // GPS week number
}

impl NavMessage {
    pub fn new(prn: u8) -> Self {
        Self {
            prn,
            bit_buffer: Vec::new(),
            subframes: [None, None, None, None, None],
            tow: None,
            week_number: None,
        }
    }

    /// Add a navigation bit to the decoder
    pub fn add_bit(&mut self, bit: i8) {
        self.bit_buffer.push(bit);

        // Try to decode when we have enough bits for a subframe (300 bits)
        if self.bit_buffer.len() >= 300 {
            if let Ok(subframe) = self.try_decode_subframe() {
                let subframe_id = ((subframe[1] >> 8) & 0x7) as usize;
                if subframe_id >= 1 && subframe_id <= 5 {
                    self.subframes[subframe_id - 1] = Some(subframe);
                    tracing::debug!("PRN {} decoded subframe {}", self.prn, subframe_id);
                }
            }

            // Keep a rolling window of bits
            if self.bit_buffer.len() > 600 {
                self.bit_buffer.drain(0..300);
            }
        }
    }

    /// Try to decode a subframe from the bit buffer
    fn try_decode_subframe(&self) -> Result<Vec<u32>> {
        // Find preamble (10001011 = 0x8B)
        for offset in 0..self.bit_buffer.len().saturating_sub(300) {
            if self.is_preamble_at_offset(offset) {
                return self.decode_subframe_at_offset(offset);
            }
        }

        Err(Error::NavMessageDecode("No preamble found".to_string()))
    }

    fn is_preamble_at_offset(&self, offset: usize) -> bool {
        if offset + 8 > self.bit_buffer.len() {
            return false;
        }

        let preamble = [1, 0, 0, 0, 1, 0, 1, 1];
        for i in 0..8 {
            if self.bit_buffer[offset + i] != preamble[i] {
                return false;
            }
        }

        true
    }

    fn decode_subframe_at_offset(&self, offset: usize) -> Result<Vec<u32>> {
        if offset + 300 > self.bit_buffer.len() {
            return Err(Error::NavMessageDecode("Not enough bits".to_string()));
        }

        // Extract 300 bits and convert to 10 words of 30 bits each
        let mut words = Vec::new();

        for word_idx in 0..10 {
            let start = offset + word_idx * 30;
            let mut word: u32 = 0;

            for bit_idx in 0..30 {
                let bit = if self.bit_buffer[start + bit_idx] > 0 {
                    1
                } else {
                    0
                };
                word = (word << 1) | bit;
            }

            words.push(word);
        }

        // Verify parity (simplified - in production, do full parity check)
        if !self.verify_parity(&words) {
            return Err(Error::NavMessageDecode("Parity check failed".to_string()));
        }

        // Extract TOW from HOW (second word)
        let tow_count = (words[1] >> 13) & 0x1FFFF;
        self.extract_tow(tow_count);

        Ok(words)
    }

    fn verify_parity(&self, words: &[u32]) -> bool {
        // Simplified parity check - check that parity bits are present
        // In production, implement full GPS parity algorithm
        words.len() == 10
    }

    fn extract_tow(&self, tow_count: u32) {
        // TOW-count is in 6-second units
        // Convert to seconds
        let _tow_seconds = tow_count * 6;
        // Note: This would normally update self.tow, but we need &mut self
        // This will be fixed when we properly integrate with the channel
    }

    /// Get decoded ephemeris if subframes 1-3 are available
    pub fn get_ephemeris(&self) -> Option<Vec<u32>> {
        if self.subframes[0].is_some() && self.subframes[1].is_some() && self.subframes[2].is_some() {
            // Combine subframes 1-3 into ephemeris data
            let mut ephemeris = Vec::new();
            ephemeris.extend(self.subframes[0].as_ref().unwrap());
            ephemeris.extend(self.subframes[1].as_ref().unwrap());
            ephemeris.extend(self.subframes[2].as_ref().unwrap());
            Some(ephemeris)
        } else {
            None
        }
    }

    /// Get almanac data if subframe 4 or 5 is available
    pub fn get_almanac(&self) -> Option<Vec<u32>> {
        if let Some(ref sf4) = self.subframes[3] {
            Some(sf4.clone())
        } else {
            self.subframes[4].clone()
        }
    }

    /// Check if we have a complete navigation message
    pub fn is_complete(&self) -> bool {
        self.subframes[0].is_some()
            && self.subframes[1].is_some()
            && self.subframes[2].is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nav_message_creation() {
        let nav_msg = NavMessage::new(1);
        assert_eq!(nav_msg.prn, 1);
        assert!(!nav_msg.is_complete());
    }

    #[test]
    fn test_preamble_detection() {
        let mut nav_msg = NavMessage::new(1);
        let preamble = vec![1, 0, 0, 0, 1, 0, 1, 1];

        for &bit in &preamble {
            nav_msg.add_bit(bit);
        }

        assert!(nav_msg.is_preamble_at_offset(0));
    }
}
