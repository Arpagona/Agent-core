//! CompressedCognitiveAttentionAdapter — ContextAssembler implementation backed by
//! the Compressed Convolutional Memory Retrieval crate.
//!
//! This adapter bridges `crates/compressed-cognitive-attention` (a deterministic,
//! non-authorizing memory retrieval mechanism) into the Neutral Orchestrator's
//! context assembly pipeline.
//!
//! When the orchestrator asks for advisory CompressedCognitiveAttention context,
//! this adapter:
//!   - Generates a deterministic embedding vector from the objective text
//!   - Runs the full CCA pipeline: projection → convolution → cosine scoring → top-k
//!   - Returns the best-matching memory events as advisory `ContextItem` values
//!
//! # Safety invariants
//!
//! - All items are advisory and non-authorizing.
//! - No response may contain an approval, authorization or execution token.
//! - Retrieval scores are purely advisory — they indicate compressed similarity,
//!   not truth or authority.
//! - The deterministic text-to-embedding function is a stand-in for a real
//!   embedding model and is explicitly documented as such.
//!
//! # Usage
//!
//! ```ignore
//! use arpagona_compressed_cognitive_attention::{Config, MemoryEvent};
//! use arpagona_neutral_orchestrator::CompressedCognitiveAttentionAdapter;
//!
//! let events = vec![];
//! let config = Config::new(16, 4);  // 16-dim embedding, 4-dim latent
//! let adapter = CompressedCognitiveAttentionAdapter::new(events, config);
//! let engine = OrchestratorEngine::new()
//!     .with_context_assembler(Box::new(adapter));
//! ```

use crate::context_assembler::ContextAssembler;
use arpagona_agent_core::cognitive_work::ContextItem;
use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};
use arpagona_compressed_cognitive_attention::{self as cca, Config, MemoryEvent};
use std::sync::Mutex;

// ─── Deterministic text-to-embedding constants ──────────────────────────────

/// Default embedding dimension for synthesized queries and stored events.
const DEFAULT_EMBEDDING_DIM: usize = 16;

/// Default latent dimension for the CCA pipeline.
const DEFAULT_LATENT_DIM: usize = 4;

// ─── CompressedCognitiveAttentionAdapter ────────────────────────────────────

/// A ContextAssembler that uses Compressed Convolutional Memory Retrieval to
/// provide advisory temporally-enriched context for the orchestrator.
///
/// This adapter provides real compressed-memory retrieval:
///   - Stores a set of `MemoryEvent` values with pre-computed embeddings
///   - Generates a deterministic query embedding from objective text
///   - Runs the full CCA pipeline via `cca::retrieve()`
///   - Returns matched event summaries as `ContextItem` values
///   - All results are advisory only
///
/// # Embedding model
///
/// The adapter uses a **deterministic**, **hash-based** text-to-embedding
/// function as a stand-in for a real embedding model. This function uses the
/// same deterministic LCG as the CCA crate itself to produce reproducible
/// `Vec<f64>` vectors from any input text. It is NOT a neural embedding —
/// it is a reproducible spatial hash for alpha demonstration and testing.
///
/// When a real embedding model becomes available (local or cloud-based), the
/// function can be replaced without changing the CCA pipeline integration.
///
/// # Configuration
///
/// | Parameter | Default | Description |
/// |---|---|---|
/// | `embedding_dimension` | 16 | Dimensionality of the embedding vectors |
/// | `latent_dimension` | 4 | Dimensionality of the compressed latent space |
/// | `window_size` | 3 | Temporal convolution window size |
/// | `top_k` | 5 | Number of retrieval results to return |
/// | `projection_seed` | 42 | Seed for deterministic projection matrix |
/// | `max_items` | 10 | Max items returned to orchestrator |
pub struct CompressedCognitiveAttentionAdapter {
    /// Stored memory events with pre-computed embeddings.
    events: Mutex<Vec<MemoryEvent>>,
    /// CCA configuration for retrieval.
    config: Config,
    /// Embedding dimension (may differ from config for projection).
    embedding_dim: usize,
    /// Maximum items to return per query.
    max_items: usize,
}

impl CompressedCognitiveAttentionAdapter {
    /// Create a new CompressedCognitiveAttentionAdapter.
    ///
    /// The `events` are pre-computed `MemoryEvent` values with embeddings.
    /// The `config` controls the CCA retrieval pipeline parameters.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let events = vec![]; // pre-computed memory events
    /// let config = Config::new(16, 4);
    /// let adapter = CompressedCognitiveAttentionAdapter::new(events, config);
    /// ```
    pub fn new(events: Vec<MemoryEvent>, config: Config) -> Self {
        let embedding_dim = config.embedding_dimension;
        Self {
            events: Mutex::new(events),
            config,
            embedding_dim,
            max_items: 10,
        }
    }

    /// Create a new adapter with default CCA configuration.
    ///
    /// Defaults: embedding_dim=16, latent_dim=4, window_size=3, top_k=5, seed=42.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let events = vec![]; // pre-computed memory events
    /// let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
    /// ```
    pub fn with_defaults(events: Vec<MemoryEvent>) -> Self {
        Self {
            events: Mutex::new(events),
            config: Config::new(DEFAULT_EMBEDDING_DIM, DEFAULT_LATENT_DIM),
            embedding_dim: DEFAULT_EMBEDDING_DIM,
            max_items: 10,
        }
    }

    /// Override the maximum number of retrieval results to return.
    ///
    /// The default is 10. Setting this higher may return more context items
    /// but could include lower-scoring events.
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = max;
        self
    }

    /// Set a custom embedding dimension (must match stored events).
    pub fn with_embedding_dim(mut self, dim: usize) -> Self {
        self.embedding_dim = dim;
        self
    }

    /// Add a memory event to the adapter's store.
    ///
    /// Returns an error if the event's embedding dimension doesn't match
    /// the adapter's configured `embedding_dim`.
    pub fn add_event(&self, event: MemoryEvent) -> Result<(), String> {
        if event.embedding.len() != self.embedding_dim {
            return Err(format!(
                "Event '{}' has embedding dimension {} but adapter expects {}",
                event.id,
                event.embedding.len(),
                self.embedding_dim
            ));
        }
        let mut store = self
            .events
            .lock()
            .map_err(|e| format!("Storage lock poisoned: {}", e))?;
        store.push(event);
        Ok(())
    }

    // ─── Deterministic text-to-embedding ──────────────────────────────────

    /// Generate a deterministic embedding vector from input text.
    ///
    /// This function produces a `Vec<f64>` of length `embedding_dim` where
    /// each position is influenced by character n-gram hashes of the input
    /// text. The same text always produces the same vector.
    ///
    /// This is a **deterministic stand-in** for a real embedding model. It
    /// is NOT a neural embedding — it is a reproducible spatial hash used
    /// for alpha demonstration and testing purposes.
    ///
    /// # Algorithm
    ///
    /// For each word in the text, a deterministic hash (via the same LCG
    /// used by the CCA crate) determines which positions in the embedding
    /// vector are incremented. After processing all words, the vector is
    /// L2-normalized so it can be used as a query in the CCA pipeline.
    fn text_to_embedding(&self, text: &str) -> Vec<f64> {
        let dim = self.embedding_dim;
        let mut vec = vec![0.0_f64; dim];

        // Split text into words, filter short tokens
        let words: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()))
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            // Edge case: all short tokens — return uniform zero-norm vector
            // (cosine similarity will be 0.0 against everything)
            return vec;
        }

        for word in &words {
            // Use the deterministic LCG to hash the word
            let hash = Self::word_hash(word);
            // Use the hash to decide which dimensions to activate
            let pos = (hash % dim as u64) as usize;
            let amp = (hash as f64 / u64::MAX as f64) * 2.0 - 1.0; // [-1.0, 1.0]
            vec[pos] += amp;

            // Activate a second position for richer representation
            let hash2 = Self::word_hash(&format!("{}:{}", word, word.len()));
            let pos2 = (hash2 % dim as u64) as usize;
            let amp2 = (hash2 as f64 / u64::MAX as f64) * 2.0 - 1.0;
            vec[pos2] += amp2;
        }

        // L2-normalize
        let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for val in vec.iter_mut() {
                *val /= norm;
            }
        }

        vec
    }

    /// Deterministic hash of a word using the CCA crate's LCG parameters.
    fn word_hash(word: &str) -> u64 {
        // Use the same LCG constants as the CCA crate but with a different
        // seed so we get different values than the projection matrix.
        const A: u64 = 6364136223846793005;
        const C: u64 = 1442695040888963407;
        let bytes = word.as_bytes();
        let mut state: u64 = 9999999999999999999; // arbitrary seed for word hashing
        for &b in bytes {
            state = state.wrapping_mul(A).wrapping_add(C);
            state ^= b as u64;
        }
        // Mix a few more rounds
        state = state.wrapping_mul(A).wrapping_add(C);
        state = state.wrapping_mul(A).wrapping_add(C);
        state
    }

    /// Extract a readable label from a MemoryEvent (fallback to id).
    fn event_label_or_id(event: &MemoryEvent) -> String {
        event.label.clone().unwrap_or_else(|| event.id.clone())
    }
}

impl ContextAssembler for CompressedCognitiveAttentionAdapter {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            let response = match source {
                ContextSource::CompressedCognitiveAttention => {
                    self.assemble_compressed_cognitive_attention(request)
                }
                _ => MemoryQueryResponse::new(source.clone()),
            };
            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![ContextSource::CompressedCognitiveAttention]
    }
}

// ─── Internal assembly logic ────────────────────────────────────────────────

impl CompressedCognitiveAttentionAdapter {
    /// Assemble Compressed Cognitive Attention context: generate a deterministic
    /// query embedding, run the CCA pipeline, and convert matches into ContextItems.
    ///
    /// Compute route awareness: when `local_preferred` is true, the adapter
    /// returns fewer retrieval results (lighter computation). When a cloud/strong
    /// route is indicated, it returns more results for richer context.
    fn assemble_compressed_cognitive_attention(
        &self,
        request: &MemoryQueryRequest,
    ) -> MemoryQueryResponse {
        // ── Compute-route aware limit adjustment ───────────────────────
        let route_suffix = if let Some(ref label) = request.compute_route_label {
            let local = request.local_preferred.unwrap_or(false);
            format!(" [compute: {} | local: {}]", label, local)
        } else {
            String::new()
        };

        // Local routes: fewer results (lighter); cloud routes: more
        let effective_max = if request.local_preferred.unwrap_or(false) {
            std::cmp::max(1, self.max_items.saturating_sub(self.max_items / 2))
        } else {
            self.max_items
        };

        // Step 1: Lock the event store
        let events = match self.events.lock() {
            Ok(e) => e,
            Err(e) => {
                let _msg = format!("CCA event store lock poisoned: {}", e);
                return MemoryQueryResponse::new(ContextSource::CompressedCognitiveAttention)
                    .with_unavailable();
            }
        };

        // Step 2: Handle empty store
        if events.is_empty() {
            return MemoryQueryResponse::new(ContextSource::CompressedCognitiveAttention)
                .with_explanation(
                    "Compressed Cognitive Attention store is empty — no memory events available.",
                );
        }

        // Step 3: Generate deterministic query embedding from objective text
        let query = self.text_to_embedding(&request.objective_text);

        // Step 4: Create a modified config that respects the request max items
        let effective_k = self.config.top_k.min(effective_max);
        let request_k = request.max_items_per_source.min(effective_k);
        let mut run_config = self.config.clone();
        run_config.top_k = request_k.max(1);

        // Step 5: Run the CCA pipeline
        let response = cca::retrieve(&query, &events, &run_config);

        // Step 6: Convert results into advisory ContextItems
        let items: Vec<ContextItem> = response
            .results
            .iter()
            .map(|r| {
                let event = events.iter().find(|e| e.id == r.id);
                let label = event
                    .map(Self::event_label_or_id)
                    .unwrap_or_else(|| r.id.clone());

                ContextItem {
                    key: format!("cca_result:{}", r.id),
                    value: format!("[CCA rank={} score={:.4}] {}", r.rank, r.score, label,),
                    source: "compressed_cognitive_attention_adapter".to_owned(),
                }
            })
            .collect();

        let count = items.len();

        MemoryQueryResponse {
            source: ContextSource::CompressedCognitiveAttention,
            items,
            available: true,
            explanation: format!(
                "{} — returned {} item(s) to orchestrator.{}",
                response.explanation, count, route_suffix
            ),
        }
    }
}

// ─── Helper extension for MemoryQueryResponse ───────────────────────────────

/// Convenience method for setting a custom explanation on a "no items" response.
trait MemoryQueryResponseExt {
    fn with_explanation(self, explanation: &str) -> Self;
}

impl MemoryQueryResponseExt for MemoryQueryResponse {
    fn with_explanation(mut self, explanation: &str) -> Self {
        self.explanation = explanation.to_owned();
        self
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId, WorkspaceId};
    use arpagona_compressed_cognitive_attention::MemoryEvent;

    fn make_request() -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "compressed cognitive memory retrieval test",
            WorkspaceId::new("ws-test"),
        )
    }

    fn make_request_with_text(text: &str) -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            text,
            WorkspaceId::new("ws-test"),
        )
    }

    fn make_request_with_sources(text: &str, sources: Vec<ContextSource>) -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            text,
            WorkspaceId::new("ws-test"),
        )
        .with_sources(sources)
    }

    /// Create a deterministic embedding vector for testing.
    /// Uses a fixed seed LCG (same approach as the CCA crate) so tests
    /// produce reproducible embeddings.
    fn make_embedding(dim: usize, seed: u64) -> Vec<f64> {
        const A: u64 = 6364136223846793005;
        const C: u64 = 1442695040888963407;
        let mut state = seed;
        let mut vec = Vec::with_capacity(dim);
        for _ in 0..dim {
            state = state.wrapping_mul(A).wrapping_add(C);
            let val = (state as f64 / u64::MAX as f64) * 2.0 - 1.0;
            vec.push(val);
        }
        // L2-normalize
        let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        }
        vec
    }

    /// Create a test memory event with a deterministic embedding.
    fn make_event(id: &str, dim: usize, seed: u64) -> MemoryEvent {
        MemoryEvent::new(id.to_owned(), make_embedding(dim, seed))
            .with_label(format!("test-event-{}", id))
    }

    // ─── Supported sources ─────────────────────────────────────────────────

    #[test]
    fn adapter_returns_cca_source() {
        let events = vec![];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let sources = adapter.supported_sources();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&ContextSource::CompressedCognitiveAttention));
    }

    // ─── Non-matching source ───────────────────────────────────────────────

    #[test]
    fn adapter_ignores_non_matching_sources() {
        let events = vec![];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let request = make_request_with_sources("test", vec![ContextSource::GraphMemory]);
        let responses = adapter.assemble(&request);
        assert_eq!(responses.len(), 1);
        let resp = &responses[0];
        assert_eq!(resp.source, ContextSource::GraphMemory);
        assert!(resp.items.is_empty());
        assert!(resp.available);
    }

    // ─── Empty store returns empty but available ───────────────────────────

    #[test]
    fn adapter_handles_empty_store() {
        let events = vec![];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        assert!(resp.available);
        assert!(resp.items.is_empty());
        assert!(resp.explanation.contains("empty"));
    }

    // ─── Text-to-embedding ─────────────────────────────────────────────────

    #[test]
    fn text_to_embedding_returns_correct_dimension() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let emb = adapter.text_to_embedding("cognitive architecture memory retrieval");
        assert_eq!(emb.len(), DEFAULT_EMBEDDING_DIM);
    }

    #[test]
    fn text_to_embedding_is_deterministic() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let emb1 = adapter.text_to_embedding("cognitive architecture memory retrieval");
        let emb2 = adapter.text_to_embedding("cognitive architecture memory retrieval");
        assert_eq!(emb1, emb2);
    }

    #[test]
    fn text_to_embedding_differs_for_different_text() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let emb1 = adapter.text_to_embedding("cognitive architecture memory retrieval");
        let emb2 = adapter.text_to_embedding("completely different unrelated topic");
        assert_ne!(emb1, emb2);
    }

    #[test]
    fn text_to_embedding_handles_empty_text() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let emb = adapter.text_to_embedding("");
        // Should be zero vector (all short tokens or empty)
        assert_eq!(emb.len(), DEFAULT_EMBEDDING_DIM);
        // All values should be 0.0 due to zero norm
        assert!(emb.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn text_to_embedding_handles_short_tokens_only() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let emb = adapter.text_to_embedding("a an at");
        // All tokens are <= 2 chars, so no words contribute
        // Should be zero vector
        assert_eq!(emb.len(), DEFAULT_EMBEDDING_DIM);
        assert!(emb.iter().all(|v| *v == 0.0));
    }

    // ─── Retrieval with stored events ──────────────────────────────────────

    #[test]
    fn adapter_returns_matching_events() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![
            make_event("memory-1", dim, 100),
            make_event("memory-2", dim, 200),
            make_event("memory-3", dim, 300),
        ];

        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events.clone());
        let request = make_request_with_text("cognitive memory retrieval");

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        assert!(resp.available);
        assert!(!resp.items.is_empty(), "Should find matching events");
        assert!(
            resp.explanation.contains("retrieved"),
            "Explanation should indicate retrieval: {}",
            resp.explanation
        );
    }

    #[test]
    fn adapter_context_items_contain_scores() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![
            make_event("memory-1", dim, 100),
            make_event("memory-2", dim, 200),
        ];

        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let request = make_request_with_text("cognitive memory");

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        for item in &resp.items {
            assert!(
                item.value.contains("score="),
                "Item should contain score info: {}",
                item.value
            );
            assert!(
                item.value.contains("CCA"),
                "Item should be marked as CCA: {}",
                item.value
            );
            assert_eq!(
                item.source, "compressed_cognitive_attention_adapter",
                "Item source should identify the adapter"
            );
        }
    }

    // ─── Max items limit ───────────────────────────────────────────────────

    #[test]
    fn adapter_respects_max_items() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events: Vec<MemoryEvent> = (0..8)
            .map(|i| make_event(&format!("memory-{}", i), dim, 100 + i as u64))
            .collect();

        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events).with_max_items(3);
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        assert!(
            resp.items.len() <= 3,
            "Should have at most 3 items, got {}",
            resp.items.len()
        );
    }

    // ─── Add event ─────────────────────────────────────────────────────────

    #[test]
    fn adapter_add_event_increases_stored_count() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let event = make_event("new-event", DEFAULT_EMBEDDING_DIM, 42);
        let result = adapter.add_event(event);
        assert!(result.is_ok());

        let request = make_request_with_text("test");
        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        assert!(!resp.items.is_empty(), "Should find the newly added event");
    }

    #[test]
    fn adapter_add_event_rejects_wrong_dimension() {
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(vec![]);
        let wrong_event = MemoryEvent::new("wrong-dim", vec![1.0, 2.0, 3.0]); // 3-dim, not 16
        let result = adapter.add_event(wrong_event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("embedding dimension"));
    }

    // ─── Determinism ───────────────────────────────────────────────────────

    #[test]
    fn adapter_retrieval_is_deterministic() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![
            make_event("mem-1", dim, 100),
            make_event("mem-2", dim, 200),
            make_event("mem-3", dim, 300),
        ];

        let adapter1 = CompressedCognitiveAttentionAdapter::with_defaults(events.clone());
        let adapter2 = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let request = make_request_with_text("cognitive memory retrieval");

        let resp1 = adapter1.assemble(&request);
        let resp2 = adapter2.assemble(&request);

        let cca1 = resp1
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention)
            .unwrap();
        let cca2 = resp2
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention)
            .unwrap();

        // Same input → same output (deterministic)
        assert_eq!(cca1.items.len(), cca2.items.len());
        assert_eq!(cca1.explanation, cca2.explanation);
    }

    // ─── Non-authorizing invariant ─────────────────────────────────────────

    #[test]
    fn adapter_response_never_contains_approval() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![make_event("mem-1", dim, 100)];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let request = make_request_with_text("cognitive");

        let responses = adapter.assemble(&request);
        for resp in &responses {
            let json = serde_json::to_value(resp).expect("should serialize");
            assert!(json.get("approved").is_none());
            assert!(json.get("authorized").is_none());
            assert!(json.get("execution_token").is_none());
        }
    }

    // ─── Compute-route awareness tests ─────────────────────────────────

    #[test]
    fn cca_adapter_local_route_reduces_results() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![
            make_event("mem-1", dim, 100),
            make_event("mem-2", dim, 200),
            make_event("mem-3", dim, 300),
            make_event("mem-4", dim, 400),
            make_event("mem-5", dim, 500),
        ];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events).with_max_items(5);
        let request = make_request_with_text("cognitive memory")
            .with_compute_route(Some("local-small"), Some(true));

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        // Local route reduces ~half — should have 3 or fewer items
        assert!(
            resp.items.len() <= 3,
            "Local route should limit items, got {}",
            resp.items.len()
        );
        assert!(
            resp.explanation.contains("local"),
            "Explanation should mention route: {}",
            resp.explanation
        );
    }

    #[test]
    fn cca_adapter_cloud_route_full_items() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![
            make_event("mem-1", dim, 100),
            make_event("mem-2", dim, 200),
            make_event("mem-3", dim, 300),
        ];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events).with_max_items(5);
        let request = make_request_with_text("cognitive memory")
            .with_compute_route(Some("cloud-strong"), Some(false));

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        assert!(
            resp.explanation.contains("compute: cloud-strong"),
            "Explanation should mention cloud route: {}",
            resp.explanation
        );
    }

    #[test]
    fn cca_adapter_default_route_has_no_compute_prefix() {
        let dim = DEFAULT_EMBEDDING_DIM;
        let events = vec![make_event("mem-1", dim, 100)];
        let adapter = CompressedCognitiveAttentionAdapter::with_defaults(events);
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let cca_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::CompressedCognitiveAttention);
        assert!(cca_resp.is_some());
        let resp = cca_resp.unwrap();

        assert!(
            !resp.explanation.contains("[compute:"),
            "No compute prefix expected: {}",
            resp.explanation
        );
    }
}
