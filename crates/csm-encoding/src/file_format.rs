use serde::{Deserialize, Serialize};
use csm_core::CsmError;

pub const MAGIC: [u8; 4] = *b"CSM4";
pub const MAJOR_VERSION: u8 = 4;
pub const MINOR_VERSION: u8 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmHeader {
    /// Magic bytes: b"CSM4"
    pub magic: [u8; 4],
    pub major_version: u8,
    pub minor_version: u8,
    
    /// Flags bitfield (u16, little-endian)
    pub flags: u16,
    
    /// Vocabulary size
    pub vocab_size: u32,
    /// Pattern count
    pub pattern_count: u32,
    /// Slot definitions count
    pub slot_count: u32,
    
    /// Domain: 0=Generic, 1=Log, 2=Code, 3=Text
    pub domain: u8,
    pub _reserved_1: [u8; 3],
    
    /// Tier cutoffs for vocabulary
    pub tier_cutoff_01: u32,
    pub tier_cutoff_12: u32,
    pub tier_cutoff_23: u32,
    pub _reserved_2: u32,
    
    /// Token count statistics
    pub token_count_encoded: u64,
    pub token_count_original: u64,
    
    /// Vocabulary fingerprint
    pub vocab_fingerprint: u64,
    
    /// Corpus entropy and compression ratio
    pub corpus_entropy: f32,
    pub compression_ratio: f32,
    
    /// Section offsets
    pub section_offset_vocab: u64,
    pub section_offset_pattern: u64,
    pub section_offset_slot: u64,
    pub section_offset_data: u64,
    pub section_offset_index: u64,
    
    /// Build timestamp (Unix micros)
    pub build_timestamp: u64,
    
    /// Header CRC32c (Castagnoli)
    pub header_crc32c: u32,
}

impl Default for CsmHeader {
    fn default() -> Self {
        CsmHeader {
            magic: MAGIC,
            major_version: MAJOR_VERSION,
            minor_version: MINOR_VERSION,
            flags: 0,
            vocab_size: 0,
            pattern_count: 0,
            slot_count: 0,
            domain: 0,
            _reserved_1: [0; 3],
            tier_cutoff_01: 0,
            tier_cutoff_12: 0,
            tier_cutoff_23: 0,
            _reserved_2: 0,
            token_count_encoded: 0,
            token_count_original: 0,
            vocab_fingerprint: 0,
            corpus_entropy: 0.0,
            compression_ratio: 1.0,
            section_offset_vocab: 0,
            section_offset_pattern: 0,
            section_offset_slot: 0,
            section_offset_data: 0,
            section_offset_index: 0,
            build_timestamp: 0,
            header_crc32c: 0,
        }
    }
}

impl CsmHeader {
    /// Size of the header in bytes (fixed 128 bytes)
    pub const SIZE: usize = 128;

    /// Read header from byte slice
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CsmError> {
        if bytes.len() < Self::SIZE {
            return Err(CsmError::Other("Header too short".to_string()));
        }

        let mut header = CsmHeader::default();
        
        // Read magic and versions
        header.magic.copy_from_slice(&bytes[0x00..0x04]);
        header.major_version = bytes[0x04];
        header.minor_version = bytes[0x05];
        header.flags = u16::from_le_bytes([bytes[0x06], bytes[0x07]]);
        
        // Read counts
        header.vocab_size = u32::from_le_bytes(bytes[0x08..0x0C].try_into().unwrap());
        header.pattern_count = u32::from_le_bytes(bytes[0x0C..0x10].try_into().unwrap());
        header.slot_count = u32::from_le_bytes(bytes[0x10..0x14].try_into().unwrap());
        
        // Domain and reserved
        header.domain = bytes[0x14];
        header._reserved_1.copy_from_slice(&bytes[0x15..0x18]);
        
        // Tier cutoffs
        header.tier_cutoff_01 = u32::from_le_bytes(bytes[0x18..0x1C].try_into().unwrap());
        header.tier_cutoff_12 = u32::from_le_bytes(bytes[0x1C..0x20].try_into().unwrap());
        header.tier_cutoff_23 = u32::from_le_bytes(bytes[0x20..0x24].try_into().unwrap());
        header._reserved_2 = u32::from_le_bytes(bytes[0x24..0x28].try_into().unwrap());
        
        // Token counts
        header.token_count_encoded = u64::from_le_bytes(bytes[0x28..0x30].try_into().unwrap());
        header.token_count_original = u64::from_le_bytes(bytes[0x30..0x38].try_into().unwrap());
        
        // Fingerprint and stats
        header.vocab_fingerprint = u64::from_le_bytes(bytes[0x38..0x40].try_into().unwrap());
        header.corpus_entropy = f32::from_le_bytes(bytes[0x40..0x44].try_into().unwrap());
        header.compression_ratio = f32::from_le_bytes(bytes[0x44..0x48].try_into().unwrap());
        
        // Section offsets
        header.section_offset_vocab = u64::from_le_bytes(bytes[0x48..0x50].try_into().unwrap());
        header.section_offset_pattern = u64::from_le_bytes(bytes[0x50..0x58].try_into().unwrap());
        header.section_offset_slot = u64::from_le_bytes(bytes[0x58..0x60].try_into().unwrap());
        header.section_offset_data = u64::from_le_bytes(bytes[0x60..0x68].try_into().unwrap());
        header.section_offset_index = u64::from_le_bytes(bytes[0x68..0x70].try_into().unwrap());
        
        // Timestamp and CRC
        header.build_timestamp = u64::from_le_bytes(bytes[0x70..0x78].try_into().unwrap());
        header.header_crc32c = u32::from_le_bytes(bytes[0x78..0x7C].try_into().unwrap());
        
        // Validate magic
        if header.magic != MAGIC {
            return Err(CsmError::Other("Invalid magic bytes".to_string()));
        }
        
        Ok(header)
    }

    /// Write header to byte buffer
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        // Write magic and versions
        bytes[0x00..0x04].copy_from_slice(&self.magic);
        bytes[0x04] = self.major_version;
        bytes[0x05] = self.minor_version;
        bytes[0x06..0x08].copy_from_slice(&self.flags.to_le_bytes());
        
        // Write counts
        bytes[0x08..0x0C].copy_from_slice(&self.vocab_size.to_le_bytes());
        bytes[0x0C..0x10].copy_from_slice(&self.pattern_count.to_le_bytes());
        bytes[0x10..0x14].copy_from_slice(&self.slot_count.to_le_bytes());
        
        // Domain and reserved
        bytes[0x14] = self.domain;
        bytes[0x15..0x18].copy_from_slice(&self._reserved_1);
        
        // Tier cutoffs
        bytes[0x18..0x1C].copy_from_slice(&self.tier_cutoff_01.to_le_bytes());
        bytes[0x1C..0x20].copy_from_slice(&self.tier_cutoff_12.to_le_bytes());
        bytes[0x20..0x24].copy_from_slice(&self.tier_cutoff_23.to_le_bytes());
        bytes[0x24..0x28].copy_from_slice(&self._reserved_2.to_le_bytes());
        
        // Token counts
        bytes[0x28..0x30].copy_from_slice(&self.token_count_encoded.to_le_bytes());
        bytes[0x30..0x38].copy_from_slice(&self.token_count_original.to_le_bytes());
        
        // Fingerprint and stats
        bytes[0x38..0x40].copy_from_slice(&self.vocab_fingerprint.to_le_bytes());
        bytes[0x40..0x44].copy_from_slice(&self.corpus_entropy.to_le_bytes());
        bytes[0x44..0x48].copy_from_slice(&self.compression_ratio.to_le_bytes());
        
        // Section offsets
        bytes[0x48..0x50].copy_from_slice(&self.section_offset_vocab.to_le_bytes());
        bytes[0x50..0x58].copy_from_slice(&self.section_offset_pattern.to_le_bytes());
        bytes[0x58..0x60].copy_from_slice(&self.section_offset_slot.to_le_bytes());
        bytes[0x60..0x68].copy_from_slice(&self.section_offset_data.to_le_bytes());
        bytes[0x68..0x70].copy_from_slice(&self.section_offset_index.to_le_bytes());
        
        // Timestamp and CRC
        bytes[0x70..0x78].copy_from_slice(&self.build_timestamp.to_le_bytes());
        bytes[0x78..0x7C].copy_from_slice(&self.header_crc32c.to_le_bytes());
        
        bytes
    }
}

// Flags bitfield (u16)
pub mod flags {
    pub const MULTI_TIER_PACK: u16 = 1 << 0;
    pub const VITERBI_SELECT: u16 = 1 << 1;
    pub const DELTA_SLOTS: u16 = 1 << 2;
    pub const ANS_TIER3: u16 = 1 << 3;
    pub const SLOT_TYPED: u16 = 1 << 4;
    pub const HAS_FEATURES: u16 = 1 << 5;
    pub const STREAMING_MODE: u16 = 1 << 6;
    pub const HAS_INDEX: u16 = 1 << 7;
    pub const HELD_OUT_VERIFIED: u16 = 1 << 8;
}