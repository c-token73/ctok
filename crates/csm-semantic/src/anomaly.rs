use csm_core::*;
use rustc_hash::FxHashMap;

/// Per-slot running statistics using Welford's online algorithm
pub struct SlotStatEngine {
    /// stats per (pattern_id, slot_position)
    stats: FxHashMap<(PatternId, u8), OnlineStats>,
}

/// Welford's online mean + variance (numerically stable)
#[derive(Debug, Clone)]
pub struct OnlineStats {
    pub count: u64,
    pub mean: f64,
    pub m2: f64, // running sum of squared deviations
    pub min: f64,
    pub max: f64,
}

impl Default for OnlineStats {
    fn default() -> Self {
        OnlineStats {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }
}

impl OnlineStats {
    pub fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
        if x < self.min {
            self.min = x;
        }
        if x > self.max {
            self.max = x;
        }
    }

    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    pub fn std(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn z_score(&self, x: f64) -> f64 {
        let s = self.std();
        if s < 1e-12 {
            0.0
        } else {
            (x - self.mean) / s
        }
    }
}

impl SlotStatEngine {
    pub fn new() -> Self {
        SlotStatEngine {
            stats: FxHashMap::default(),
        }
    }

    pub fn update(&mut self, pattern_id: PatternId, slot_pos: u8, value: f64) {
        self.stats
            .entry((pattern_id, slot_pos))
            .or_insert_with(OnlineStats::default)
            .update(value);
    }

    pub fn get_stats(&self, pattern_id: PatternId, slot_pos: u8) -> Option<&OnlineStats> {
        self.stats.get(&(pattern_id, slot_pos))
    }
}