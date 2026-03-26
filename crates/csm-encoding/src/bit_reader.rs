use csm_core::{CsmError};

/// Multi-tier bit reader for efficient decoding
pub struct BitReader<'a> {
    bytes: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0-7, where 0 is MSB
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        BitReader {
            bytes,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Read n bits as value (MSB first)
    pub fn read_bits(&mut self, n: u8) -> Result<u32, CsmError> {
        assert!(n <= 32, "Cannot read more than 32 bits");
        let mut value = 0u32;
        
        for _ in 0..n {
            if self.byte_pos >= self.bytes.len() {
                return Err(CsmError::Other("Unexpected end of bit stream".to_string()));
            }
            let byte = self.bytes[self.byte_pos];
            let bit = (byte >> (7 - self.bit_pos)) & 1;
            value = (value << 1) | (bit as u32);
            
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Ok(value)
    }

    /// Read a value with tier-based decoding
    pub fn read_tiered(&mut self, tier: u8) -> Result<u32, CsmError> {
        match tier {
            0 => self.read_bits(8),
            1 => self.read_bits(14),
            2 => self.read_bits(18),
            3 => self.read_elias_gamma(),
            _ => Err(CsmError::Other(format!("Invalid tier: {}", tier))),
        }
    }

    /// Read using Elias-γ code
    fn read_elias_gamma(&mut self) -> Result<u32, CsmError> {
        // Count leading zeros
        let mut zeros = 0;
        while self.read_bits(1)? == 0 {
            zeros += 1;
            if zeros > 32 {
                return Err(CsmError::Other("Elias-γ overflow".to_string()));
            }
        }
        
        // Read the value (n bits, where n = zeros + 1)
        let value = if zeros == 0 {
            1
        } else {
            let val = self.read_bits(zeros)?;
            (1 << zeros) | val
        };
        
        Ok(value - 1) // Convert back from 1-indexed
    }

    /// Check if we're at end of stream
    pub fn is_empty(&self) -> bool {
        self.byte_pos >= self.bytes.len() && self.bit_pos == 0
    }

    /// Align to next byte boundary
    pub fn align(&mut self) {
        if self.bit_pos > 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }
}