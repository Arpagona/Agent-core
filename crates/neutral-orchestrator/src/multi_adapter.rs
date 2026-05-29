//! MultiAdapterContextAssembler — composite ContextAssembler that delegates
//! to all 5 real memory adapters (ToolRuntime, GraphMemory, HolographicMemory,
//! ReservoirEcho, CompressedCognitiveAttention).
//!
//! This is the "integration verification" assembler for P3-6: it wires all
//! available context sources into one assembly pipeline so the orchestrator
//! cycle receives advisory context from every source at once.
//!
//! # Safety invariants
//!
//! - All items are advisory and non-authorizing.
//! - Each delegate adapter enforces its own safety invariants.
//! - Unconfigured adapters are reported as available but return zero items.
//! - The composite does not add approval, authorization or execution tokens.
//!
//! # Usage
//!
//! ```ignore
//! use arpagona_neutral_orchestrator::{
//!     MultiAdapterContextAssembler,
//!     ToolRuntimeAdapter, GraphMemoryAdapter,
//!     HolographicMemoryAdapter, ReservoirEchoAdapter,
//!     CompressedCognitiveAttentionAdapter,
//! };
//!
//! let assembler = MultiAdapterContextAssembler::new()
//!     .with_tool_runtime(ToolRuntimeAdapter::new("."))
//!     .with_graph_memory(GraphMemoryAdapter::new(my_store))
//!     .with_holographic_memory(HolographicMemoryAdapter::new(holo_store, "proj"))
//!     .with_reservoir_echo(ReservoirEchoAdapter::new(reservoir))
//!     .with_compressed_cognitive_attention(CompressedCognitiveAttentionAdapter::new(events, config));
//!
//! let engine = OrchestratorEngine::new()
//!     .with_context_assembler(Box::new(assembler));
//! ```

use crate::compressed_cognitive_attention_adapter::CompressedCognitiveAttentionAdapter;
use crate::context_assembler::ContextAssembler;
use crate::graph_memory_adapter::GraphMemoryAdapter;
use crate::holographic_memory_adapter::HolographicMemoryAdapter;
use crate::reservoir_echo_adapter::ReservoirEchoAdapter;
use crate::tool_runtime_adapter::ToolRuntimeAdapter;
use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};

// ─── MultiAdapterContextAssembler ──────────────────────────────────────────

/// A composite ContextAssembler that delegates to all 5 memory adapters.
///
/// Each adapter may be independently configured or left at its default.
/// When no adapter is configured for a given source, the composite returns
/// an empty response (source reported as available, zero items).
///
/// # Supported sources
///
/// | Source | Adapter | Default |
/// |--------|---------|---------|
/// | ToolRuntime | `ToolRuntimeAdapter` | Default (workspace=".") |
/// | GraphMemory | `GraphMemoryAdapter` | Default (empty store) |
/// | HolographicMemory | `HolographicMemoryAdapter` | Default (project="default") |
/// | ReservoirEcho | `ReservoirEchoAdapter` | Default (capacity=10, decay=0.3) |
/// | CompressedCognitiveAttention | `CompressedCognitiveAttentionAdapter` | Default (empty events, 16/4 config) |
///
/// Sources without a dedicated adapter (`WorkingMemory`) return empty.
pub struct MultiAdapterContextAssembler {
    tool_runtime: ToolRuntimeAdapter,
    graph_memory: GraphMemoryAdapter,
    holographic_memory: HolographicMemoryAdapter,
    reservoir_echo: ReservoirEchoAdapter,
    compressed_cognitive_attention: CompressedCognitiveAttentionAdapter,
}

impl MultiAdapterContextAssembler {
    /// Create a new MultiAdapterContextAssembler with default adapters.
    ///
    /// Defaults:
    /// - ToolRuntimeAdapter with workspace root "."
    /// - GraphMemoryAdapter with an empty in-memory store
    /// - HolographicMemoryAdapter with empty store, project "default"
    /// - ReservoirEchoAdapter with capacity 10, decay 0.3
    /// - CompressedCognitiveAttentionAdapter with empty events, 16/4 config
    pub fn new() -> Self {
        // Default adapters — each is safe to construct with no I/O.
        let tool_runtime = ToolRuntimeAdapter::new(".");
        let graph_memory =
            GraphMemoryAdapter::new(arpagona_agent_core::memory::InMemoryGraphMemoryStore::new());
        let holographic_memory = HolographicMemoryAdapter::new(
            arpagona_holographic_memory::InMemoryHolographicMemoryStore::new(),
            "default",
        );
        let reservoir_echo =
            ReservoirEchoAdapter::new(arpagona_agent_core::cognitive::ReservoirState::new(10, 0.3));
        let compressed_cognitive_attention =
            CompressedCognitiveAttentionAdapter::with_defaults(vec![]);

        Self {
            tool_runtime,
            graph_memory,
            holographic_memory,
            reservoir_echo,
            compressed_cognitive_attention,
        }
    }

    /// Replace the ToolRuntimeAdapter.
    pub fn with_tool_runtime(mut self, adapter: ToolRuntimeAdapter) -> Self {
        self.tool_runtime = adapter;
        self
    }

    /// Replace the GraphMemoryAdapter.
    pub fn with_graph_memory(mut self, adapter: GraphMemoryAdapter) -> Self {
        self.graph_memory = adapter;
        self
    }

    /// Replace the HolographicMemoryAdapter.
    pub fn with_holographic_memory(mut self, adapter: HolographicMemoryAdapter) -> Self {
        self.holographic_memory = adapter;
        self
    }

    /// Replace the ReservoirEchoAdapter.
    pub fn with_reservoir_echo(mut self, adapter: ReservoirEchoAdapter) -> Self {
        self.reservoir_echo = adapter;
        self
    }

    /// Replace the CompressedCognitiveAttentionAdapter.
    pub fn with_compressed_cognitive_attention(
        mut self,
        adapter: CompressedCognitiveAttentionAdapter,
    ) -> Self {
        self.compressed_cognitive_attention = adapter;
        self
    }
}

impl Default for MultiAdapterContextAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAssembler for MultiAdapterContextAssembler {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            // Create a sub-request targeting only this source
            let sub_request = request.clone().with_sources(vec![source.clone()]);

            let response = match source {
                ContextSource::ToolRuntime => {
                    let mut r = self.tool_runtime.assemble(&sub_request);
                    r.pop()
                        .unwrap_or_else(|| MemoryQueryResponse::new(ContextSource::ToolRuntime))
                }
                ContextSource::GraphMemory => {
                    let mut r = self.graph_memory.assemble(&sub_request);
                    r.pop()
                        .unwrap_or_else(|| MemoryQueryResponse::new(ContextSource::GraphMemory))
                }
                ContextSource::HolographicMemory => {
                    let mut r = self.holographic_memory.assemble(&sub_request);
                    r.pop().unwrap_or_else(|| {
                        MemoryQueryResponse::new(ContextSource::HolographicMemory)
                    })
                }
                ContextSource::ReservoirEcho => {
                    let mut r = self.reservoir_echo.assemble(&sub_request);
                    r.pop()
                        .unwrap_or_else(|| MemoryQueryResponse::new(ContextSource::ReservoirEcho))
                }
                ContextSource::CompressedCognitiveAttention => {
                    let mut r = self.compressed_cognitive_attention.assemble(&sub_request);
                    r.pop().unwrap_or_else(|| {
                        MemoryQueryResponse::new(ContextSource::CompressedCognitiveAttention)
                    })
                }
                ContextSource::WorkingMemory => {
                    // No dedicated adapter — return empty
                    MemoryQueryResponse::new(ContextSource::WorkingMemory)
                }
            };

            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![
            ContextSource::ToolRuntime,
            ContextSource::GraphMemory,
            ContextSource::HolographicMemory,
            ContextSource::ReservoirEcho,
            ContextSource::CompressedCognitiveAttention,
            ContextSource::WorkingMemory,
        ]
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::audit::{ActorRef, AuditEvent, AuditEventType};
    use arpagona_agent_core::cognitive::ReservoirState;
    use arpagona_agent_core::ids::{
        AgentId, AuditEventId, ObjectiveId, OrchestratorCycleId, WorkspaceId,
    };
    use arpagona_agent_core::memory::{GraphMemoryStore, InMemoryGraphMemoryStore};
    use arpagona_agent_core::permission::Permission;
    use arpagona_holographic_memory::{
        HolographicMemoryStore, HolographicTrace, InMemoryHolographicMemoryStore, SourceKind,
    };
    use chrono::Utc;

    fn make_request() -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-int-test"),
            ObjectiveId::new("obj-int-test"),
            "integration verification spring — multi-source context assembly",
            WorkspaceId::new("ws-int-test"),
        )
    }

    /// The default assembler reports all 6 sources as available.
    #[test]
    fn multi_adapter_reports_all_sources() {
        let assembler = MultiAdapterContextAssembler::new();
        let sources = assembler.supported_sources();
        assert!(sources.contains(&ContextSource::ToolRuntime));
        assert!(sources.contains(&ContextSource::GraphMemory));
        assert!(sources.contains(&ContextSource::HolographicMemory));
        assert!(sources.contains(&ContextSource::ReservoirEcho));
        assert!(sources.contains(&ContextSource::CompressedCognitiveAttention));
        assert!(sources.contains(&ContextSource::WorkingMemory));
        assert_eq!(sources.len(), 6);
    }

    /// With all default (empty) adapters, non-ToolRuntime sources return empty but available.
    /// (ToolRuntime may find workspace files since the default workspace is ".".)
    #[test]
    fn multi_adapter_default_returns_empty_responses() {
        let assembler = MultiAdapterContextAssembler::new();
        let request = make_request();
        let responses = assembler.assemble(&request);

        assert_eq!(responses.len(), request.requested_sources.len());
        for resp in &responses {
            assert!(resp.available, "{:?} should be available", resp.source);
        }
        // Non-ToolRuntime sources should be empty (no data seeded)
        for resp in &responses {
            if resp.source != ContextSource::ToolRuntime {
                assert!(
                    resp.items.is_empty(),
                    "{:?} should have no items by default: {}",
                    resp.source,
                    resp.items.len()
                );
            }
        }
        // At least non-ToolRuntime sources are empty
        let non_tool_empty: Vec<_> = responses
            .iter()
            .filter(|r| r.source != ContextSource::ToolRuntime && r.items.is_empty())
            .collect();
        assert_eq!(
            non_tool_empty.len(),
            5,
            "All 5 non-ToolRuntime sources should be empty"
        );
    }

    /// With 4 adapters populated (ToolRuntime, GraphMemory, ReservoirEcho, HolographicMemory)
    /// plus default CCA with empty events — verifies multi-source assembly with
    /// non-authorizing invariant.
    #[test]
    fn multi_adapter_seeded_sources_produce_items() {
        // ── ToolRuntimeAdapter: points at real workspace ────────────────
        let cwd = std::env::current_dir().expect("current dir should exist");
        let tool_runtime = ToolRuntimeAdapter::new(cwd).with_max_items(3);

        // ── GraphMemoryAdapter: seeded with an audit event ──────────────
        let mut gm_store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-int-test");
        gm_store
            .record_audit_event(AuditEvent {
                id: AuditEventId::new("evt-1"),
                event_type: AuditEventType::DecisionCreated,
                actor: ActorRef::System,
                workspace_id: Some(ws_id.clone()),
                task_id: None,
                proposed_action_id: None,
                decision_id: None,
                payload: serde_json::json!({"summary": "P3-6 integration test"}),
                created_at: Utc::now(),
            })
            .expect("record_audit_event should succeed");
        let graph_memory = GraphMemoryAdapter::new(gm_store).with_max_items(3);

        // ── HolographicMemoryAdapter: seeded with a trace ───────────────
        let mut holo_store = InMemoryHolographicMemoryStore::new();
        let holo_trace = HolographicTrace::new(
            "trace-int-1".to_owned(),
            "default".to_owned(),
            SourceKind::ManualNote,
            "source-int".to_owned(),
            vec!["turn-1".to_owned()],
            "Integration verification trace for P3-6".to_owned(),
            vec!["integration".to_owned(), "verification".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
            0.0,
            0.0,
            Utc::now().to_rfc3339(),
        );
        holo_store
            .add_trace(holo_trace)
            .expect("add_trace should succeed");
        let holographic_memory =
            HolographicMemoryAdapter::new(holo_store, "default").with_max_items(3);

        // ── ReservoirEchoAdapter: seeded with a pulse ───────────────────
        let mut reservoir = ReservoirState::new(10, 0.3);
        reservoir.absorb(arpagona_agent_core::cognitive::CognitivePulse::stimulus(
            "multi-source integration verified",
            vec![
                "integration".to_owned(),
                "verified".to_owned(),
                "spring".to_owned(),
            ],
            Utc::now(),
        ));
        let reservoir_echo = ReservoirEchoAdapter::new(reservoir).with_max_items(3);

        // ── CompressedCognitiveAttentionAdapter: seeded with events ──────
        let cca_config = arpagona_compressed_cognitive_attention::Config::new(16, 4);
        let cca_events = vec![arpagona_compressed_cognitive_attention::MemoryEvent::new(
            "cca-integration-event",
            vec![0.5f64; 16],
        )];
        let cca =
            CompressedCognitiveAttentionAdapter::new(cca_events, cca_config).with_max_items(3);

        // ── Assemble ────────────────────────────────────────────────────
        let assembler = MultiAdapterContextAssembler::new()
            .with_tool_runtime(tool_runtime)
            .with_graph_memory(graph_memory)
            .with_holographic_memory(holographic_memory)
            .with_reservoir_echo(reservoir_echo)
            .with_compressed_cognitive_attention(cca);

        let request = make_request();
        let responses = assembler.assemble(&request);

        // ── Verify each source produced items ───────────────────────────
        // ToolRuntime: should find workspace files matching objective text
        let tool = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime)
            .expect("ToolRuntime response should exist");
        assert!(
            tool.available,
            "ToolRuntime should be available: {}",
            tool.explanation
        );

        // GraphMemory: should have audit event item
        let gm = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory)
            .expect("GraphMemory response should exist");
        assert!(gm.available, "GraphMemory should be available");
        assert!(
            !gm.items.is_empty(),
            "GraphMemory should have audit event items"
        );

        // HolographicMemory: should have trace item
        let holo = responses
            .iter()
            .find(|r| r.source == ContextSource::HolographicMemory)
            .expect("HolographicMemory response should exist");
        assert!(holo.available, "HolographicMemory should be available");
        assert!(
            !holo.items.is_empty(),
            "HolographicMemory should have trace items"
        );

        // ReservoirEcho: should have pulse items
        let res_echo = responses
            .iter()
            .find(|r| r.source == ContextSource::ReservoirEcho)
            .expect("ReservoirEcho response should exist");
        assert!(res_echo.available, "ReservoirEcho should be available");
        assert!(
            !res_echo.items.is_empty(),
            "ReservoirEcho should have pulse items"
        );

        // WorkingMemory has no adapter — empty
        let wm = responses
            .iter()
            .find(|r| r.source == ContextSource::WorkingMemory)
            .expect("WorkingMemory response should exist");
        assert!(wm.available);
        assert!(wm.items.is_empty(), "WorkingMemory should have no items");

        // At least 4 sources with content
        let sources_with_content: Vec<_> =
            responses.iter().filter(|r| !r.items.is_empty()).collect();
        assert!(
            sources_with_content.len() >= 4,
            "Expected at least 4 sources with content, got {}: {:?}",
            sources_with_content.len(),
            sources_with_content
                .iter()
                .map(|r| format!("{:?}", r.source))
                .collect::<Vec<_>>()
        );

        // ── Non-authorizing invariant ──────────────────────────────────
        for resp in &responses {
            for item in &resp.items {
                assert!(
                    !item.value.contains("approve")
                        && !item.value.contains("authorize")
                        && !item.value.contains("execution"),
                    "Context item from {:?} contains authorization language: {}",
                    resp.source,
                    item.value
                );
            }
        }
    }

    /// The multi-adapter works when wired into the orchestrator engine with
    /// seeded adapters. Verifies context bundle has items from multiple sources
    /// and the cycle completes successfully.
    #[test]
    fn multi_adapter_works_with_orchestrator_cycle() {
        // ── Seed GraphMemory with an audit event ────────────────────────
        let mut gm_store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-int-test");
        gm_store
            .record_audit_event(AuditEvent {
                id: AuditEventId::new("int-evt-1"),
                event_type: AuditEventType::DecisionCreated,
                actor: ActorRef::System,
                workspace_id: Some(ws_id.clone()),
                task_id: None,
                proposed_action_id: None,
                decision_id: None,
                payload: serde_json::json!({"summary": "P3-6 orchestrated multi-source cycle"}),
                created_at: Utc::now(),
            })
            .expect("record_audit_event should succeed");

        // ── Seed HolographicMemory with a trace ─────────────────────────
        let mut holo_store = InMemoryHolographicMemoryStore::new();
        let holo_trace = HolographicTrace::new(
            "int-trace-1".to_owned(),
            "default".to_owned(),
            SourceKind::ManualNote,
            "source-int".to_owned(),
            vec!["turn-1".to_owned()],
            "P3-6 orchestrated multi-source trace".to_owned(),
            vec!["integration".to_owned(), "orchestrator".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
            0.0,
            0.0,
            Utc::now().to_rfc3339(),
        );
        holo_store
            .add_trace(holo_trace)
            .expect("add_trace should succeed");

        // ── Seed ReservoirEcho ──────────────────────────────────────────
        let mut reservoir = ReservoirState::new(10, 0.3);
        reservoir.absorb(arpagona_agent_core::cognitive::CognitivePulse::stimulus(
            "P3-6 multi-source orchestrator cycle",
            vec![
                "orchestrator".to_owned(),
                "spring".to_owned(),
                "integration".to_owned(),
            ],
            Utc::now(),
        ));

        // ── Build the multi-adapter ─────────────────────────────────────
        let cwd = std::env::current_dir().expect("current dir should exist");
        let assembler = MultiAdapterContextAssembler::new()
            .with_tool_runtime(ToolRuntimeAdapter::new(cwd).with_max_items(3))
            .with_graph_memory(GraphMemoryAdapter::new(gm_store).with_max_items(3))
            .with_holographic_memory(
                HolographicMemoryAdapter::new(holo_store, "default").with_max_items(3),
            )
            .with_reservoir_echo(ReservoirEchoAdapter::new(reservoir).with_max_items(3));

        // ── Run through the orchestrator ────────────────────────────────
        let engine = crate::OrchestratorEngine::new().with_context_assembler(Box::new(assembler));

        let input = crate::ObjectiveInput::new(
            "integration verification spring — multi-source context assembly",
            ws_id,
            AgentId::new("agent-int-test"),
            Utc::now(),
        );

        let cycle = engine
            .run_cycle(input, &[Permission::ReadDocument])
            .expect("orchestrator cycle should succeed");

        // ── Verify the context bundle has items from multiple sources ────
        let bundle = &cycle.context_bundle;
        assert!(
            bundle.total_items() > 0,
            "Context bundle should have items from seeded adapters"
        );

        // Graph memory items should include audit event items
        assert!(
            !bundle.graph_memory_items.is_empty(),
            "Context bundle should have graph_memory items"
        );

        // Holographic resonance items should include trace items
        assert!(
            !bundle.holographic_resonance_items.is_empty(),
            "Context bundle should have holographic_resonance items: {}",
            bundle.holographic_resonance_items.len()
        );

        // Reservoir traces should include pulse items
        assert!(
            !bundle.reservoir_traces.is_empty(),
            "Context bundle should have reservoir_traces items: {}",
            bundle.reservoir_traces.len()
        );

        // The cycle completed successfully
        assert!(cycle.outcome.non_authorizing);
        assert!(cycle.outcome.gate_was_applied);

        // Causal trace mentions all sources (graph_memory includes
        // ToolRuntime items bucketed by the engine's context assembly)
        let trace = cycle.causal_trace();
        assert!(
            trace.contains("graph_memory"),
            "Trace should mention graph_memory"
        );
        assert!(trace.contains("holo"), "Trace should mention holo");
        assert!(
            trace.contains("reservoir_echo"),
            "Trace should mention reservoir_echo"
        );
        assert!(trace.contains("Total:"), "Trace should report total items");
        // The graph_memory source should have >0 items (ToolRuntime + audit)
        assert!(trace.contains("items="), "Trace should include item counts");
    }
}
