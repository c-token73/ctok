use smallvec::SmallVec;
use serde::{Deserialize, Serialize};

pub mod vocab;
pub mod pattern;
pub mod slot;
pub mod fst_engine;
pub mod viterbi;
pub mod fallback;
pub mod error;

pub use vocab::*;
pub use pattern::*;
pub use slot::*;
pub use error::*;
pub use viterbi::*;
pub use smallvec;

/// Trait utama untuk semua tokenizer domain
pub trait DomainTokenizer: Send + Sync {
    type Config: Default + Clone + Send;

    fn new(config: Self::Config) -> Self;
    fn tokenize<'a>(&self, input: &'a str) -> Vec<Token<'a>>;
    fn domain(&self) -> DomainKind;
}

/// Trait untuk semua encoder (sync atau streaming)
pub trait CsmEncoder {
    type Error: std::error::Error + Send + Sync + 'static;

    fn encode(&mut self, input: &str) -> Result<CsmData, Self::Error>;
    fn encode_batch(&mut self, inputs: &[&str]) -> Result<Vec<CsmData>, Self::Error>;
    fn domain(&self) -> DomainKind;
}

/// Trait untuk output format (Arrow, Parquet, JSON, binary)
pub trait OutputFormat: Send + Sync {
    fn write_record(&mut self, record: &SemanticRecord) -> Result<(), CsmError>;
    fn write_batch(&mut self, records: &[SemanticRecord]) -> Result<(), CsmError>;
    fn flush(&mut self) -> Result<Vec<u8>, CsmError>;
    fn format_name(&self) -> &'static str;
}

/// Trait untuk sumber data streaming
pub trait DataSource: Send {
    type Item: AsRef<[u8]>;
    fn next_chunk(&mut self) -> Option<Result<Self::Item, CsmError>>;
    fn source_id(&self) -> &str;
}

/// Trait untuk feature extractors
pub trait FeatureExtractor: Send + Sync {
    fn extract(&self, records: &[SemanticRecord]) -> FeatureVector;
    fn feature_names(&self) -> Vec<String>;
    fn feature_count(&self) -> usize;
}

// Placeholder structs, to be implemented
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token<'a> {
    pub id: VocabId,
    pub raw: &'a str,
    pub kind: TokenKind,
}

/// Tipe semantik dari slot — menentukan cara ekstraksi dan feature generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenKind {
    Literal,
    Slot(SlotType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmData {
    pub header: CsmLineHeader,
    pub bits: Vec<u8>,
    pub slots: Vec<SlotValue>,
    pub features: Option<FeatureVector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmLineHeader {
    pub source_offset: u64,
    pub log_timestamp: Option<i64>,
    pub token_count_orig: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRecord {
    pub source_id: String,
    pub offset: u64,
    pub line_number: Option<u64>,
    pub ingested_at: i64,
    pub log_timestamp: Option<i64>,
    pub pattern_id: PatternId,
    pub template: String,
    pub domain: DomainKind,
    pub slots: Vec<NamedSlot>,
    pub raw_token_count: u16,
    pub encoded_bits: u32,
    pub compress_ratio: f32,
    pub features: Option<FeatureVector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedSlot {
    pub name: String,
    pub slot_type: SlotType,
    pub value: SlotValue,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub pattern_id: u32,
    pub pattern_freq_1m: f32,
    pub pattern_freq_1h: f32,
    pub pattern_freq_rank: f32,
    pub is_rare_pattern: bool,
    pub pattern_novelty: f32,
    pub slot_stats: Vec<SlotStats>,
    pub seq_bigram_score: f32,
    pub seq_entropy_local: f32,
    pub seq_burst_score: f32,
    pub anomaly_score: f32,
    pub anomaly_components: AnomalyComponents,
    pub window_id: u64,
    pub computed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotStats {
    pub slot_name: String,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub z_score: f64,
    pub percentile: f32,
    pub is_outlier: bool,
    pub cardinality: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyComponents {
    pub freq_anomaly: f32,
    pub slot_anomaly: f32,
    pub seq_anomaly: f32,
    pub temporal_anomaly: f32,
}