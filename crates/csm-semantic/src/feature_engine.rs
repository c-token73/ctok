use csm_core::*;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

/// Sliding window frequency counter per pattern_id
pub struct PatternFrequencyEngine {
    /// Window 1 menit: deque of (timestamp_micros, pattern_id)
    window_1m: VecDeque<(i64, PatternId)>,
    /// Window 1 jam
    window_1h: VecDeque<(i64, PatternId)>,
    /// Count per pattern dalam masing-masing window
    count_1m: FxHashMap<PatternId, u32>,
    count_1h: FxHashMap<PatternId, u32>,
    /// Historical mean + variance (EMA) per pattern
    ema_mean: FxHashMap<PatternId, f64>,
    ema_var: FxHashMap<PatternId, f64>,
    /// EMA decay factor (default 0.05)
    alpha: f64,
}

impl PatternFrequencyEngine {
    pub fn new(alpha: f64) -> Self {
        PatternFrequencyEngine {
            window_1m: VecDeque::new(),
            window_1h: VecDeque::new(),
            count_1m: FxHashMap::default(),
            count_1h: FxHashMap::default(),
            ema_mean: FxHashMap::default(),
            ema_var: FxHashMap::default(),
            alpha,
        }
    }

    pub fn update(&mut self, pattern_id: PatternId, ts: i64) {
        let cutoff_1m = ts - 60_000_000; // micros (1 minute)
        let cutoff_1h = ts - 3_600_000_000; // micros (1 hour)

        // Evict old entries from 1m window
        while self
            .window_1m
            .front()
            .map(|(t, _)| *t < cutoff_1m)
            .unwrap_or(false)
        {
            if let Some((_, pid)) = self.window_1m.pop_front() {
                if let Some(count) = self.count_1m.get_mut(&pid) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.count_1m.remove(&pid);
                    }
                }
            }
        }

        // Evict old entries from 1h window
        while self
            .window_1h
            .front()
            .map(|(t, _)| *t < cutoff_1h)
            .unwrap_or(false)
        {
            if let Some((_, pid)) = self.window_1h.pop_front() {
                if let Some(count) = self.count_1h.get_mut(&pid) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.count_1h.remove(&pid);
                    }
                }
            }
        }

        // Add new entries
        self.window_1m.push_back((ts, pattern_id));
        *self.count_1m.entry(pattern_id).or_insert(0) += 1;

        self.window_1h.push_back((ts, pattern_id));
        *self.count_1h.entry(pattern_id).or_insert(0) += 1;

        // Update EMA
        let freq = *self.count_1m.get(&pattern_id).unwrap_or(&0) as f64;
        let mean = self.ema_mean.entry(pattern_id).or_insert(freq);
        let var = self.ema_var.entry(pattern_id).or_insert(0.0);
        let delta = freq - *mean;
        *mean += self.alpha * delta;
        *var = (1.0 - self.alpha) * (*var + self.alpha * delta * delta);
    }

    pub fn freq_1m(&self, pid: PatternId) -> u32 {
        *self.count_1m.get(&pid).unwrap_or(&0)
    }

    pub fn freq_1h(&self, pid: PatternId) -> u32 {
        *self.count_1h.get(&pid).unwrap_or(&0)
    }

    pub fn z_score_1m(&self, pid: PatternId) -> f64 {
        let freq = self.freq_1m(pid) as f64;
        let mean = *self.ema_mean.get(&pid).unwrap_or(&freq);
        let std = self.ema_var.get(&pid).unwrap_or(&1.0).sqrt();
        if std < 1e-9 {
            0.0
        } else {
            (freq - mean) / std
        }
    }
}