//! # Compressed Convolutional Memory Retrieval
//!
//! This crate implements a **deterministic**, **non-authorizing** memory retrieval
//! mechanism inspired by Compressed Convolutional Attention.
//!
//! ## Pipeline
//!
//! ```text
//! Ordered memory events (embedding vectors)
//!   → Deterministic projection (embedding_dim → latent_dim)
//!   → Local temporal convolution (per-event neighborhood smoothing)
//!   → Cosine scoring against query
//!   → Top-k retrieval with readback-friendly explanation
//! ```
//!
//! ## Design principles
//!
//! - **No LLM calls** — pure deterministic math
//! - **No GPU dependency** — standard f64 arithmetic
//! - **No persistent mutation** — pure functions, no side effects
//! - **No authorization semantics** — retrieval is advisory only
//! - **Deterministic** — same inputs always produce same outputs
//!
//! ## Safety invariants
//!
//! All public functions are pure (no I/O, no mutation of inputs).
//! The crate does not read files, call APIs, or access any system state.
//! Retrieval results carry explicit `non_authorizing: true` markers.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A memory event with its embedding vector and optional temporal metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// Human-readable or machine-readable identifier for this memory event.
    pub id: String,
    /// The embedding vector (dense f64 array) representing this memory.
    pub embedding: Vec<f64>,
    /// Optional ordering timestamp. Events are sorted by this before processing.
    /// When `None`, events are kept in the order they are provided.
    pub timestamp: Option<i64>,
    /// Optional free-form label for audit/readback (e.g. "task outcome", "decision trace").
    pub label: Option<String>,
}

impl MemoryEvent {
    pub fn new(id: impl Into<String>, embedding: Vec<f64>) -> Self {
        Self {
            id: id.into(),
            embedding,
            timestamp: None,
            label: None,
        }
    }

    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Configuration for the compressed retrieval pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Dimensionality of the input embedding vectors.
    pub embedding_dimension: usize,
    /// Dimensionality of the compressed latent space.
    pub latent_dimension: usize,
    /// Local temporal window size for convolution (must be ≥ 1).
    /// window=3 means each event is averaged with its immediate predecessor and successor.
    pub window_size: usize,
    /// Number of top results to return.
    pub top_k: usize,
    /// Fixed seed for deterministic projection matrix generation.
    pub projection_seed: u64,
}

impl Config {
    /// Create a new Config with sensible defaults for the given dimensions.
    ///
    /// Defaults: window_size = 3, top_k = 5, projection_seed = 42.
    pub fn new(embedding_dimension: usize, latent_dimension: usize) -> Self {
        Self {
            embedding_dimension,
            latent_dimension,
            window_size: 3,
            top_k: 5,
            projection_seed: 42,
        }
    }

    /// Validate that all parameters are internally consistent.
    ///
    /// Returns `Ok(())` if valid, or a descriptive error string.
    pub fn validate(&self) -> Result<(), String> {
        if self.embedding_dimension == 0 {
            return Err("embedding_dimension must be > 0".to_string());
        }
        if self.latent_dimension == 0 {
            return Err("latent_dimension must be > 0".to_string());
        }
        if self.latent_dimension > self.embedding_dimension {
            return Err("latent_dimension must not exceed embedding_dimension".to_string());
        }
        if self.window_size == 0 {
            return Err("window_size must be ≥ 1".to_string());
        }
        if self.top_k == 0 {
            return Err("top_k must be ≥ 1".to_string());
        }
        Ok(())
    }
}

/// A single retrieval result with score and rank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// The memory event identifier.
    pub id: String,
    /// Cosine similarity score in [-1.0, 1.0].
    pub score: f64,
    /// Rank (1-based, lower is better / more relevant).
    pub rank: usize,
    /// The projected (latent) vector of this memory after convolution.
    pub latent: Vec<f64>,
}

/// Full retrieval response. Includes the explanation for audit/readback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResponse {
    /// Whether this response authorizes any action. Always `false`.
    pub non_authorizing: bool,
    /// The config used during this retrieval.
    pub config: Config,
    /// Total number of memory events considered.
    pub total_events: usize,
    /// Retrieval results, sorted by score descending.
    pub results: Vec<RetrievalResult>,
    /// Human-readable explanation of what was computed.
    pub explanation: String,
}

// ---------------------------------------------------------------------------
// Deterministic projection matrix generation
// ---------------------------------------------------------------------------

/// A deterministic projection matrix from embedding space to latent space.
///
/// Generated using a fixed-seed LCG (Linear Congruential Generator) so that
/// the same seed always produces the same matrix. No randomness or external
/// entropy is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionMatrix {
    pub rows: usize, // embedding_dimension
    pub cols: usize, // latent_dimension
    pub data: Vec<f64>,
}

impl ProjectionMatrix {
    /// Number of rows (embedding dimension).
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns (latent dimension).
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get the weight at (row, col).
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }
}

/// Deterministic LCG with a fixed seed.
///
/// Used to generate reproducible projection matrix weights.
fn deterministic_lcg(seed: u64, index: u64) -> u64 {
    // LCG parameters from Numerical Recipes (press et al.)
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;
    let mut state = seed.wrapping_add(index);
    state = state.wrapping_mul(A).wrapping_add(C);
    state = state.wrapping_mul(A).wrapping_add(C);
    state = state.wrapping_mul(A).wrapping_add(C);
    state
}

/// Generate a deterministic projection matrix.
///
/// The matrix is `embedding_dim × latent_dim`. Each entry is drawn from
/// a deterministic LCG seeded by `seed` and scaled to the range [-1.0, 1.0].
///
/// The generation is **O(embedding_dim × latent_dim)** and always produces
/// the same matrix for the same parameters.
pub fn generate_projection_matrix(
    embedding_dim: usize,
    latent_dim: usize,
    seed: u64,
) -> ProjectionMatrix {
    let total = embedding_dim * latent_dim;
    let mut data = Vec::with_capacity(total);
    for i in 0..total {
        let hash = deterministic_lcg(seed, i as u64);
        // Scale from [0, u64::MAX] to [-1.0, 1.0]
        let value = (hash as f64 / u64::MAX as f64) * 2.0 - 1.0;
        data.push(value);
    }
    ProjectionMatrix {
        rows: embedding_dim,
        cols: latent_dim,
        data,
    }
}

// ---------------------------------------------------------------------------
// Projection
// ---------------------------------------------------------------------------

/// Project memory events from embedding space to latent space.
///
/// For each event, computes: `latent_j = Σ_i embedding_i × matrix(i, j)`
///
/// Returns `None` if any embedding dimension mismatches `matrix.rows()`.
pub fn project(events: &[MemoryEvent], matrix: &ProjectionMatrix) -> Option<Vec<Vec<f64>>> {
    events
        .iter()
        .map(|ev| project_single(&ev.embedding, matrix))
        .collect::<Option<Vec<_>>>()
}

fn project_single(embedding: &[f64], matrix: &ProjectionMatrix) -> Option<Vec<f64>> {
    if embedding.len() != matrix.rows() {
        return None;
    }
    let mut latent = vec![0.0_f64; matrix.cols()];
    for j in 0..matrix.cols() {
        let mut sum = 0.0;
        for i in 0..matrix.rows() {
            sum += embedding[i] * matrix.get(i, j);
        }
        latent[j] = sum;
    }
    // L2-normalize the latent vector for proper cosine scoring
    let norm = l2_norm(&latent);
    if norm > 0.0 {
        for val in latent.iter_mut() {
            *val /= norm;
        }
    }
    Some(latent)
}

// ---------------------------------------------------------------------------
// Convolution
// ---------------------------------------------------------------------------

/// Apply local temporal convolution over the latent vectors.
///
/// For each event at position `i`, the output is the average of the latent
/// vectors at positions `max(0, i - half_window)` through
/// `min(len - 1, i + half_window)`, inclusive, where
/// `half_window = window_size.saturating_sub(1) / 2`.
///
/// Edge behaviour: partial windows at the boundaries use fewer neighbors
/// and are re-normalized so the output vector is not attenuated.
pub fn convolve(events: &[Vec<f64>], window_size: usize) -> Vec<Vec<f64>> {
    if events.is_empty() {
        return vec![];
    }
    let dim = events[0].len();
    let half = window_size.saturating_sub(1) / 2;
    let len = events.len();

    events
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(len);
            let count = end - start;

            let mut avg = vec![0.0_f64; dim];
            for pos in start..end {
                for d in 0..dim {
                    avg[d] += events[pos][d];
                }
            }
            // Normalize by count
            let inv = 1.0 / count as f64;
            for val in avg.iter_mut() {
                *val *= inv;
            }
            avg
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Compute the cosine similarity between two vectors.
///
/// Returns a value in [-1.0, 1.0]:
/// - `1.0` = identical direction
/// - `0.0` = orthogonal
/// - `-1.0` = opposite direction
///
/// If either vector is zero-norm, returns `0.0`.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

fn l2_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

// ---------------------------------------------------------------------------
// Retrieval (main entry point)
// ---------------------------------------------------------------------------

/// Retrieve top-k memory events most similar to a query vector.
///
/// Full pipeline: validate config → project query → project memories →
/// convolve memories → cosine score → sort → top-k → explain.
///
/// # Returns
///
/// A `RetrievalResponse` that is **always** `non_authorizing: true`.
pub fn retrieve(query: &[f64], events: &[MemoryEvent], config: &Config) -> RetrievalResponse {
    // 1. Validate config
    if let Err(msg) = config.validate() {
        return RetrievalResponse {
            non_authorizing: true,
            config: config.clone(),
            total_events: events.len(),
            results: vec![],
            explanation: format!("Config validation failed: {}", msg),
        };
    }

    // 2. Validate query dimension
    if query.len() != config.embedding_dimension {
        return RetrievalResponse {
            non_authorizing: true,
            config: config.clone(),
            total_events: events.len(),
            results: vec![],
            explanation: format!(
                "Query dimension {} does not match config embedding_dimension {}",
                query.len(),
                config.embedding_dimension
            ),
        };
    }

    // 3. Handle empty events
    if events.is_empty() {
        return RetrievalResponse {
            non_authorizing: true,
            config: config.clone(),
            total_events: 0,
            results: vec![],
            explanation: "No memory events provided for retrieval.".to_string(),
        };
    }

    // 4. Sort events by timestamp (stable: preserve insertion order for None)
    let mut sorted: Vec<&MemoryEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.timestamp);

    // 5. Check that all events have consistent embedding dimensions
    for ev in &sorted {
        if ev.embedding.len() != config.embedding_dimension {
            return RetrievalResponse {
                non_authorizing: true,
                config: config.clone(),
                total_events: events.len(),
                results: vec![],
                explanation: format!(
                    "Event '{}' has embedding dimension {} but config expects {}",
                    ev.id,
                    ev.embedding.len(),
                    config.embedding_dimension
                ),
            };
        }
    }

    // 6. Generate projection matrix
    let matrix = generate_projection_matrix(
        config.embedding_dimension,
        config.latent_dimension,
        config.projection_seed,
    );

    // 7. Project query to latent space
    let query_latent = match project_single(query, &matrix) {
        Some(v) => v,
        None => {
            return RetrievalResponse {
                non_authorizing: true,
                config: config.clone(),
                total_events: events.len(),
                results: vec![],
                explanation: "Query projection failed: dimension mismatch.".to_string(),
            };
        }
    };

    // 8. Project all memory events
    let latent_events: Vec<Vec<f64>> = sorted
        .iter()
        .filter_map(|ev| project_single(&ev.embedding, &matrix))
        .collect();

    // 9. Convolve
    let convolved = convolve(&latent_events, config.window_size);

    // 10. Score
    let mut scored: Vec<(&MemoryEvent, f64, &[f64])> = sorted
        .into_iter()
        .zip(convolved.iter())
        .map(|(ev, conv)| {
            let score = cosine_similarity(&query_latent, conv);
            (ev, score, conv.as_slice())
        })
        .collect();

    // 11. Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 12. Take top-k
    let k = config.top_k.min(scored.len());
    let results: Vec<RetrievalResult> = scored[..k]
        .iter()
        .enumerate()
        .map(|(rank, (ev, score, latent))| RetrievalResult {
            id: ev.id.clone(),
            score: *score,
            rank: rank + 1,
            latent: latent.to_vec(),
        })
        .collect();

    let count_str = if results.is_empty() {
        "none".to_string()
    } else {
        format!("{}", results.len())
    };
    let explanation = format!(
        "retrieved top-{} from {} events (embedding_dim={}, latent_dim={}, \
         window={}, seed={}) — {} returned, best score: {:.4}",
        config.top_k,
        events.len(),
        config.embedding_dimension,
        config.latent_dimension,
        config.window_size,
        config.projection_seed,
        count_str,
        results.first().map(|r| r.score).unwrap_or(0.0),
    );

    RetrievalResponse {
        non_authorizing: true,
        config: config.clone(),
        total_events: events.len(),
        results,
        explanation,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Deterministic LCG ---

    #[test]
    fn test_lcg_deterministic() {
        let a = deterministic_lcg(42, 0);
        let b = deterministic_lcg(42, 0);
        assert_eq!(a, b, "LCG must be deterministic for same seed+index");
    }

    #[test]
    fn test_lcg_different_seeds_different_output() {
        let a = deterministic_lcg(42, 5);
        let b = deterministic_lcg(123, 5);
        assert_ne!(a, b, "Different seeds should produce different values");
    }

    #[test]
    fn test_lcg_different_indices_different_output() {
        let a = deterministic_lcg(42, 0);
        let b = deterministic_lcg(42, 1);
        assert_ne!(a, b, "Different indices should produce different values");
    }

    // --- Projection matrix generation ---

    #[test]
    fn test_generate_projection_matrix_deterministic() {
        let m1 = generate_projection_matrix(4, 2, 42);
        let m2 = generate_projection_matrix(4, 2, 42);
        assert_eq!(m1.data, m2.data, "Same seed → same matrix");
    }

    #[test]
    fn test_generate_projection_matrix_dimensions() {
        let m = generate_projection_matrix(8, 3, 42);
        assert_eq!(m.rows(), 8);
        assert_eq!(m.cols(), 3);
        assert_eq!(m.data.len(), 24);
    }

    #[test]
    fn test_generate_projection_matrix_different_seed() {
        let m1 = generate_projection_matrix(4, 2, 42);
        let m2 = generate_projection_matrix(4, 2, 99);
        assert_ne!(m1.data, m2.data, "Different seeds → different matrices");
    }

    #[test]
    fn test_generate_projection_matrix_1x1() {
        let m = generate_projection_matrix(1, 1, 42);
        assert_eq!(m.rows(), 1);
        assert_eq!(m.cols(), 1);
    }

    #[test]
    fn test_generate_projection_matrix_values_in_range() {
        let m = generate_projection_matrix(100, 10, 42);
        for &val in &m.data {
            assert!(
                (-1.0..=1.0).contains(&val),
                "Value {} out of range [-1,1]",
                val
            );
        }
    }

    // --- Projection ---

    #[test]
    fn test_project_dimension_mismatch_returns_none() {
        let events = vec![MemoryEvent::new("e1", vec![1.0, 2.0, 3.0])];
        let matrix = generate_projection_matrix(4, 2, 42);
        let result = project(&events, &matrix);
        assert!(
            result.is_none(),
            "3-dim embedding vs 4-dim matrix should fail"
        );
    }

    #[test]
    fn test_project_basic() {
        let events = vec![
            MemoryEvent::new("e1", vec![1.0, 0.0, 0.0, 0.0]),
            MemoryEvent::new("e2", vec![0.0, 1.0, 0.0, 0.0]),
        ];
        let matrix = generate_projection_matrix(4, 2, 42);
        let result = project(&events, &matrix).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[1].len(), 2);
    }

    #[test]
    fn test_project_deterministic() {
        let events = vec![MemoryEvent::new("e1", vec![1.0, 0.0, 0.0, 0.0])];
        let matrix = generate_projection_matrix(4, 2, 42);
        let r1 = project(&events, &matrix).unwrap();
        let r2 = project(&events, &matrix).unwrap();
        assert_eq!(r1, r2, "Projection must be deterministic");
    }

    #[test]
    fn test_project_latent_normalized() {
        let events = vec![MemoryEvent::new("e1", vec![0.5, 1.5, -0.8, 2.0])];
        let matrix = generate_projection_matrix(4, 2, 42);
        let result = project(&events, &matrix).unwrap();
        let norm = l2_norm(&result[0]);
        // Due to floating point, should be very close to 1.0
        assert!(
            (norm - 1.0).abs() < 1e-10,
            "Latent vector should be L2-normalized, got norm = {}",
            norm
        );
    }

    #[test]
    fn test_project_zero_embedding() {
        let events = vec![MemoryEvent::new("e1", vec![0.0, 0.0, 0.0, 0.0])];
        let matrix = generate_projection_matrix(4, 2, 42);
        let result = project(&events, &matrix).unwrap();
        // Zero embedding projects to zero latent (which stays zero after norm check)
        let norm = l2_norm(&result[0]);
        assert_eq!(
            norm, 0.0,
            "Zero embedding → zero latent (skip normalization)"
        );
    }

    // --- Convolution ---

    #[test]
    fn test_convolve_empty() {
        let result = convolve(&[], 3);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convolve_window_one_no_change() {
        let events = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let result = convolve(&events, 1);
        assert_eq!(result, events, "Window 1 should not change vectors");
    }

    #[test]
    fn test_convolve_window_three_smoothing() {
        // Three identical events, one outlier in the middle
        let events = vec![
            vec![1.0, 0.0],
            vec![10.0, 10.0], // outlier
            vec![1.0, 0.0],
        ];
        let result = convolve(&events, 3);
        // Middle event should be averaged: [(1+10+1)/3, (0+10+0)/3] = [4.0, 3.33...]
        assert!(
            (result[1][0] - 4.0).abs() < 1e-10,
            "Expected smoothed x ≈ 4.0, got {}",
            result[1][0]
        );
        assert!(
            (result[1][1] - (10.0 / 3.0)).abs() < 1e-10,
            "Expected smoothed y ≈ 3.33, got {}",
            result[1][1]
        );
    }

    #[test]
    fn test_convolve_edge_left() {
        let events = vec![vec![1.0, 0.0], vec![2.0, 1.0]];
        let result = convolve(&events, 3);
        // First event: only itself and right neighbor
        assert_eq!(
            result[0],
            vec![1.5, 0.5],
            "Left edge should average with right neighbor"
        );
    }

    #[test]
    fn test_convolve_edge_right() {
        let events = vec![vec![1.0, 0.0], vec![2.0, 1.0]];
        let result = convolve(&events, 3);
        // Last event: only left neighbor and itself
        assert_eq!(
            result[1],
            vec![1.5, 0.5],
            "Right edge should average with left neighbor"
        );
    }

    #[test]
    fn test_convolve_window_larger_than_length() {
        let events = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        // window=100 → all events averaged over all 3
        let result = convolve(&events, 100);
        assert!(
            (result[0][0] - 3.0).abs() < 1e-10,
            "Expected centroid x=3.0, got {}",
            result[0][0]
        );
        assert!(
            (result[0][1] - 4.0).abs() < 1e-10,
            "Expected centroid y=4.0, got {}",
            result[0][1]
        );
    }

    // --- Cosine similarity ---

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let score = cosine_similarity(&v, &v);
        assert!(
            (score - 1.0).abs() < 1e-10,
            "Identical vectors → score 1.0, got {}",
            score
        );
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let score = cosine_similarity(&a, &b);
        assert!(
            (score - 0.0).abs() < 1e-10,
            "Orthogonal → score 0.0, got {}",
            score
        );
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let score = cosine_similarity(&a, &b);
        assert!(
            (score + 1.0).abs() < 1e-10,
            "Opposite → score -1.0, got {}",
            score
        );
    }

    #[test]
    fn test_cosine_similarity_partial() {
        let a = vec![2.0, 1.0];
        let b = vec![1.0, 2.0];
        let score = cosine_similarity(&a, &b);
        // dot = 2*1 + 1*2 = 4, norm_a = sqrt(5) ≈ 2.236, norm_b = sqrt(5) ≈ 2.236
        // score = 4 / 5 = 0.8
        assert!((score - 0.8).abs() < 1e-10, "Expected 0.8, got {}", score);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        let score = cosine_similarity(&a, &b);
        assert_eq!(score, 0.0, "Zero vector → score 0.0");
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0];
        let score = cosine_similarity(&a, &b);
        assert_eq!(score, 0.0, "Dimension mismatch → score 0.0");
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let score = cosine_similarity(&[], &[]);
        assert_eq!(score, 0.0, "Empty vectors → score 0.0");
    }

    // --- L2 norm ---

    #[test]
    fn test_l2_norm_basic() {
        let v = vec![3.0, 4.0];
        assert!((l2_norm(&v) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_l2_norm_zero() {
        assert_eq!(l2_norm(&[]), 0.0);
        assert_eq!(l2_norm(&[0.0, 0.0]), 0.0);
    }

    // --- Config validation ---

    #[test]
    fn test_config_validate_ok() {
        let cfg = Config::new(8, 4);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_config_validate_zero_embedding_dim() {
        let cfg = Config {
            embedding_dimension: 0,
            ..Config::new(8, 4)
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_latent_dim() {
        let cfg = Config {
            latent_dimension: 0,
            ..Config::new(8, 4)
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_latent_exceeds_embedding() {
        let cfg = Config {
            latent_dimension: 16,
            ..Config::new(8, 4)
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_window() {
        let cfg = Config {
            window_size: 0,
            ..Config::new(8, 4)
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_topk() {
        let cfg = Config {
            top_k: 0,
            ..Config::new(8, 4)
        };
        assert!(cfg.validate().is_err());
    }

    // --- Full retrieval pipeline ---

    #[test]
    fn test_retrieve_empty_events() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let config = Config::new(4, 2);
        let response = retrieve(&query, &[], &config);
        assert!(response.non_authorizing);
        assert_eq!(response.total_events, 0);
        assert!(response.results.is_empty());
        assert!(response.explanation.contains("No memory events"));
    }

    #[test]
    fn test_retrieve_basic() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![
            MemoryEvent::new("similar", vec![0.9, 0.1, 0.0, 0.0]),
            MemoryEvent::new("dissimilar", vec![0.0, 0.0, 0.9, 0.1]),
            MemoryEvent::new("middle", vec![0.5, 0.0, 0.5, 0.0]),
        ];
        let config = Config {
            embedding_dimension: 4,
            latent_dimension: 2,
            window_size: 1,
            top_k: 3,
            projection_seed: 42,
        };
        let response = retrieve(&query, &events, &config);
        assert!(response.non_authorizing);
        assert_eq!(response.total_events, 3);
        assert_eq!(response.results.len(), 3);
        // First result should have the highest score
        assert_eq!(response.results[0].rank, 1);
        assert!(response.results[0].score >= response.results[1].score);
    }

    #[test]
    fn test_retrieve_top_k() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = (0..10)
            .map(|i| {
                let v = vec![(10 - i) as f64 / 10.0, 0.0, 0.0, 0.0];
                MemoryEvent::new(format!("e{}", i), v)
            })
            .collect::<Vec<_>>();
        let config = Config {
            embedding_dimension: 4,
            latent_dimension: 2,
            window_size: 1,
            top_k: 3,
            projection_seed: 42,
        };
        let response = retrieve(&query, &events, &config);
        assert_eq!(response.results.len(), 3);
    }

    #[test]
    fn test_retrieve_config_validation_rejected() {
        let query = vec![1.0, 0.0];
        let events = vec![MemoryEvent::new("e1", vec![0.5, 0.5])];
        let cfg = Config {
            embedding_dimension: 0,
            ..Config::new(2, 1)
        };
        let response = retrieve(&query, &events, &cfg);
        assert!(response.results.is_empty());
        assert!(response.explanation.contains("Config validation failed"));
    }

    #[test]
    fn test_retrieve_query_dim_mismatch() {
        let query = vec![1.0, 0.0, 0.0]; // 3-dim, config expects 4
        let events = vec![MemoryEvent::new("e1", vec![0.5, 0.5, 0.5, 0.5])];
        let config = Config::new(4, 2);
        let response = retrieve(&query, &events, &config);
        assert!(response.results.is_empty());
        assert!(response.explanation.contains("dimension"));
    }

    #[test]
    fn test_retrieve_event_dim_mismatch() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![MemoryEvent::new("e1", vec![0.5, 0.5])]; // 2-dim, config expects 4
        let config = Config::new(4, 2);
        let response = retrieve(&query, &events, &config);
        assert!(response.results.is_empty());
        assert!(response.explanation.contains("dimension"));
    }

    #[test]
    fn test_retrieve_deterministic() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![
            MemoryEvent::new("e1", vec![0.9, 0.0, 0.0, 0.0]),
            MemoryEvent::new("e2", vec![0.1, 0.9, 0.0, 0.0]),
        ];
        let config = Config::new(4, 2);
        let r1 = retrieve(&query, &events, &config);
        let r2 = retrieve(&query, &events, &config);
        assert_eq!(r1.results.len(), r2.results.len());
        for (a, b) in r1.results.iter().zip(r2.results.iter()) {
            assert!(
                (a.score - b.score).abs() < 1e-12,
                "Scores must be identical across runs"
            );
        }
    }

    #[test]
    fn test_retrieve_sort_by_timestamp() {
        // Events sorted by timestamp regardless of query similarity.
        // Both have the same angular direction after normalization, so scores
        // are identical — the test just verifies no panic and correct retrieval.
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![
            MemoryEvent::new("old", vec![1.0, 0.0, 0.0, 0.0]).with_timestamp(100),
            MemoryEvent::new("new", vec![1.0, 0.0, 0.0, 0.0]).with_timestamp(200),
        ];
        let config = Config::new(4, 2);
        let response = retrieve(&query, &events, &config);
        assert_eq!(response.total_events, 2);
        assert_eq!(response.results.len(), 2);
        // Scores should be 1.0 since query and events are identical
        for r in &response.results {
            assert!(
                (r.score - 1.0).abs() < 1e-6,
                "Expected score ≈ 1.0, got {}",
                r.score
            );
        }
    }

    #[test]
    fn test_retrieve_non_authorizing_invariant() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![MemoryEvent::new("e1", vec![0.5, 0.5, 0.5, 0.5])];
        let config = Config::new(4, 2);
        let response = retrieve(&query, &events, &config);
        assert!(
            response.non_authorizing,
            "All retrieval must be non-authorizing"
        );
    }

    #[test]
    fn test_retrieve_with_temporal_effect() {
        // With window_size=3, events at similar timestamps influence each other.
        // Three events: two similar (A and C) flank a dissimilar one (B).
        // With window=3, B should be pulled toward A and C.
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![
            MemoryEvent::new("a", vec![1.0, 0.0, 0.0, 0.0]).with_timestamp(0),
            MemoryEvent::new("b", vec![0.0, 1.0, 0.0, 0.0]).with_timestamp(1), // outlier
            MemoryEvent::new("c", vec![1.0, 0.0, 0.0, 0.0]).with_timestamp(2),
        ];

        // Without convolution (window=1): a and c score high, b scores low
        let config_no_conv = Config {
            embedding_dimension: 4,
            latent_dimension: 2,
            window_size: 1,
            top_k: 3,
            projection_seed: 42,
        };
        let r_no_conv = retrieve(&query, &events, &config_no_conv);

        // With convolution (window=3): b is averaged with a and c
        let config_conv = Config {
            embedding_dimension: 4,
            latent_dimension: 2,
            window_size: 3,
            top_k: 3,
            projection_seed: 42,
        };
        let r_conv = retrieve(&query, &events, &config_conv);

        // Event 'b' should score higher with convolution (neighbors pull it toward query)
        let score_b_no_conv = r_no_conv
            .results
            .iter()
            .find(|r| r.id == "b")
            .map(|r| r.score)
            .unwrap_or(0.0);
        let score_b_conv = r_conv
            .results
            .iter()
            .find(|r| r.id == "b")
            .map(|r| r.score)
            .unwrap_or(0.0);
        assert!(
            score_b_conv > score_b_no_conv,
            "Convolution should pull 'b' toward its neighbors: no_conv={:.4}, conv={:.4}",
            score_b_no_conv,
            score_b_conv
        );
    }

    #[test]
    fn test_retrieve_explanation_format() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![MemoryEvent::new("e1", vec![0.9, 0.0, 0.0, 0.0])];
        let config = Config {
            embedding_dimension: 4,
            latent_dimension: 2,
            window_size: 3,
            top_k: 1,
            projection_seed: 42,
        };
        let response = retrieve(&query, &events, &config);
        assert!(response.explanation.contains("top-1"));
        assert!(response.explanation.contains("embedding_dim=4"));
        assert!(response.explanation.contains("latent_dim=2"));
        assert!(response.explanation.contains("window=3"));
        assert!(response.explanation.contains("seed=42"));
    }

    // --- Serialization round-trips ---

    #[test]
    fn test_memory_event_serde_roundtrip() {
        let event = MemoryEvent::new("test-id", vec![1.0, 2.0, 3.0])
            .with_timestamp(1000)
            .with_label("test label");
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MemoryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, event.id);
        assert_eq!(deserialized.embedding, event.embedding);
        assert_eq!(deserialized.timestamp, event.timestamp);
        assert_eq!(deserialized.label, event.label);
    }

    #[test]
    fn test_retrieval_response_serde_roundtrip() {
        let response = RetrievalResponse {
            non_authorizing: true,
            config: Config::new(8, 4),
            total_events: 10,
            results: vec![RetrievalResult {
                id: "e1".to_string(),
                score: 0.95,
                rank: 1,
                latent: vec![0.6, 0.8],
            }],
            explanation: "test".to_string(),
        };
        let json = serde_json::to_string_pretty(&response).unwrap();
        let deserialized: RetrievalResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.non_authorizing);
        assert_eq!(deserialized.total_events, 10);
        assert_eq!(deserialized.results[0].id, "e1");
        assert!((deserialized.results[0].score - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = Config {
            embedding_dimension: 128,
            latent_dimension: 16,
            window_size: 5,
            top_k: 10,
            projection_seed: 12345,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.embedding_dimension, 128);
        assert_eq!(deserialized.latent_dimension, 16);
        assert_eq!(deserialized.window_size, 5);
        assert_eq!(deserialized.top_k, 10);
        assert_eq!(deserialized.projection_seed, 12345);
    }

    // --- Edge cases ---

    #[test]
    fn test_retrieve_single_event() {
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let events = vec![MemoryEvent::new("only", vec![0.5, 0.5, 0.5, 0.5])];
        let config = Config::new(4, 2);
        let response = retrieve(&query, &events, &config);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].id, "only");
    }

    #[test]
    fn test_retrieve_query_identical_to_event() {
        let query = vec![0.5, 0.5, 0.5, 0.5];
        let events = vec![MemoryEvent::new("same", vec![0.5, 0.5, 0.5, 0.5])];
        let config = Config::new(4, 2);
        let response = retrieve(&query, &events, &config);
        assert_eq!(response.results.len(), 1);
        // Score should be 1.0 (identical after normalization)
        assert!(
            (response.results[0].score - 1.0).abs() < 1e-6,
            "Identical query and event should score 1.0, got {}",
            response.results[0].score
        );
    }
}
