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

        // Build a compute-aware prefix for explanations when route info is available
        let compute_prefix = if let Some(ref route_label) = request.compute_route_label {
            let local = request.local_preferred.unwrap_or(false);
            format!(" [compute: {} | local: {}]", route_label, local)
        } else {
            String::new()
        };

        // Build an efficiency feedback prefix when previous-cycle signals exist
        let eff_prefix = if !request.efficiency_feedback.is_empty() {
            let labels: Vec<&str> = request
                .efficiency_feedback
                .iter()
                .map(|s| s.context_label())
                .collect();
            format!(" [efficiency: {}]", labels.join(", "))
        } else {
            String::new()
        };

        for source in &request.requested_sources {
            let response = MemoryQueryResponse {
                source: source.clone(),
                items: vec![],
                available: true,
                explanation: format!(
                    "Simulated: {:?} has no items (no adapter installed).{}{}",
                    source, compute_prefix, eff_prefix
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

    // ─── Compute-aware context assembly tests ────────────────────────────

    #[test]
    fn simulated_assembler_includes_compute_route_in_explanation() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Summarize governance docs",
            WorkspaceId::new("ws-test"),
        )
        .with_compute_route(Some("local-small (llm, $0, 800ms)"), Some(true));

        let responses = assembler.assemble(&request);

        // Every response should include the compute route info in the explanation
        for response in &responses {
            assert!(
                response.explanation.contains("[compute: local-small"),
                "Expected compute route in explanation, got: {}",
                response.explanation
            );
            assert!(
                response.explanation.contains("local: true"),
                "Expected local:true in explanation, got: {}",
                response.explanation
            );
        }
    }

    #[test]
    fn simulated_assembler_without_compute_route_has_no_prefix() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Simple task without compute hint",
            WorkspaceId::new("ws-test"),
        );

        let responses = assembler.assemble(&request);

        // Without compute route, explanations should not contain compute prefix
        for response in &responses {
            assert!(
                !response.explanation.contains("[compute:"),
                "Expected no compute prefix in explanation, got: {}",
                response.explanation
            );
        }
    }

    #[test]
    fn simulated_assembler_with_cloud_route_reflects_routing() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Complex financial analysis",
            WorkspaceId::new("ws-test"),
        )
        .with_compute_route(Some("cloud-strong (cloudllm, $50, 2000ms)"), Some(false));

        let responses = assembler.assemble(&request);

        for response in &responses {
            assert!(
                response.explanation.contains("cloud-strong"),
                "Expected cloud route in explanation, got: {}",
                response.explanation
            );
            assert!(
                response.explanation.contains("local: false"),
                "Expected local:false in explanation, got: {}",
                response.explanation
            );
        }
    }

    #[test]
    fn request_with_compute_route_stores_fields() {
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Test compute route storage",
            WorkspaceId::new("ws-test"),
        )
        .with_compute_route(Some("local-small"), Some(true));

        assert_eq!(request.compute_route_label, Some("local-small".to_owned()));
        assert_eq!(request.local_preferred, Some(true));
    }

    #[test]
    fn request_without_compute_route_has_none() {
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Test default compute route",
            WorkspaceId::new("ws-test"),
        );

        assert!(request.compute_route_label.is_none());
        assert!(request.local_preferred.is_none());
    }

    // ─── Efficiency feedback tests ─────────────────────────────────────

    #[test]
    fn simulated_assembler_includes_efficiency_feedback_in_explanation() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-eff-test"),
            ObjectiveId::new("obj-eff"),
            "Task with efficiency feedback",
            WorkspaceId::new("ws-eff"),
        )
        .with_efficiency_feedback(vec![
            arpagona_agent_core::orchestrator::EfficiencySignal::FallbackRouting,
        ]);

        let responses = assembler.assemble(&request);

        for response in &responses {
            assert!(
                response.explanation.contains("[efficiency: eff:fallback]"),
                "Expected efficiency prefix in explanation, got: {}",
                response.explanation
            );
        }
    }

    #[test]
    fn simulated_assembler_with_multiple_efficiency_signals() {
        let assembler = SimulatedContextAssembler::new();
        use arpagona_agent_core::orchestrator::EfficiencySignal;
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-multi-eff"),
            ObjectiveId::new("obj-multi-eff"),
            "Multiple signals",
            WorkspaceId::new("ws-multi-eff"),
        )
        .with_efficiency_feedback(vec![
            EfficiencySignal::FallbackRouting,
            EfficiencySignal::MissingComputeRoute,
        ]);

        let responses = assembler.assemble(&request);

        for response in &responses {
            assert!(
                response.explanation.contains("eff:fallback"),
                "Missing eff:fallback in: {}",
                response.explanation
            );
            assert!(
                response.explanation.contains("eff:no-route"),
                "Missing eff:no-route in: {}",
                response.explanation
            );
        }
    }

    #[test]
    fn simulated_assembler_without_efficiency_has_no_prefix() {
        let assembler = SimulatedContextAssembler::new();
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-no-eff"),
            ObjectiveId::new("obj-no-eff"),
            "No efficiency feedback",
            WorkspaceId::new("ws-no-eff"),
        );

        let responses = assembler.assemble(&request);

        for response in &responses {
            assert!(
                !response.explanation.contains("[efficiency:"),
                "Expected no efficiency prefix, got: {}",
                response.explanation
            );
        }
    }

    #[test]
    fn simulated_assembler_with_compute_and_efficiency_shows_both() {
        let assembler = SimulatedContextAssembler::new();
        use arpagona_agent_core::orchestrator::EfficiencySignal;
        let request = MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-combined"),
            ObjectiveId::new("obj-combined"),
            "Combined signals",
            WorkspaceId::new("ws-combined"),
        )
        .with_compute_route(Some("local-small"), Some(true))
        .with_efficiency_feedback(vec![
            EfficiencySignal::FallbackRouting,
            EfficiencySignal::IneffectiveComputeOnFailedCycle,
        ]);

        let responses = assembler.assemble(&request);

        for response in &responses {
            assert!(
                response.explanation.contains("[compute: local-small"),
                "Expected compute prefix, got: {}",
                response.explanation
            );
            assert!(
                response.explanation.contains("[efficiency:"),
                "Expected efficiency prefix, got: {}",
                response.explanation
            );
            assert!(
                response.explanation.contains("eff:failed-cycle"),
                "Missing eff:failed-cycle in: {}",
                response.explanation
            );
        }
    }
}
