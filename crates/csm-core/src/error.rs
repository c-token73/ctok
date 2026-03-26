use crate::*;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CsmError {
    #[error("Vocab is frozen; cannot insert '{0}'")]
    VocabFrozen(String),
    #[error("Vocab drift detected: expected fingerprint {expected:#016x}, got {found:#016x}")]
    VocabDrift { expected: u64, found: u64 },
    #[error("VocabId {0} out of range (vocab size: {1})")]
    VocabIdOob(u32, usize),
    #[error("Invalid UTF-8 at byte offset {offset}: {source}")]
    InvalidUtf8 { offset: usize, source: std::str::Utf8Error },
    #[error("Input too large: {size} bytes (max: {max})")]
    InputTooLarge { size: usize, max: usize },
    #[error("CRC32c mismatch: expected {expected:#010x}, computed {computed:#010x}")]
    CrcMismatch { expected: u32, computed: u32 },
    #[error("Invalid magic bytes: expected 'CSM4', found {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("Unsupported version: {major}.{minor}")]
    UnsupportedVersion { major: u8, minor: u8 },
    #[error("Section {section} corrupt or truncated")]
    SectionCorrupt { section: &'static str },
    #[error("PatternId {0} not found in registry")]
    PatternNotFound(PatternId),
    #[error("Pattern compress_gain {0} ≤ 0; pattern should be rejected at build")]
    NegativeCompressGain(f32),
    #[error("Slot extraction failed for type {slot_type:?}: {reason}")]
    SlotExtractionFailed { slot_type: SlotType, reason: String },
    #[error("Slot entropy {entropy:.3} outside bounds [{min:.1}, {max:.1}]")]
    SlotEntropyOob { entropy: f32, min: f32, max: f32 },
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Arrow error: {0}")]
    Arrow(String),
    #[error("Kafka error: {0}")]
    Kafka(String),
    #[error("Channel closed unexpectedly")]
    ChannelClosed,
    #[error("{0}")]
    Other(String),
}