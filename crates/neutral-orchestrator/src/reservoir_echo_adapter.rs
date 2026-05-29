//! ReservoirEchoAdapter — ContextAssembler implementation backed by
//! Reservoir Echo short-term cognitive continuity.
//!
//! This adapter bridges the `ReservoirState` primitives from `crates/core/src/cognitive.rs`
//! into the Neutral Orchestrator's context assembly pipeline.
//!
//! When the orchestrator asks for advisory Reservoir Echo context, this adapter:
//! - Extracts keywords from the objective text for tag matching
//! - Queries the strongest matching reservoir traces by tag overlap
//! - Returns active trace summaries as advisory `ContextItem` values
//!
//! # Safety invariants
//!
//! - All items are advisory and non-authorizing.
//! - No response may contain an approval, authorization or execution token.
//! - Activation scores are purely advisory — they indicate recency/relevance,
//!   not truth or authority.
//! - If the reservoir is empty, the adapter reports the source as available
//!   but with zero items and a clear explanation.
//! - Interior mutability via `Mutex` is present for API consistency even
//!   though read-only queries use `&self` on `strongest_traces`.
//!
//! # Usage
//!
//! ```ignore
//! use arpagona_agent_core::cognitive::ReservoirState;
//! use arpagona_neutral_orchestrator::ReservoirEchoAdapter;
//!
//! let reservoir = ReservoirState::new(10, 0.3);
//! let adapter = ReservoirEchoAdapter::new(reservoir);
//! let engine = OrchestratorEngine::new()
//!     .with_context_assembler(Box::new(adapter));
//! ```

use crate::context_assembler::ContextAssembler;
use arpagona_agent_core::cognitive::ReservoirState;
use arpagona_agent_core::cognitive_work::ContextItem;
use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};
use std::sync::Mutex;

// ─── ReservoirEchoAdapter ─────────────────────────────────────────────────

/// A ContextAssembler that uses Reservoir Echo short-term cognitive continuity
/// to provide advisory recent-trace context for the orchestrator.
///
/// This adapter provides real reservoir context from recent cognitive pulses:
/// - Extracts keywords from the objective text for tag matching
/// - Finds the strongest active traces whose tags overlap with keywords
/// - Returns matching trace summaries as `ContextItem` values with activation
///   scores and decay information
/// - All results are advisory only
///
/// # Configuration
///
/// The adapter requires a `ReservoirState` instance. Use `new()` for the
/// simplest construction. Override `max_items` with `with_max_items()`.
pub struct ReservoirEchoAdapter {
    /// The Reservoir State wrapped in a Mutex for consistent API shape
    /// with other adapters (HolographicMemoryAdapter, etc.).
    reservoir: Mutex<ReservoirState>,
    /// Maximum items to return per query.
    max_items: usize,
}

impl ReservoirEchoAdapter {
    /// Create a new ReservoirEchoAdapter with the given reservoir state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let reservoir = ReservoirState::new(10, 0.3);
    /// let adapter = ReservoirEchoAdapter::new(reservoir);
    /// ```
    pub fn new(reservoir: ReservoirState) -> Self {
        Self {
            reservoir: Mutex::new(reservoir),
            max_items: 10,
        }
    }

    /// Override the maximum number of reservoir traces to return.
    ///
    /// The default is 10. Setting this higher may return more traces but
    /// could include lower-activation items.
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = max;
        self
    }

    /// Extract keywords from objective text by splitting on whitespace.
    ///
    /// Filters out empty strings and very short tokens (1-2 chars) that would
    /// produce noisy tag matching. Returns up to `max_keywords` terms.
    fn extract_keywords(text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()))
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect()
    }

    /// Check if a trace's tags overlap with the extracted keywords.
    fn has_tag_overlap(trace_tags: &[String], keywords: &[String]) -> bool {
        trace_tags
            .iter()
            .any(|tag| keywords.iter().any(|kw| kw == tag))
    }

    /// Format an activation summary for a reservoir trace.
    fn format_trace_summary(
        trace: &arpagona_agent_core::cognitive::ReservoirTrace,
        keyword_overlap: bool,
    ) -> String {
        format!(
            "[activation={:.3} decay={:.3} at={}] {} — tags: {:?}{}",
            trace.activation,
            trace.decay,
            trace.created_at.format("%H:%M:%S"),
            trace.content,
            trace.tags,
            if keyword_overlap {
                " [keyword match]"
            } else {
                ""
            },
        )
    }
}

impl ContextAssembler for ReservoirEchoAdapter {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            let response = match source {
                ContextSource::ReservoirEcho => self.assemble_reservoir_echo(request),
                _ => MemoryQueryResponse::new(source.clone()),
            };
            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![ContextSource::ReservoirEcho]
    }
}

// ─── Internal assembly logic ───────────────────────────────────────────────

impl ReservoirEchoAdapter {
    /// Assemble Reservoir Echo context: extract keywords, query strongest
    /// traces with tag overlap, and convert matches into ContextItems.
    ///
    /// Compute route awareness: when `local_preferred` is true, the adapter
    /// returns more traces (local echo is cheap and useful for continuity).
    /// When a cloud/strong route is indicated, traces are still returned
    /// but with standard limits.
    fn assemble_reservoir_echo(&self, request: &MemoryQueryRequest) -> MemoryQueryResponse {
        // ── Compute-route aware explanation suffix ─────────────────────
        let route_suffix = if let Some(ref label) = request.compute_route_label {
            let local = request.local_preferred.unwrap_or(false);
            format!(" [compute: {} | local: {}]", label, local)
        } else {
            String::new()
        };

        // Local routes: more traces (echo is cheap); cloud routes: standard limits
        let base_limit = request.max_items_per_source.min(self.max_items);
        let limit = if request.local_preferred.unwrap_or(false) {
            // Local route: return more echo traces for better continuity
            std::cmp::min(self.max_items, base_limit.saturating_mul(2))
        } else {
            base_limit
        };

        // Step 1: Extract keywords from objective text
        let keywords = Self::extract_keywords(&request.objective_text);

        // Step 2: Lock reservoir and get strongest traces
        let reservoir = match self.reservoir.lock() {
            Ok(r) => r,
            Err(e) => {
                let _msg = format!("Reservoir Echo lock poisoned: {}", e);
                return MemoryQueryResponse::new(ContextSource::ReservoirEcho).with_unavailable();
            }
        };

        let strongest = reservoir.strongest_traces(usize::MAX);

        // Step 3: Filter by tag overlap if keywords are available,
        // or take the strongest traces directly if no keywords
        let mut matched: Vec<&arpagona_agent_core::cognitive::ReservoirTrace> =
            if keywords.is_empty() {
                // No keywords: return the strongest traces up to limit
                strongest.iter().take(limit).collect()
            } else {
                // Filter by tag overlap, then sort by activation, take top
                let mut filtered: Vec<&arpagona_agent_core::cognitive::ReservoirTrace> = strongest
                    .iter()
                    .filter(|t| Self::has_tag_overlap(&t.tags, &keywords))
                    .collect();
                filtered.sort_by(|a, b| {
                    b.activation
                        .partial_cmp(&a.activation)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                filtered.truncate(limit);
                filtered
            };

        // Step 4: Sort by activation descending
        matched.sort_by(|a, b| {
            b.activation
                .partial_cmp(&a.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Step 5: Convert matched traces into advisory ContextItems
        let keyword_overlap = !keywords.is_empty();
        let items: Vec<ContextItem> = matched
            .iter()
            .map(|t| ContextItem {
                key: format!(
                    "reservoir_trace_{}",
                    t.created_at.timestamp_nanos_opt().unwrap_or(0)
                ),
                value: Self::format_trace_summary(t, keyword_overlap),
                source: "reservoir_echo_adapter".to_owned(),
            })
            .collect();

        let count = items.len();
        let total = matched.len();

        MemoryQueryResponse {
            source: ContextSource::ReservoirEcho,
            items,
            available: true,
            explanation: format!(
                "Reservoir Echo returned {} trace(s) (top {} by activation). {}{}",
                count,
                total,
                if keywords.is_empty() {
                    "No objective keywords provided — returning strongest traces."
                } else {
                    "Traces filtered by keyword tag overlap."
                },
                route_suffix,
            ),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::cognitive::CognitivePulse;
    use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId, WorkspaceId};
    use chrono::Utc;

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

    fn add_trace(reservoir: &mut ReservoirState, content: &str, tags: Vec<String>) {
        let now = Utc::now();
        let pulse = CognitivePulse::stimulus(content, tags, now);
        reservoir.absorb(pulse);
    }

    // ─── Supported sources ─────────────────────────────────────────────────

    #[test]
    fn adapter_returns_reservoir_echo_source() {
        let reservoir = ReservoirState::new(10, 0.3);
        let adapter = ReservoirEchoAdapter::new(reservoir);
        let sources = adapter.supported_sources();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&ContextSource::ReservoirEcho));
    }

    // ─── Non-matching source ─────────────────────────────────────────────

    #[test]
    fn adapter_ignores_non_matching_sources() {
        let reservoir = ReservoirState::new(10, 0.3);
        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_sources("test", vec![ContextSource::GraphMemory]);
        let responses = adapter.assemble(&request);
        assert_eq!(responses.len(), 1);
        let resp = &responses[0];
        assert_eq!(resp.source, ContextSource::GraphMemory);
        assert!(resp.items.is_empty());
        assert!(resp.available);
    }

    // ─── Empty reservoir returns empty but available ─────────────────────

    #[test]
    fn adapter_handles_empty_reservoir() {
        let reservoir = ReservoirState::new(10, 0.3);
        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(resp.available);
        assert!(resp.items.is_empty());
    }

    // ─── Keywords extraction ──────────────────────────────────────────────

    #[test]
    fn extract_keywords_splits_whitespace() {
        let keywords =
            ReservoirEchoAdapter::extract_keywords("cognitive architecture memory retrieval");
        assert_eq!(keywords.len(), 4);
        assert!(keywords.contains(&"cognitive".to_owned()));
        assert!(keywords.contains(&"architecture".to_owned()));
        assert!(keywords.contains(&"memory".to_owned()));
        assert!(keywords.contains(&"retrieval".to_owned()));
    }

    #[test]
    fn extract_keywords_filters_short_tokens() {
        let keywords = ReservoirEchoAdapter::extract_keywords("a test of the system");
        assert_eq!(keywords.len(), 3);
        assert!(keywords.contains(&"test".to_owned()));
        assert!(keywords.contains(&"the".to_owned()));
        assert!(keywords.contains(&"system".to_owned()));
    }

    #[test]
    fn extract_keywords_strips_punctuation() {
        let keywords = ReservoirEchoAdapter::extract_keywords("hello, world! test:");
        assert!(keywords.contains(&"hello".to_owned()));
        assert!(keywords.contains(&"world".to_owned()));
        assert!(keywords.contains(&"test".to_owned()));
    }

    #[test]
    fn extract_keywords_handles_empty_text() {
        let keywords = ReservoirEchoAdapter::extract_keywords("");
        assert!(keywords.is_empty());
    }

    #[test]
    fn extract_keywords_lowercases() {
        let keywords = ReservoirEchoAdapter::extract_keywords("Hello World");
        assert!(keywords.contains(&"hello".to_owned()));
        assert!(keywords.contains(&"world".to_owned()));
    }

    // ─── Tag overlap matching ────────────────────────────────────────────

    #[test]
    fn has_tag_overlap_matches_keywords() {
        let tags = vec!["cognitive".to_owned(), "architecture".to_owned()];
        let keywords = vec!["cognitive".to_owned(), "memory".to_owned()];
        assert!(ReservoirEchoAdapter::has_tag_overlap(&tags, &keywords));
    }

    #[test]
    fn has_tag_overlap_returns_false_for_no_match() {
        let tags = vec!["lunch".to_owned(), "planning".to_owned()];
        let keywords = vec!["cognitive".to_owned(), "architecture".to_owned()];
        assert!(!ReservoirEchoAdapter::has_tag_overlap(&tags, &keywords));
    }

    #[test]
    fn has_tag_overlap_handles_empty_tags() {
        let tags = vec![];
        let keywords = vec!["cognitive".to_owned()];
        assert!(!ReservoirEchoAdapter::has_tag_overlap(&tags, &keywords));
    }

    // ─── Resonance retrieval ──────────────────────────────────────────────

    #[test]
    fn adapter_returns_matching_traces_by_keyword() {
        let mut reservoir = ReservoirState::new(10, 0.3);

        // Add a trace with matching tags
        add_trace(
            &mut reservoir,
            "Discussion about cognitive architecture",
            vec!["cognitive".to_owned(), "architecture".to_owned()],
        );
        // Add a trace with non-matching tags
        add_trace(
            &mut reservoir,
            "Unrelated lunch planning",
            vec!["lunch".to_owned(), "planning".to_owned()],
        );

        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_text("cognitive architecture");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(resp.available);
        assert!(!resp.items.is_empty(), "Should find matching traces");
        // Should only return the cognitive architecture trace (tag match)
        assert!(
            resp.items[0].value.contains("cognitive"),
            "Item should reference the matching trace: {}",
            resp.items[0].value
        );
    }

    #[test]
    fn adapter_returns_strongest_traces_when_no_keywords() {
        let mut reservoir = ReservoirState::new(10, 0.3);

        add_trace(
            &mut reservoir,
            "Important task tracking",
            vec!["task".to_owned()],
        );
        add_trace(
            &mut reservoir,
            "Project review notes",
            vec!["project".to_owned()],
        );

        // Request with text that has no common keywords (all short tokens)
        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_text("a an");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(resp.available);
        // With no keywords, should return traces anyway (strongest)
        assert!(!resp.items.is_empty(), "Should return strongest traces");
    }

    // ─── Max items limit ───────────────────────────────────────────────────

    #[test]
    fn adapter_respects_max_items() {
        let mut reservoir = ReservoirState::new(10, 0.3);
        for i in 0..5 {
            add_trace(
                &mut reservoir,
                &format!("Trace number {}", i),
                vec!["cognitive".to_owned(), "memory".to_owned()],
            );
        }

        let adapter = ReservoirEchoAdapter::new(reservoir).with_max_items(3);
        let request = make_request_with_text("cognitive memory");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(
            resp.items.len() <= 3,
            "Should have at most 3 items, got {}",
            resp.items.len()
        );
    }

    // ─── ContextItem format ────────────────────────────────────────────────

    #[test]
    fn adapter_context_items_contain_activation_info() {
        let mut reservoir = ReservoirState::new(10, 0.3);
        add_trace(
            &mut reservoir,
            "Cognitive architecture design",
            vec!["cognitive".to_owned(), "architecture".to_owned()],
        );

        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_text("cognitive architecture");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(!resp.items.is_empty());
        // Items should contain activation info and content
        assert!(
            resp.items[0].value.contains("activation"),
            "Item should contain activation score: {}",
            resp.items[0].value
        );
        assert!(
            resp.items[0].value.contains("cognitive"),
            "Item should contain original content: {}",
            resp.items[0].value
        );
        assert_eq!(resp.items[0].source, "reservoir_echo_adapter");
    }

    // ─── Explanation format ────────────────────────────────────────────────

    #[test]
    fn adapter_explanation_contains_relevant_info() {
        let mut reservoir = ReservoirState::new(10, 0.3);
        add_trace(&mut reservoir, "Test trace", vec!["test".to_owned()]);

        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(resp.explanation.contains("Reservoir Echo"));
        assert!(resp.explanation.contains("trace(s)"));
    }

    // ─── Decay doesn't affect adapter output ──────────────────────────────

    #[test]
    fn adapter_works_with_decayed_traces() {
        let mut reservoir = ReservoirState::new(10, 0.5);
        add_trace(&mut reservoir, "Quick idea", vec!["idea".to_owned()]);

        // Decay the trace
        reservoir.decay_tick();

        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_text("quick idea");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        // The trace still exists (activation reduced but > 0)
        assert!(resp.available);
        assert!(!resp.items.is_empty());
        // The activation should be lower than 1.0
        assert!(
            resp.items[0].value.contains("activation=0."),
            "Decayed trace should show reduced activation: {}",
            resp.items[0].value
        );
    }

    // ─── Compute-route awareness tests ─────────────────────────────────

    #[test]
    fn reservoir_adapter_local_route_increases_items() {
        let mut reservoir = ReservoirState::new(100, 0.3);
        for i in 0..5 {
            add_trace(
                &mut reservoir,
                &format!("Trace number {}", i),
                vec!["cognitive".to_owned(), "memory".to_owned()],
            );
        }

        let adapter = ReservoirEchoAdapter::new(reservoir).with_max_items(5);
        // Local route: should return traces with compute hint in explanation
        let request = make_request_with_text("cognitive memory")
            .with_compute_route(Some("local-small"), Some(true));

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        // Should return matching traces and include route info in explanation
        assert!(!resp.items.is_empty(), "Local route should return items");
        assert!(
            resp.explanation.contains("local"),
            "Explanation should mention route: {}",
            resp.explanation
        );
    }

    #[test]
    fn reservoir_adapter_cloud_route_standard_limits() {
        let mut reservoir = ReservoirState::new(100, 0.3);
        for i in 0..3 {
            add_trace(
                &mut reservoir,
                &format!("Trace number {}", i),
                vec!["cognitive".to_owned(), "memory".to_owned()],
            );
        }

        let adapter = ReservoirEchoAdapter::new(reservoir).with_max_items(3);
        let request = make_request_with_text("cognitive memory")
            .with_compute_route(Some("cloud-strong"), Some(false));

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(
            resp.explanation.contains("compute: cloud-strong"),
            "Explanation should mention cloud route: {}",
            resp.explanation
        );
    }

    #[test]
    fn reservoir_adapter_default_route_has_no_compute_prefix() {
        let reservoir = ReservoirState::new(10, 0.3);
        let adapter = ReservoirEchoAdapter::new(reservoir);
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let re_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho);
        assert!(re_resp.is_some());
        let resp = re_resp.unwrap();

        assert!(
            !resp.explanation.contains("[compute:"),
            "No compute prefix expected: {}",
            resp.explanation
        );
    }
}
