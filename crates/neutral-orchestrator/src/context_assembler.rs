//! ContextAssembler trait and implementations.
//!
//! The ContextAssembler is the pluggable component that gathers advisory context
//! from memory sources (Graph Memory, Holographic Memory, Reservoir Echo,
//! Compressed Cognitive Attention, Tool Runtime, Working Memory) for the
//! orchestrator cycle.
//!
//! Every item returned is advisory and non-authorizing. No context in any
//! response may approve, authorize or execute an action.
//!
//! Current implementations:
//! - `SimulatedContextAssembler` — no-op, returns empty results for all sources.
//!   This is the default and requires no I/O, no LLM, no persistence.
//!
//! Future implementations (not yet built):
//! - `GraphMemoryAdapter` — queries `crates/graph-memory`
//! - `HolographicMemoryAdapter` — queries `crates/holographic-memory`
//! - `ReservoirEchoAdapter` — queries reservoir state
//! - `CompressedCognitiveAttentionAdapter` — queries `crates/compressed-cognitive-attention`
//! - `ToolRuntimeAdapter` — queries `crates/tool-runtime`

use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};

// ─── ContextAssembler trait ────────────────────────────────────────────────

/// Pluggable component that gathers advisory context from memory sources.
///
/// Implementations may query real adapters (GraphMemoryAdapter, etc.) or
/// return synthetic/no-op results for testing and simulation.
///
/// # Safety invariants
///
/// - Every item returned is advisory and non-authorizing.
/// - No response may contain an approval, authorization or execution token.
/// - Empty/unavailable sources must report availability honestly.
///
/// # Implementations
///
/// | Implementation | Use | Dependencies |
/// |---|---|---|
/// | `SimulatedContextAssembler` | Default, tests, simulation | None |
/// | `GraphMemoryAdapter` (future) | Real Graph Memory queries | `crates/graph-memory` |
/// | `HolographicMemoryAdapter` (future) | Real Holographic Memory queries | `crates/holographic-memory` |
/// | `ReservoirEchoAdapter` (future) | Real reservoir queries | reservoir primitives |
/// | `CompressedCognitiveAttentionAdapter` (future) | Real CCA queries | `crates/compressed-cognitive-attention` |
/// | `ToolRuntimeAdapter` (future) | Real workspace perception | `crates/tool-runtime` |
pub trait ContextAssembler {
    /// Assemble advisory context for the given objective.
    ///
    /// Returns one `MemoryQueryResponse` per requested source. Each response
    /// contains:
    /// - `source`: which adapter produced this response
    /// - `items`: advisory context items (may be empty)
    /// - `available`: whether the source was reachable
    /// - `explanation`: human-readable description of the query result
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse>;

    /// Return the list of sources this assembler can query.
    fn supported_sources(&self) -> Vec<ContextSource>;
}

// ─── SimulatedContextAssembler ─────────────────────────────────────────────

/// Deterministic, no-op ContextAssembler that returns empty responses for all
/// sources.
///
/// This is the default assembler used by `OrchestratorEngine::new()`.
/// It requires no I/O, no LLM calls, no persistence, and no dependencies.
///
/// # Simulated behavior
///
/// - All sources are reported as available (but return zero items).
/// - The response explanations state "Simulated: {...} has no items."
/// - No database, no filesystem, no network access.
///
/// This allows the orchestrator to compile and run without any real memory
/// adapter installed. Real adapters can be plugged in progressively.
#[derive(Clone, Debug)]
pub struct SimulatedContextAssembler;

impl SimulatedContextAssembler {
    /// Create a new SimulatedContextAssembler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedContextAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAssembler for SimulatedContextAssembler {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            let response = MemoryQueryResponse {
                source: source.clone(),
                items: vec![],
                available: true,
                explanation: format!(
                    "Simulated: {:?} has no items (no adapter installed).",
                    source
                ),
            };
            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![
            ContextSource::GraphMemory,
            ContextSource::HolographicMemory,
            ContextSource::ReservoirEcho,
            ContextSource::ToolRuntime,
            ContextSource::WorkingMemory,
            ContextSource::CompressedCognitiveAttention,
        ]
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId, WorkspaceId};

    fn make_request() -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Test objective text",
            WorkspaceId::new("ws-test"),
        )
    }

    #[test]
    fn simulated_assembler_returns_empty_responses() {
        let assembler = SimulatedContextAssembler::new();
        let request = make_request();
        let responses = assembler.assemble(&request);

        // Should return one response per requested source
        assert_eq!(responses.len(), request.requested_sources.len());

        // Every response should have zero items
        for response in &responses {
            assert!(response.items.is_empty());
            assert!(response.available);
        }
    }

    #[test]
    fn simulated_assembler_reports_all_sources_available() {
        let assembler = SimulatedContextAssembler::new();
        let sources = assembler.supported_sources();

        assert!(sources.contains(&ContextSource::GraphMemory));
        assert!(sources.contains(&ContextSource::HolographicMemory));
        assert!(sources.contains(&ContextSource::ReservoirEcho));
        assert!(sources.contains(&ContextSource::ToolRuntime));
        assert!(sources.contains(&ContextSource::WorkingMemory));
        assert!(sources.contains(&ContextSource::CompressedCognitiveAttention));
        assert_eq!(sources.len(), 6);
    }

    #[test]
    fn simulated_assembler_responses_have_explanations() {
        let assembler = SimulatedContextAssembler::new();
        let request = make_request();
        let responses = assembler.assemble(&request);

        for response in &responses {
            assert!(!response.explanation.is_empty());
            assert!(response.explanation.contains("Simulated"));
            assert!(response.explanation.contains("no adapter"));
        }
    }

    #[test]
    fn simulated_assembler_respects_source_filter() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Test",
            WorkspaceId::new("ws-test"),
        )
        .with_sources(vec![
            ContextSource::GraphMemory,
            ContextSource::HolographicMemory,
        ]);

        let responses = assembler.assemble(&request);

        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].source, ContextSource::GraphMemory);
        assert_eq!(responses[1].source, ContextSource::HolographicMemory);
    }

    #[test]
    fn simulated_assembler_adheres_to_max_items() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Test with max 5 items per source",
            WorkspaceId::new("ws-test"),
        )
        .with_max_items(5);

        let responses = assembler.assemble(&request);

        // Simulated always returns 0 items regardless of max
        for response in &responses {
            assert_eq!(response.items.len(), 0);
        }
    }
}
