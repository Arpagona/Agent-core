//! HolographicMemoryAdapter — ContextAssembler implementation backed by
//! Holographic Memory resonance retrieval.
//!
//! This adapter bridges the `crates/holographic-memory` symbolic associative
//! memory crate into the Neutral Orchestrator's context assembly pipeline.
//!
//! When the orchestrator asks for advisory Holographic Memory context, this
//! adapter:
//! - Creates a `HolographicQuery` from the objective text (splitting words
//!   into keywords for resonance matching)
//! - Calls `retrieve_by_resonance` to find top-matching traces
//! - Returns resonance match summaries as advisory `ContextItem` values
//!
//! # Safety invariants
//!
//! - All items are advisory and non-authorizing.
//! - No response may contain an approval, authorization or execution token.
//! - Resonance scores are purely advisory — they indicate pattern overlap,
//!   not truth or authority.
//! - If the store is unavailable or contains no matching traces, the adapter
//!   reports the source as available but with zero items and a clear
//!   explanation.
//! - Interior mutability via `Mutex` allows the store to increment activation
//!   counts while the `ContextAssembler::assemble` interface uses `&self`.
//!
//! # Usage
//!
//! ```ignore
//! use arpagona_holographic_memory::InMemoryHolographicMemoryStore;
//! use arpagona_neutral_orchestrator::HolographicMemoryAdapter;
//!
//! let store = InMemoryHolographicMemoryStore::new();
//! let adapter = HolographicMemoryAdapter::new(store, "project-alpha");
//! let engine = OrchestratorEngine::new()
//!     .with_context_assembler(Box::new(adapter));
//! ```

use crate::context_assembler::ContextAssembler;
use arpagona_agent_core::cognitive_work::ContextItem;
use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};
use arpagona_holographic_memory::{HolographicMemoryStore, HolographicQuery};
use std::sync::Mutex;

// ─── HolographicMemoryAdapter ───────────────────────────────────────────────

/// A ContextAssembler that uses Holographic Memory resonance retrieval to
/// provide advisory pattern-matching context for the orchestrator.
///
/// This adapter provides real associative memory context from prior traces:
/// - Splits the objective text into keywords for resonance matching
/// - Queries the Holographic Memory store via `retrieve_by_resonance`
/// - Returns matched trace summaries as `ContextItem` values
/// - All results are advisory only
///
/// # Configuration
///
/// The adapter requires a `HolographicMemoryStore` implementation and a
/// `project_id` string that scopes the resonance retrieval. Use `new()` for
/// the simplest construction.
pub struct HolographicMemoryAdapter {
    /// The Holographic Memory store wrapped in a Mutex for interior mutability.
    /// `retrieve_by_resonance` requires `&mut self` (activation counts), but
    /// `ContextAssembler::assemble` takes `&self`.
    store: Mutex<Box<dyn HolographicMemoryStore>>,
    /// Project scope — only traces with this `project_id` are considered.
    project_id: String,
    /// Maximum items to return per query.
    max_items: usize,
}

impl HolographicMemoryAdapter {
    /// Create a new HolographicMemoryAdapter with the given store and project.
    ///
    /// The `project_id` scopes all resonance queries — only traces belonging
    /// to this project are returned.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let store = InMemoryHolographicMemoryStore::new();
    /// let adapter = HolographicMemoryAdapter::new(store, "my-project");
    /// ```
    pub fn new(
        store: impl HolographicMemoryStore + 'static,
        project_id: impl Into<String>,
    ) -> Self {
        Self {
            store: Mutex::new(Box::new(store)),
            project_id: project_id.into(),
            max_items: 10,
        }
    }

    /// Create a HolographicMemoryAdapter from an already-boxed store.
    ///
    /// Use this when you have a store on the heap (e.g., `SqliteHolographicMemoryStore`)
    /// and want to avoid boxing it again.
    pub fn from_boxed_store(
        store: Box<dyn HolographicMemoryStore>,
        project_id: impl Into<String>,
    ) -> Self {
        Self {
            store: Mutex::new(store),
            project_id: project_id.into(),
            max_items: 10,
        }
    }

    /// Override the maximum number of resonance matches to return.
    ///
    /// The default is 10. Setting this higher may increase latency for stores
    /// with many traces.
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = max;
        self
    }

    /// Extract keywords from objective text by splitting on whitespace.
    ///
    /// Filters out empty strings and very short tokens (1-2 chars) that would
    /// produce noisy resonance matches. Returns up to `max_keywords` terms.
    fn extract_keywords(text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()))
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect()
    }
}

impl ContextAssembler for HolographicMemoryAdapter {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            let response = match source {
                ContextSource::HolographicMemory => self.assemble_holographic_memory(request),
                _ => MemoryQueryResponse::new(source.clone()),
            };
            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![ContextSource::HolographicMemory]
    }
}

// ─── Internal assembly logic ───────────────────────────────────────────────

impl HolographicMemoryAdapter {
    /// Assemble Holographic Memory context: create a query from the objective
    /// text, run resonance retrieval, and convert matches into ContextItems.
    ///
    /// Compute route awareness: when `local_preferred` is true, the adapter
    /// returns fewer resonance matches (lighter retrieval). When a cloud/strong
    /// route is indicated, more matches are returned for richer pattern resonance.
    fn assemble_holographic_memory(&self, request: &MemoryQueryRequest) -> MemoryQueryResponse {
        // ── Compute-route aware limit adjustment ───────────────────────
        let route_suffix = if let Some(ref label) = request.compute_route_label {
            let local = request.local_preferred.unwrap_or(false);
            format!(" [compute: {} | local: {}]", label, local)
        } else {
            String::new()
        };

        // Local routes: fewer traces (lighter retrieval); cloud routes: more traces
        let effective_max = if request.local_preferred.unwrap_or(false) {
            std::cmp::max(1, self.max_items.saturating_sub(self.max_items / 2))
        } else {
            self.max_items
        };

        // Step 1: Extract keywords from objective text
        let keywords = Self::extract_keywords(&request.objective_text);

        if keywords.is_empty() {
            return MemoryQueryResponse::new(ContextSource::HolographicMemory).with_items(vec![]);
        }

        // Step 2: Create the holographic query
        let query = HolographicQuery::new(
            self.project_id.clone(),
            request.objective_text.clone(),
            keywords,
            vec![], // concepts — not extracted from raw text in this adapter
            vec![], // entities — not extracted from raw text in this adapter
        );

        // Step 3: Query the store and convert results
        let mut store = match self.store.lock() {
            Ok(s) => s,
            Err(e) => {
                let _msg = format!("Holographic Memory store lock poisoned: {}", e);
                return MemoryQueryResponse::new(ContextSource::HolographicMemory)
                    .with_unavailable();
            }
        };

        let result = store.retrieve_by_resonance(&self.project_id, query, effective_max);

        // Step 4: Convert resonance matches into advisory ContextItems
        let items: Vec<ContextItem> = result
            .matches
            .iter()
            .map(|m| {
                let score_info = format!(
                    "total={:.3} symbolic={:.3} concept={:.3} entity={:.3} decision={:.3}",
                    m.score.total,
                    m.score.symbolic_overlap,
                    m.score.concept_overlap,
                    m.score.entity_overlap,
                    m.score.decision_overlap,
                );

                ContextItem {
                    key: format!("holographic_trace:{}", m.trace.id),
                    value: format!(
                        "[resonance {}] {} — keywords: {:?}, concepts: {:?}, entities: {:?}",
                        score_info,
                        m.trace.content_summary,
                        m.matched_keywords,
                        m.matched_concepts,
                        m.matched_entities,
                    ),
                    source: "holographic_memory_adapter".to_owned(),
                }
            })
            .collect();

        let count = items.len();

        MemoryQueryResponse {
            source: ContextSource::HolographicMemory,
            items,
            available: true,
            explanation: format!(
                "Holographic Memory resonance found {} trace(s) in project '{}'. {}{}",
                count, self.project_id, result.reconstruction_summary, route_suffix,
            ),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId, WorkspaceId};
    use arpagona_holographic_memory::{
        HolographicTrace, InMemoryHolographicMemoryStore, SourceKind,
    };

    fn make_request() -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "cognitive architecture memory retrieval test",
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

    fn add_trace(
        store: &mut InMemoryHolographicMemoryStore,
        id: &str,
        project_id: &str,
        content: &str,
        keywords: Vec<String>,
    ) {
        let trace = HolographicTrace::new(
            id.to_owned(),
            project_id.to_owned(),
            SourceKind::MemoryCandidate,
            "test-source".to_owned(),
            vec![],
            content.to_owned(),
            keywords,
            vec![], // concepts
            vec![], // entities
            vec![], // linked_memory_ids
            vec![], // linked_decision_ids
            0.8,    // importance
            0.9,    // confidence
            0.5,    // emotional_weight
            0.7,    // strategic_weight
            "2026-05-28T00:00:00Z".to_owned(),
        );
        store.add_trace(trace).expect("add_trace should succeed");
    }

    // ─── Supported sources ─────────────────────────────────────────────────

    #[test]
    fn adapter_returns_holographic_memory_source() {
        let store = InMemoryHolographicMemoryStore::new();
        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let sources = adapter.supported_sources();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&ContextSource::HolographicMemory));
    }

    // ─── Non-matching source ───────────────────────────────────────────────

    #[test]
    fn adapter_ignores_non_matching_sources() {
        let store = InMemoryHolographicMemoryStore::new();
        let adapter = HolographicMemoryAdapter::new(store, "test-project");
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
        let store = InMemoryHolographicMemoryStore::new();
        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let request = make_request();
        let responses = adapter.assemble(&request);

        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(resp.available);
        assert!(resp.items.is_empty());
    }

    // ─── Keywords extraction ───────────────────────────────────────────────

    #[test]
    fn extract_keywords_splits_whitespace() {
        let keywords =
            HolographicMemoryAdapter::extract_keywords("cognitive architecture memory retrieval");
        assert_eq!(keywords.len(), 4);
        assert!(keywords.contains(&"cognitive".to_owned()));
        assert!(keywords.contains(&"architecture".to_owned()));
        assert!(keywords.contains(&"memory".to_owned()));
        assert!(keywords.contains(&"retrieval".to_owned()));
    }

    #[test]
    fn extract_keywords_filters_short_tokens() {
        let keywords = HolographicMemoryAdapter::extract_keywords("a test of the system");
        // "a", "of" are 1-2 chars and should be filtered
        assert_eq!(keywords.len(), 3);
        assert!(keywords.contains(&"test".to_owned()));
        assert!(keywords.contains(&"the".to_owned()));
        assert!(keywords.contains(&"system".to_owned()));
    }

    #[test]
    fn extract_keywords_strips_punctuation() {
        let keywords = HolographicMemoryAdapter::extract_keywords("hello, world! test:");
        assert!(keywords.contains(&"hello".to_owned()));
        assert!(keywords.contains(&"world".to_owned()));
        assert!(keywords.contains(&"test".to_owned()));
    }

    #[test]
    fn extract_keywords_handles_empty_text() {
        let keywords = HolographicMemoryAdapter::extract_keywords("");
        assert!(keywords.is_empty());
    }

    #[test]
    fn extract_keywords_lowercases() {
        let keywords = HolographicMemoryAdapter::extract_keywords("Hello World");
        assert!(keywords.contains(&"hello".to_owned()));
        assert!(keywords.contains(&"world".to_owned()));
    }

    // ─── Resonance retrieval ───────────────────────────────────────────────

    #[test]
    fn adapter_returns_matching_traces() {
        let mut store = InMemoryHolographicMemoryStore::new();
        add_trace(
            &mut store,
            "trace-1",
            "test-project",
            "Discussion about cognitive architecture",
            vec!["cognitive".to_owned(), "architecture".to_owned()],
        );
        add_trace(
            &mut store,
            "trace-2",
            "test-project",
            "Unrelated lunch planning",
            vec!["lunch".to_owned(), "planning".to_owned()],
        );

        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let request = make_request_with_text("cognitive architecture");

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(resp.available);
        assert!(!resp.items.is_empty(), "Should find matching traces");
        assert!(
            resp.explanation.contains("test-project"),
            "Explanation should reference the project: {}",
            resp.explanation
        );
    }

    #[test]
    fn adapter_does_not_return_other_project_traces() {
        let mut store = InMemoryHolographicMemoryStore::new();
        add_trace(
            &mut store,
            "trace-1",
            "other-project",
            "Discussion about cognitive architecture",
            vec!["cognitive".to_owned(), "architecture".to_owned()],
        );

        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let request = make_request_with_text("cognitive architecture");

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(resp.available);
        // Traces from other projects should not appear
        assert!(resp.items.is_empty());
    }

    // ─── Max items limit ───────────────────────────────────────────────────

    #[test]
    fn adapter_respects_max_items() {
        let mut store = InMemoryHolographicMemoryStore::new();
        for i in 0..5 {
            add_trace(
                &mut store,
                &format!("trace-{}", i),
                "test-project",
                &format!("Cognitive memory trace number {}", i),
                vec!["cognitive".to_owned(), "memory".to_owned()],
            );
        }

        let adapter = HolographicMemoryAdapter::new(store, "test-project").with_max_items(3);
        let request = make_request_with_text("cognitive memory");

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(
            resp.items.len() <= 3,
            "Should have at most 3 items, got {}",
            resp.items.len()
        );
    }

    // ─── ContextItem format ────────────────────────────────────────────────

    #[test]
    fn adapter_context_items_contain_resonance_info() {
        let mut store = InMemoryHolographicMemoryStore::new();
        add_trace(
            &mut store,
            "trace-1",
            "test-project",
            "Cognitive architecture design",
            vec!["cognitive".to_owned(), "architecture".to_owned()],
        );

        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let request = make_request_with_text("cognitive architecture");

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(!resp.items.is_empty());
        let item = &resp.items[0];
        assert!(item.key.starts_with("holographic_trace:"));
        assert!(item.value.contains("resonance"));
        assert!(item.value.contains("Cognitive architecture"));
        assert_eq!(item.source, "holographic_memory_adapter");
    }

    // ─── Completion without panic ──────────────────────────────────────────

    #[test]
    fn adapter_completes_without_panic() {
        let store = InMemoryHolographicMemoryStore::new();
        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let request = make_request_with_text("some objective text for holographic retrieval");

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(!resp.explanation.is_empty());
    }

    // ─── Compute-route awareness tests ─────────────────────────────────

    #[test]
    fn holographic_adapter_local_route_reduces_traces() {
        let mut store = InMemoryHolographicMemoryStore::new();
        for i in 0..5 {
            add_trace(
                &mut store,
                &format!("trace-{}", i),
                "test-project",
                &format!("Cognitive trace number {}", i),
                vec!["cognitive".to_owned(), "memory".to_owned()],
            );
        }

        let adapter = HolographicMemoryAdapter::new(store, "test-project").with_max_items(5);
        // Local route: should return fewer items (reduced)
        let request = make_request_with_text("cognitive memory")
            .with_compute_route(Some("local-small"), Some(true));

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        // Local route reduces ~half — should have 3 or fewer items
        assert!(resp.items.len() <= 3, "Local route should limit items, got {}", resp.items.len());
        assert!(
            resp.explanation.contains("local"),
            "Explanation should mention route: {}",
            resp.explanation
        );
    }

    #[test]
    fn holographic_adapter_cloud_route_full_items() {
        let mut store = InMemoryHolographicMemoryStore::new();
        for i in 0..3 {
            add_trace(
                &mut store,
                &format!("trace-{}", i),
                "test-project",
                &format!("Cognitive trace number {}", i),
                vec!["cognitive".to_owned(), "memory".to_owned()],
            );
        }

        let adapter = HolographicMemoryAdapter::new(store, "test-project").with_max_items(5);
        let request = make_request_with_text("cognitive memory")
            .with_compute_route(Some("cloud-strong"), Some(false));

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(
            resp.explanation.contains("compute: cloud-strong"),
            "Explanation should mention cloud route: {}",
            resp.explanation
        );
    }

    #[test]
    fn holographic_adapter_default_route_has_no_compute_prefix() {
        let store = InMemoryHolographicMemoryStore::new();
        let adapter = HolographicMemoryAdapter::new(store, "test-project");
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let hm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory);
        assert!(hm_resp.is_some());
        let resp = hm_resp.unwrap();

        assert!(
            !resp.explanation.contains("[compute:"),
            "No compute prefix expected: {}",
            resp.explanation
        );
    }
}
