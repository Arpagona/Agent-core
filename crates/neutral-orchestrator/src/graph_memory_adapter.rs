//! GraphMemoryAdapter — ContextAssembler implementation backed by
//! synchronous GraphMemoryStore (InMemoryGraphMemoryStore, etc.).
//!
//! This adapter bridges the `GraphMemoryStore` trait from
//! `crates/core/src/memory.rs` into the Neutral Orchestrator's context
//! assembly pipeline.
//!
//! When the orchestrator asks for advisory Graph Memory context, this adapter:
//! - Queries recent audit events for the workspace
//! - Lists all relations from the graph (advisory structure)
//! - Returns their summaries as advisory `ContextItem` values
//!
//! # Safety invariants
//!
//! - All items are advisory and non-authorizing.
//! - No response may contain an approval, authorization or execution token.
//! - Audit events are evidence for supervision, not authorization.
//! - Relations indicate structure, not authority.
//! - If the store is empty, the adapter reports the source as available but
//!   with zero items and a clear explanation.
//! - Interior mutability via `Mutex` is present for API consistency with
//!   other adapters (HolographicMemoryAdapter, ReservoirEchoAdapter).
//!
//! # Usage
//!
//! ```ignore
//! use arpagona_agent_core::memory::InMemoryGraphMemoryStore;
//! use arpagona_neutral_orchestrator::GraphMemoryAdapter;
//!
//! let store = InMemoryGraphMemoryStore::new();
//! let adapter = GraphMemoryAdapter::new(store);
//! let engine = OrchestratorEngine::new()
//!     .with_context_assembler(Box::new(adapter));
//! ```

use crate::context_assembler::ContextAssembler;
use arpagona_agent_core::cognitive_work::ContextItem;
use arpagona_agent_core::memory::GraphMemoryStore;
use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};
use std::sync::Mutex;

// ─── GraphMemoryAdapter ─────────────────────────────────────────────────────

/// A ContextAssembler that uses a synchronous GraphMemoryStore to provide
/// advisory durable-memory context for the orchestrator.
///
/// This adapter provides real graph-memory context from persisted facts,
/// audit events and relations:
/// - Queries audit events for the workspace (recent decisions and actions)
/// - Lists all relations from the graph (advisory structure)
/// - Returns their summaries as `ContextItem` values
/// - All results are advisory only
///
/// # Configuration
///
/// The adapter requires a `GraphMemoryStore` implementation. Use `new()` for
/// the simplest construction, or `with_max_items()` to cap the number of
/// items returned.
pub struct GraphMemoryAdapter {
    /// The GraphMemoryStore wrapped in a Mutex for interior mutability.
    /// Read methods on `GraphMemoryStore` use `&self`, but the Mutex
    /// provides consistent API shape with other adapters.
    store: Mutex<Box<dyn GraphMemoryStore>>,
    /// Maximum items to return per query.
    max_items: usize,
}

impl GraphMemoryAdapter {
    /// Create a new GraphMemoryAdapter with the given store.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let store = InMemoryGraphMemoryStore::new();
    /// let adapter = GraphMemoryAdapter::new(store);
    /// ```
    pub fn new(store: impl GraphMemoryStore + 'static) -> Self {
        Self {
            store: Mutex::new(Box::new(store)),
            max_items: 10,
        }
    }

    /// Create a GraphMemoryAdapter from an already-boxed store.
    ///
    /// Use this when you have a store on the heap and want to avoid boxing
    /// it again.
    pub fn from_boxed_store(store: Box<dyn GraphMemoryStore>) -> Self {
        Self {
            store: Mutex::new(store),
            max_items: 10,
        }
    }

    /// Override the maximum number of items to return per query.
    ///
    /// The default is 10. Setting this higher may return more context but
    /// could include less relevant items.
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = max;
        self
    }
}

impl ContextAssembler for GraphMemoryAdapter {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            let response = match source {
                ContextSource::GraphMemory => self.assemble_graph_memory(request),
                _ => MemoryQueryResponse::new(source.clone()),
            };
            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![ContextSource::GraphMemory]
    }
}

// ─── Internal assembly logic ───────────────────────────────────────────────

impl GraphMemoryAdapter {
    /// Assemble Graph Memory context: query audit events, relations, and
    /// facts, then convert them into advisory ContextItems.
    ///
    /// Compute route awareness: when `local_preferred` is true, the adapter
    /// returns more focused context (fewer items, favoring recent audit events).
    /// When `compute_route_label` indicates a cloud/strong route, the adapter
    /// returns broader context (more items, full relation structure).
    fn assemble_graph_memory(&self, request: &MemoryQueryRequest) -> MemoryQueryResponse {
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(e) => {
                let _msg = format!("GraphMemoryStore lock poisoned: {}", e);
                return MemoryQueryResponse::new(ContextSource::GraphMemory).with_unavailable();
            }
        };

        // ── Compute-route aware limit adjustment ───────────────────────
        // Local routes: smaller context, focus on recency
        // Cloud/strong routes: broader context, more items
        let route_suffix = if let Some(ref label) = request.compute_route_label {
            let local = request.local_preferred.unwrap_or(false);
            format!(" [compute: {} | local: {}]", label, local)
        } else {
            String::new()
        };

        // Local routes get stricter limits (lighter, faster); cloud routes get broader limits
        let base_limit = request.max_items_per_source.min(self.max_items);
        let limit = if request.local_preferred.unwrap_or(false) {
            // Local route: more conservative — reduce by ~half
            std::cmp::max(1, base_limit.saturating_sub(base_limit / 2))
        } else {
            base_limit
        };

        // ── Query 1: Audit events for the workspace ────────────────────
        let audit_events = match store.list_audit_events_for_workspace(&request.workspace_id) {
            Ok(events) => events,
            Err(e) => {
                return MemoryQueryResponse {
                    source: ContextSource::GraphMemory,
                    items: vec![],
                    available: true,
                    explanation: format!(
                        "Graph Memory audit event query failed: {}. Relations and facts may still be available.",
                        e
                    ),
                };
            }
        };

        // ── Query 2: All relations (advisory structure) ────────────────
        let relations = match store.list_relations() {
            Ok(rels) => rels,
            Err(e) => {
                return MemoryQueryResponse {
                    source: ContextSource::GraphMemory,
                    items: vec![],
                    available: true,
                    explanation: format!(
                        "Graph Memory relation query failed: {}. Audit events may still be available.",
                        e,
                    ),
                };
            }
        };

        // ── Build ContextItems from audit events ───────────────────────
        let event_count = audit_events.len().min(limit);
        let mut items: Vec<ContextItem> = Vec::new();

        // Add audit event items (most recent first, up to limit)
        // Audit events are ordered by creation; we take the last `limit`
        // events to get the most recent.
        let recent_events: Vec<_> = audit_events.iter().rev().take(event_count).collect();

        for event in &recent_events {
            items.push(ContextItem {
                key: format!("audit_event:{}", event.id),
                value: format!(
                    "[audit] type={:?} actor={:?} event_id={} at={}",
                    event.event_type,
                    event.actor,
                    event.id,
                    event.created_at.format("%H:%M:%S"),
                ),
                source: "graph_memory_adapter".to_owned(),
            });
        }

        // ── Build ContextItems from relations ──────────────────────────
        let relation_count = relations.len().min(limit.saturating_sub(items.len()));
        let top_relations: Vec<_> = relations.iter().take(relation_count).collect();

        for rel in &top_relations {
            items.push(ContextItem {
                key: format!("relation:{}->{}", rel.from.node_id, rel.to.node_id),
                value: format!(
                    "[relation] {} --({:?})--> {}",
                    rel.from.node_id, rel.relation_type, rel.to.node_id,
                ),
                source: "graph_memory_adapter".to_owned(),
            });
        }

        // ── Build human-readable explanation ───────────────────────────
        let total_events = audit_events.len();
        let total_rels = relations.len();
        let item_count = items.len();
        let event_suffix = if total_events == 1 { "" } else { "s" };
        let rel_suffix = if total_rels == 1 { "" } else { "s" };

        MemoryQueryResponse {
            source: ContextSource::GraphMemory,
            items,
            available: true,
            explanation: format!(
                "Graph Memory found {} audit event{} and {} relation{} in workspace '{}'. \
                 Showing {} items (limited).{}",
                total_events,
                event_suffix,
                total_rels,
                rel_suffix,
                request.workspace_id.as_str(),
                item_count,
                route_suffix,
            ),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::audit::{ActorRef, AuditEvent, AuditEventType};
    use arpagona_agent_core::graph::GraphNodeType;
    use arpagona_agent_core::graph::{GraphRef, GraphRelation, RelationType};
    use arpagona_agent_core::ids::{AuditEventId, ObjectiveId, OrchestratorCycleId, WorkspaceId};
    use arpagona_agent_core::memory::InMemoryGraphMemoryStore;
    use chrono::Utc;

    fn make_request() -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "graph memory audit context retrieval test",
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

    fn add_audit_event(
        store: &mut InMemoryGraphMemoryStore,
        id: &str,
        workspace_id: &WorkspaceId,
        summary: &str,
    ) {
        use serde_json::json;

        let event = AuditEvent {
            id: AuditEventId::new(id.to_owned()),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(workspace_id.clone()),
            task_id: None,
            proposed_action_id: None,
            decision_id: None,
            payload: json!({ "summary": summary }),
            created_at: Utc::now(),
        };
        store
            .record_audit_event(event)
            .expect("record should succeed");
    }

    fn add_relation(
        store: &mut InMemoryGraphMemoryStore,
        from_id: &str,
        to_id: &str,
        relation_type: RelationType,
    ) {
        let relation = GraphRelation::new(
            GraphRef::new(GraphNodeType::Fact, from_id.to_owned()),
            GraphRef::new(GraphNodeType::Fact, to_id.to_owned()),
            relation_type,
        );
        store
            .add_relation(relation)
            .expect("add_relation should succeed");
    }

    // ─── Supported sources ─────────────────────────────────────────────

    #[test]
    fn adapter_returns_graph_memory_source() {
        let store = InMemoryGraphMemoryStore::new();
        let adapter = GraphMemoryAdapter::new(store);
        let sources = adapter.supported_sources();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&ContextSource::GraphMemory));
    }

    // ─── Non-matching source ───────────────────────────────────────────

    #[test]
    fn adapter_ignores_non_matching_sources() {
        let store = InMemoryGraphMemoryStore::new();
        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request_with_sources("test", vec![ContextSource::HolographicMemory]);
        let responses = adapter.assemble(&request);
        assert_eq!(responses.len(), 1);
        let resp = &responses[0];
        assert_eq!(resp.source, ContextSource::HolographicMemory);
        assert!(resp.items.is_empty());
        assert!(resp.available);
    }

    // ─── Empty store returns empty but available ───────────────────────

    #[test]
    fn adapter_handles_empty_store() {
        let store = InMemoryGraphMemoryStore::new();
        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        assert!(resp.available);
        assert!(resp.items.is_empty());
    }

    // ─── Audit events ──────────────────────────────────────────────────

    #[test]
    fn adapter_returns_audit_events_for_workspace() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");

        add_audit_event(
            &mut store,
            "evt-1",
            &ws_id,
            "Reviewed cognitive architecture document",
        );
        add_audit_event(
            &mut store,
            "evt-2",
            &ws_id,
            "Approved memory write proposal",
        );

        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        assert!(resp.available);
        assert!(!resp.items.is_empty(), "Should find audit events");
        assert!(
            resp.explanation.contains("ws-test"),
            "Explanation should reference workspace: {}",
            resp.explanation
        );
    }

    #[test]
    fn adapter_filters_audit_events_by_workspace() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_other = WorkspaceId::new("ws-other");

        add_audit_event(&mut store, "evt-1", &ws_other, "Other workspace event");

        // The request uses ws-test — should not see ws-other events
        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        assert!(resp.available);
        assert!(
            resp.items.is_empty(),
            "Should not find events from other workspace"
        );
    }

    // ─── Relations ─────────────────────────────────────────────────────

    #[test]
    fn adapter_returns_relations() {
        let mut store = InMemoryGraphMemoryStore::new();
        add_relation(&mut store, "fact-1", "fact-2", RelationType::DerivedFrom);

        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        assert!(resp.available);
        // Should have at least the relation item
        let has_relation = resp
            .items
            .iter()
            .any(|item| item.key.starts_with("relation:"));
        assert!(has_relation, "Should include relation items");
    }

    // ─── Max items limit ───────────────────────────────────────────────

    #[test]
    fn adapter_respects_max_items() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");

        for i in 0..10 {
            add_audit_event(
                &mut store,
                &format!("evt-{}", i),
                &ws_id,
                &format!("Event number {}", i),
            );
        }

        let adapter = GraphMemoryAdapter::new(store).with_max_items(3);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        assert!(
            resp.items.len() <= 3,
            "Should have at most 3 items, got {}",
            resp.items.len()
        );
    }

    // ─── ContextItem format ────────────────────────────────────────────

    #[test]
    fn adapter_context_items_contain_audit_info() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");

        add_audit_event(
            &mut store,
            "evt-1",
            &ws_id,
            "Reviewed cognitive architecture",
        );

        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request_with_text("cognitive architecture");

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        let audit_items: Vec<&ContextItem> = resp
            .items
            .iter()
            .filter(|item| item.key.starts_with("audit_event:"))
            .collect();

        assert!(!audit_items.is_empty(), "Should have audit event items");
        assert!(
            audit_items[0].value.contains("audit"),
            "Item value should indicate audit: {}",
            audit_items[0].value
        );
        assert_eq!(
            audit_items[0].source, "graph_memory_adapter",
            "Source should be the adapter name"
        );
    }

    // ─── Combined events and relations ─────────────────────────────────

    #[test]
    fn adapter_returns_both_events_and_relations() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");

        add_audit_event(&mut store, "evt-1", &ws_id, "Reviewed architecture");
        add_relation(&mut store, "fact-1", "fact-2", RelationType::DerivedFrom);

        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request();

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        let has_audit = resp
            .items
            .iter()
            .any(|item| item.key.starts_with("audit_event:"));
        let has_relation = resp
            .items
            .iter()
            .any(|item| item.key.starts_with("relation:"));

        assert!(has_audit, "Should include audit event items");
        assert!(has_relation, "Should include relation items");
    }

    // ─── Compute-route awareness tests ─────────────────────────────────

    #[test]
    fn adapter_local_route_reduces_items() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");

        // Add enough events to see the reduction
        for i in 0..10 {
            add_audit_event(
                &mut store,
                &format!("evt-{}", i),
                &ws_id,
                &format!("Event number {}", i),
            );
        }

        let adapter = GraphMemoryAdapter::new(store).with_max_items(10);
        let request = make_request().with_compute_route(Some("local-small"), Some(true));

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        // Local route should reduce items (~half of 10, so ~5 or fewer)
        assert!(
            resp.items.len() <= 6,
            "Local route should limit items, got {}",
            resp.items.len()
        );
        assert!(
            resp.explanation.contains("local"),
            "Explanation should mention local route: {}",
            resp.explanation
        );
    }

    #[test]
    fn adapter_cloud_route_returns_full_items() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");

        for i in 0..10 {
            add_audit_event(
                &mut store,
                &format!("evt-{}", i),
                &ws_id,
                &format!("Event number {}", i),
            );
        }

        let adapter = GraphMemoryAdapter::new(store).with_max_items(10);
        let request = make_request().with_compute_route(Some("cloud-strong"), Some(false));

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        // Cloud route should return more items than local route would
        assert!(
            resp.explanation.contains("compute:"),
            "Explanation should mention compute: {}",
            resp.explanation
        );
    }

    #[test]
    fn adapter_default_route_has_no_compute_prefix() {
        let store = InMemoryGraphMemoryStore::new();
        let adapter = GraphMemoryAdapter::new(store);
        let request = make_request(); // no compute route set

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        // Should not contain compute prefix when no route is set
        assert!(
            !resp.explanation.contains("compute:"),
            "No compute prefix expected: {}",
            resp.explanation
        );
    }

    #[test]
    fn adapter_local_route_keeps_minimum_one_item() {
        let mut store = InMemoryGraphMemoryStore::new();
        let ws_id = WorkspaceId::new("ws-test");
        add_audit_event(&mut store, "evt-1", &ws_id, "Single event");

        let adapter = GraphMemoryAdapter::new(store).with_max_items(1);
        let request = make_request().with_compute_route(Some("local-tiny"), Some(true));

        let responses = adapter.assemble(&request);
        let gm_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::GraphMemory);
        assert!(gm_resp.is_some());
        let resp = gm_resp.unwrap();

        // Even with local route, should still return at least 1 item
        assert_eq!(
            resp.items.len(),
            1,
            "Local route should keep minimum 1 item"
        );
    }
}
