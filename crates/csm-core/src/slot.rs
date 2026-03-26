use crate::*;
use serde::{Deserialize, Serialize};

/// Tipe semantik dari slot — menentukan cara ekstraksi dan feature generation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlotType {
    // Numeric
    Integer,
    Float,
    // Network
    IpAddress,
    Port,
    // Time
    Timestamp,
    Duration,
    // System
    LogLevel,
    ExitCode,
    Pid,
    // Identifier
    Uuid,
    Hash,
    Path,
    // Code-specific
    Identifier,
    StringLiteral,
    // Generic
    Generic,
}

/// Nilai aktual dari slot setelah ekstraksi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlotValue {
    Int(i64),
    Float(f64),
    IpV4([u8; 4]),
    IpV6([u8; 16]),
    Timestamp(i64),
    Duration(u64),
    LogLevel(LogLevelKind),
    Uuid([u8; 16]),
    Hash(Vec<u8>),
    Text(u32), // interned string ID
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevelKind {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

/// Schema slot untuk satu pattern (immutable setelah pattern di-freeze)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotSchema {
    pub slots: SmallVec<[(u8, SlotType); 4]>, // (position_in_template, type)
}