use crate::*;
use rustc_hash::FxHashMap;

pub type VocabId = u32;
pub const VOCAB_ID_UNK: VocabId = 0;
pub const VOCAB_ID_UNK_BYTE_BASE: VocabId = 1;

/// Vocabulary dengan String storage (simplified, owns all strings)
pub struct Vocab {
    /// Primary lookup: string slice → ID
    str_to_id: FxHashMap<String, VocabId>,
    /// Reverse lookup: ID → string
    id_to_str: Vec<String>,
    /// Frequency dari training corpus (untuk tier assignment)
    id_to_freq: Vec<u32>,
    /// Tier assignment cache (0/1/2/3 — precomputed setelah freeze)
    id_to_tier: Vec<u8>,
    /// Zipf-sorted IDs (freq DESC) — untuk tier boundary computation
    freq_sorted: Vec<VocabId>,
    /// Tier boundaries [cutoff_0_1, cutoff_1_2, cutoff_2_3]
    tier_cutoffs: [u32; 3],
    /// Fingerprint: FNV-1a hash dari sorted(id→str) — untuk drift detection
    pub fingerprint: u64,
    /// Frozen = true → no more insertions allowed
    pub frozen: bool,
}

impl Vocab {
    pub fn new() -> Self {
        let mut str_to_id = FxHashMap::default();
        let mut id_to_str = Vec::new();
        let mut id_to_freq = Vec::new();
        let mut id_to_tier = Vec::new();
        
        let unk = "UNK".to_string();
        id_to_str.push(unk.clone());
        id_to_freq.push(0);
        id_to_tier.push(3);
        str_to_id.insert(unk, VOCAB_ID_UNK);
        
        Vocab {
            str_to_id,
            id_to_str,
            id_to_freq,
            id_to_tier,
            freq_sorted: vec![VOCAB_ID_UNK],
            tier_cutoffs: [0; 3],
            fingerprint: 0,
            frozen: false,
        }
    }

    pub fn insert(&mut self, s: &str) -> Result<VocabId, CsmError> {
        if self.frozen {
            return Err(CsmError::VocabFrozen(s.to_string()));
        }
        if let Some(&id) = self.str_to_id.get(s) {
            return Ok(id);
        }
        let id = self.id_to_str.len() as VocabId;
        let s_owned = s.to_string();
        self.str_to_id.insert(s_owned.clone(), id);
        self.id_to_str.push(s_owned);
        self.id_to_freq.push(0);
        self.id_to_tier.push(0);
        Ok(id)
    }

    pub fn freeze(&mut self) {
        // Compute frequencies (placeholder, assume set externally)
        // Sort by freq DESC
        self.freq_sorted = (0..self.id_to_str.len() as VocabId).collect();
        // Placeholder tier cutoffs
        self.tier_cutoffs = [100, 1000, 10000];
        // Compute fingerprint
        self.fingerprint = 0; // TODO: FNV-1a
        self.frozen = true;
    }

    pub fn id(&self, s: &str) -> Option<VocabId> {
        self.str_to_id.get(s).copied()
    }

    pub fn str(&self, id: VocabId) -> Option<&str> {
        self.id_to_str.get(id as usize).map(|s| s.as_str())
    }

    pub fn tier(&self, id: VocabId) -> u8 {
        self.id_to_tier.get(id as usize).copied().unwrap_or(3)
    }

    pub fn tier_bits(&self, id: VocabId) -> u8 {
        match self.tier(id) {
            0 => 8,
            1 => 14,
            2 => 18,
            _ => 255, // Elias-γ
        }
    }

    pub fn len(&self) -> usize {
        self.id_to_str.len()
    }

    pub fn size(&self) -> usize {
        self.id_to_str.len()
    }
}