use crate::*;

/// Hierarchical fallback chain for tokens that don't match patterns
/// Priority: Pattern → Token(tier0) → Token(tier1) → Token(tier2) → Byte → UNK
pub struct FallbackEngine;

impl FallbackEngine {
    pub fn new() -> Self {
        FallbackEngine
    }

    /// Generate fallback assignments for unmatched positions
    pub fn fallback(&self, tokens: &[VocabId], _vocab: &Vocab) -> Vec<Assignment> {
        let mut assignments = Vec::new();

        for (i, &token_id) in tokens.iter().enumerate() {
            assignments.push(Assignment {
                kind: AssignmentKind::Token { id: token_id },
                start: i,
                end: i + 1,
            });
        }

        assignments
    }

    /// Compute bit cost for fallback assignment
    /// Tier 0: 8 bits
    /// Tier 1: 14 bits
    /// Tier 2: 18 bits
    /// Tier 3: Elias-γ (average ~10 bits for common tokens)
    pub fn fallback_cost(vocab_tier: u8) -> i64 {
        match vocab_tier {
            0 => 8,
            1 => 14,
            2 => 18,
            3 => 10, // approximate for Elias-γ
            _ => 255,
        }
    }
}