/// Multi-tier bit writer for efficient encoding
/// Tiers: 0=8 bits, 1=14 bits, 2=18 bits, 3=255 (Elias-γ)
pub struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bit_pos: u8, // 0-7, where 0 is MSB
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            current_byte: 0,
            bit_pos: 0,
        }
    }

    /// Write n bits from value (MSB first)
    pub fn write_bits(&mut self, value: u32, n: u8) {
        assert!(n <= 32, "Cannot write more than 32 bits");
        for i in (0..n).rev() {
            let bit = (value >> i) & 1;
            self.current_byte |= (bit as u8) << (7 - self.bit_pos);
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bytes.push(self.current_byte);
                self.current_byte = 0;
                self.bit_pos = 0;
            }
        }
    }

    /// Write a value with tier-based encoding (8/14/18/255 bits)
    pub fn write_tiered(&mut self, value: u32, tier: u8) {
        match tier {
            0 => self.write_bits(value, 8),
            1 => self.write_bits(value, 14),
            2 => self.write_bits(value, 18),
            3 => self.write_elias_gamma(value),
            _ => panic!("Invalid tier: {}", tier),
        }
    }

    /// Write using Elias-γ code (for Tier 3)
    fn write_elias_gamma(&mut self, mut value: u32) {
        value += 1; // Elias-γ is 1-indexed
        
        // Find highest bit position
        let n = 32 - value.leading_zeros();
        
        // Write n-1 zeros
        for _ in 0..n - 1 {
            self.write_bits(0, 1);
        }
        
        // Write n (value in n bits, high bit is always 1)
        self.write_bits(value, n as u8);
    }

    /// Flush any pending bits and return byte vector
    pub fn finish(mut self) -> Vec<u8> {
        if self.bit_pos > 0 {
            self.bytes.push(self.current_byte);
        }
        self.bytes
    }

    /// Current size in bytes
    pub fn len(&self) -> usize {
        if self.bit_pos > 0 {
            self.bytes.len() + 1
        } else {
            self.bytes.len()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty() && self.bit_pos == 0
    }
}