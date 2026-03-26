use csm_core::*;

/// Compute anomaly score from components
/// Formula: anomaly_score = w1×freq_anomaly + w2×slot_anomaly + w3×seq_anomaly + w4×temporal_anomaly
/// Weights: w1=0.35, w2=0.30, w3=0.20, w4=0.15
pub fn compute_anomaly_score(components: &AnomalyComponents) -> f32 {
    const W1: f32 = 0.35;
    const W2: f32 = 0.30;
    const W3: f32 = 0.20;
    const W4: f32 = 0.15;

    let score =
        W1 * components.freq_anomaly
            + W2 * components.slot_anomaly
            + W3 * components.seq_anomaly
            + W4 * components.temporal_anomaly;

    score.min(1.0).max(0.0) // clamp to [0, 1]
}

/// Sigmoid function for anomaly scoring
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Compute frequency anomaly from z-score
pub fn freq_anomaly_from_zscore(z: f32) -> f32 {
    sigmoid((z.abs() - 2.0))
}

/// Placeholder for Arrow RecordBatch conversion
pub fn records_to_arrow(_records: &[SemanticRecord]) -> Result<(), CsmError> {
    // TODO: Implement Arrow RecordBatch conversion using arrow2 crate
    Ok(())
}