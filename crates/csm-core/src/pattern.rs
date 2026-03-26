use crate::*;
use smallvec::SmallVec;
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;

pub type PatternId = u32;
pub const INVALID_PATTERN: PatternId = u32::MAX;

/// Pattern struct — cache-line aligned untuk SIMD-friendly bulk scan
#[repr(C, align(64))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    // Identity
    pub id: PatternId,
    pub domain: DomainKind,
    /// Base token sequence (max 5 tokens, stack-allocated)
    pub base_seq: SmallVec<[VocabId; 5]>,
    /// Slot schema: posisi dan tipe slot
    pub slot_schema: SlotSchema,
    /// Human-readable template: "Connection from {IP} port {PORT} rejected"
    pub template: String,

    // Discovery metrics (immutable setelah build)
    pub freq: u32,
    pub ppmi_score: f32,
    pub compress_gain: f32,
    pub pgs_score: f32,
    pub stability: f32,
    pub final_score: f32,

    // Status
    pub deprecated: bool,
    pub _pad: [u8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainKind {
    Log,
    Code,
    Text,
    Generic,
}

/// Thread-safe pattern registry
pub struct PatternRegistry {
    patterns: Vec<Pattern>,
    by_domain: FxHashMap<DomainKind, Vec<PatternId>>,
    template_index: FxHashMap<String, PatternId>,
    pub frozen: bool,
}

impl PatternRegistry {
    pub fn new() -> Self {
        PatternRegistry {
            patterns: Vec::new(),
            by_domain: FxHashMap::default(),
            template_index: FxHashMap::default(),
            frozen: false,
        }
    }

    pub fn register(&mut self, p: Pattern) -> Result<PatternId, CsmError> {
        if self.frozen {
            return Err(CsmError::Other("Registry frozen".to_string()));
        }
        let id = self.patterns.len() as PatternId;
        let domain = p.domain;
        self.patterns.push(p);
        self.by_domain.entry(domain).or_insert(Vec::new()).push(id);
        Ok(id)
    }

    pub fn get(&self, id: PatternId) -> Option<&Pattern> {
        self.patterns.get(id as usize)
    }

    pub fn by_template(&self, template: &str) -> Option<PatternId> {
        self.template_index.get(template).copied()
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn len(&self) -> usize {
        self.patterns.len()
    }
}