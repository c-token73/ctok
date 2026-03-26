use csm_core::*;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

/// N-gram pattern di level PatternId sequence (bukan token sequence)
pub struct SequencePatternEngine {
    bigram_counts: FxHashMap<(PatternId, PatternId), u32>,
    unigram_counts: FxHashMap<PatternId, u32>,
    total_bigrams: u64,
    window: VecDeque<PatternId>, // recent pattern history (size=10)
}

impl SequencePatternEngine {
    pub fn new() -> Self {
        SequencePatternEngine {
            bigram_counts: FxHashMap::default(),
            unigram_counts: FxHashMap::default(),
            total_bigrams: 0,
            window: VecDeque::new(),
        }
    }

    pub fn update(&mut self, pid: PatternId) {
        if let Some(&prev) = self.window.back() {
            *self.bigram_counts.entry((prev, pid)).or_insert(0) += 1;
            self.total_bigrams += 1;
        }
        *self.unigram_counts.entry(pid).or_insert(0) += 1;
        self.window.push_back(pid);
        if self.window.len() > 10 {
            self.window.pop_front();
        }
    }

    pub fn bigram_pmi(&self, p1: PatternId, p2: PatternId) -> f64 {
        let n = self.total_bigrams as f64;
        let f12 = *self.bigram_counts.get(&(p1, p2)).unwrap_or(&0) as f64;
        let f1 = *self.unigram_counts.get(&p1).unwrap_or(&0) as f64;
        let f2 = *self.unigram_counts.get(&p2).unwrap_or(&0) as f64;
        if f12 < 1.0 || f1 < 1.0 || f2 < 1.0 {
            return -10.0;
        }
        (f12 * n / (f1 * f2)).log2()
    }

    /// Burstiness score: mendeteksi log yang datang dalam burst
    /// Nilai tinggi = bursty (tidak normal), nilai rendah = smooth
    pub fn burst_score(&self, _pid: PatternId, window_freqs: &[f64]) -> f64 {
        if window_freqs.is_empty() {
            return 0.0;
        }
        let mean = window_freqs.iter().sum::<f64>() / window_freqs.len() as f64;
        let var = window_freqs
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / window_freqs.len() as f64;
        if mean < 1e-9 {
            return 0.0;
        }
        let cv2 = var / (mean * mean);
        (cv2 - 1.0 / mean).max(0.0) // = (σ²/μ² - 1/μ), positive = bursty
    }
}