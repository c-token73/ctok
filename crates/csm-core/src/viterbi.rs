use crate::*;

#[derive(Debug, Clone)]
pub struct MatchCandidate {
    pub start: usize,
    pub end: usize,
    pub pattern_id: PatternId,
    pub compress_gain: f32,
}

#[derive(Debug, Clone)]
pub enum AssignmentKind {
    Pattern { id: PatternId },
    Token { id: VocabId },
    Fallback { level: u8 },
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub kind: AssignmentKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone)]
struct DpEntry {
    gain: i64,
    prev: usize,
    kind: AssignmentKind,
}

/// Viterbi DP — O(N × max_pattern_len) time, O(N) space
pub fn viterbi_select(
    tokens: &[VocabId],
    matches: &[MatchCandidate],
    vocab: &Vocab,
) -> Vec<Assignment> {
    let n = tokens.len();
    let neg_inf = i64::MIN / 2;

    // Pre-index: end_position → candidates ending there
    let mut by_end: Vec<Vec<&MatchCandidate>> = vec![vec![]; n + 1];
    for m in matches {
        by_end[m.end].push(m);
    }

    let mut dp: Vec<DpEntry> = (0..=n)
        .map(|_| DpEntry {
            gain: neg_inf,
            prev: 0,
            kind: AssignmentKind::Token { id: 0 },
        })
        .collect();
    dp[0].gain = 0;

    for i in 1..=n {
        // Option A: single token at i-1
        if dp[i - 1].gain != neg_inf {
            let cost = vocab.tier_bits(tokens[i - 1]) as i64;
            let g = dp[i - 1].gain - cost;
            if g > dp[i].gain {
                dp[i] = DpEntry {
                    gain: g,
                    prev: i - 1,
                    kind: AssignmentKind::Token {
                        id: tokens[i - 1],
                    },
                };
            }
        }
        // Option B: patterns ending at i
        for m in &by_end[i] {
            if dp[m.start].gain != neg_inf {
                let g = dp[m.start].gain + m.compress_gain as i64;
                if g > dp[i].gain {
                    dp[i] = DpEntry {
                        gain: g,
                        prev: m.start,
                        kind: AssignmentKind::Pattern {
                            id: m.pattern_id,
                        },
                    };
                }
            }
        }
    }

    // Backtrack
    let mut assignments = Vec::new();
    let mut pos = n;
    while pos > 0 {
        let entry = &dp[pos];
        assignments.push(Assignment {
            kind: entry.kind.clone(),
            start: entry.prev,
            end: pos,
        });
        pos = entry.prev;
    }
    assignments.reverse();
    assignments
}