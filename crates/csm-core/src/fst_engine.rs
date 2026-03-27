use crate::*;
use rustc_hash::FxHashMap;

/// FST-based pattern matching engine
/// Implements pattern lookup and matching candidate generation
pub struct FstEngine {
    /// Global pattern registry (shared across matches)
    patterns: FxHashMap<Vec<VocabId>, PatternId>,
}

impl FstEngine {
    pub fn new() -> Self {
        FstEngine {
            patterns: FxHashMap::default(),
        }
    }

    pub fn from_registry(registry: &PatternRegistry) -> Self {
        let mut engine = FstEngine::new();
        for pattern_id in 0..registry.len() as PatternId {
            if let Some(pattern) = registry.get(pattern_id) {
                    if !pattern.deprecated {
                        engine.patterns.insert(pattern.base_seq.to_vec(), pattern_id);
                    }
                }
        }
        engine
    }

    /// Register a pattern with its token sequence
    pub fn add_pattern(&mut self, seq: Vec<VocabId>, pattern_id: PatternId) {
        self.patterns.insert(seq, pattern_id);
    }

    /// Find all matching patterns in token sequence
    /// Returns MatchCandidates with [start, end, pattern_id, compress_gain]
    pub fn query(&self, tokens: &[VocabId], registry: &PatternRegistry) -> Vec<MatchCandidate> {
        let mut candidates = Vec::new();

        // Greedy longest-match strategy
        // For each starting position, try to find longest matching pattern
        for start in 0..tokens.len() {
            // Try matching patterns of increasing length
            // Max pattern length is typically 5 (see spec: §2.2)
            for len in 1..=std::cmp::min(5, tokens.len() - start) {
                let subseq = &tokens[start..start + len];
                
                if let Some(&pattern_id) = self.patterns.get(subseq) {
                    let end = start + len;

                    let compress_gain = registry
                        .get(pattern_id)
                        .map(|p| p.compress_gain)
                        .unwrap_or(((len as f32) * 2.0));

                    candidates.push(MatchCandidate {
                        start,
                        end,
                        pattern_id,
                        compress_gain,
                    });
                }
            }
        }

        // Remove overlapping candidates, keeping longest matches
        candidates.sort_by(|a, b| {
            (b.end - b.start)
                .cmp(&(a.end - a.start))
                .then_with(|| a.start.cmp(&b.start))
        });

        let mut result = Vec::new();
        let mut covered = vec![false; tokens.len()];

        for candidate in candidates {
            // Check if this match overlaps with already-selected matches
            if !covered[candidate.start..candidate.end]
                .iter()
                .any(|&c| c)
            {
                for i in candidate.start..candidate.end {
                    covered[i] = true;
                }
                result.push(candidate);
            }
        }

        result.sort_by_key(|c| c.start);
        result
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn clear(&mut self) {
        self.patterns.clear();
    }
}