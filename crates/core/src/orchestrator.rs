//! Neutral Orchestrator V0 domain contract.
//!
//! This module defines the smallest pure domain types for orchestrated work
//! cycles. The Neutral Orchestrator coordinates objective intake, context
//! assembly, compute routing, proposal routing, Decision Gate outcomes and
//! audit linkage — without becoming an execution, approval or scheduler layer.
//!
//! Every type is pure, serializable, I/O-free, LLM-free, tool-free,
//! persistence-free, and non-authorizing.
//!
//! Expected cycle shape:
//!
//! ```text
//! ObjectiveInput
//!   -> OrchestratorContextRequest -> ContextBundle(advisory)
//!   -> ComputeRouteRequest
//!   -> ProposalRequest
//!   -> ProposedAction or ToolCallIntent
//!   -> Decision Gate
//!   -> Audit-linked OrchestratorOutcome
//! ```
//!
//! Key invariants:
//! - Every step has explicit IDs for full audit traceability.
//! - Context, memory recall and compute route are advisory only.
//! - OrchestratorOutcome is always `non_authorizing: true`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::action::ToolCallIntent;
use crate::cognitive_work::{
    ContextItem, CycleStatus, Objective, ObjectiveDomain, ObjectiveStatus,
};
use crate::ids::{
    AgentId, AuditEventId, ComputeRouteId, ContextBundleId, DecisionId, ObjectiveId,
    OrchestratorCycleId, ProposalRequestId, ProposedActionId, WorkspaceId,
};

// ─── ObjectiveInput ────────────────────────────────────────────────────────

/// Input that starts an orchestrated work cycle.
///
/// This is the entry point to the Neutral Orchestrator. A human, another agent,
/// or a scheduled event submits an ObjectiveInput. The orchestrator creates the
/// cycle, assembles context, requests a compute route, and produces a proposal.
///
/// Pure domain: no I/O, no execution, no authorization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveInput {
    /// The raw objective text.
    pub text: String,
    /// Optional domain classification hint.
    pub domain_hint: Option<ObjectiveDomain>,
    /// Optional initial context text.
    pub context_hint: Option<String>,
    /// The workspace this cycle belongs to.
    pub workspace_id: WorkspaceId,
    /// The agent that submitted this input.
    pub agent_id: AgentId,
    /// The orchestrator cycle ID assigned to this input.
    pub cycle_id: OrchestratorCycleId,
    /// Timestamp of submission.
    pub created_at: DateTime<Utc>,
}

impl ObjectiveInput {
    /// Create a new ObjectiveInput with an auto-generated cycle ID.
    pub fn new(
        text: impl Into<String>,
        workspace_id: WorkspaceId,
        agent_id: AgentId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            text: text.into(),
            domain_hint: None,
            context_hint: None,
            workspace_id,
            agent_id,
            cycle_id: OrchestratorCycleId::new(format!("oc-{}", created_at.timestamp())),
            created_at,
        }
    }

    /// Attach an optional domain hint.
    pub fn with_domain(mut self, domain: ObjectiveDomain) -> Self {
        self.domain_hint = Some(domain);
        self
    }

    /// Attach optional initial context.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context_hint = Some(context.into());
        self
    }

    /// Convert this input into a bare Objective for downstream processing.
    ///
    /// This is a pure, non-authorizing conversion. The objective carries no
    /// execution rights.
    pub fn to_objective(&self) -> Objective {
        Objective {
            id: ObjectiveId::new(format!("obj-{}", self.cycle_id)),
            title: self.text.clone(),
            description: self.text.clone(),
            domain: self.domain_hint.clone().unwrap_or(ObjectiveDomain::General),
            status: ObjectiveStatus::Proposed,
            success_criteria: vec![],
            created_at: self.created_at,
        }
    }
}

// ─── OrchestratorContextRequest ────────────────────────────────────────────

/// Request to assemble advisory context for a cycle.
///
/// This is the formal request the orchestrator sends to the context-assembly
/// layer. It specifies which memory sources should be queried. The returned
/// ContextBundle is always advisory and does not authorize actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorContextRequest {
    /// The orchestrator cycle this request belongs to.
    pub cycle_id: OrchestratorCycleId,
    /// The objective being processed.
    pub objective_id: ObjectiveId,
    /// Which memory/context sources to query.
    pub requested_sources: Vec<ContextSource>,
    /// Timestamp of the request.
    pub created_at: DateTime<Utc>,
}

impl OrchestratorContextRequest {
    /// Create a new context request requesting all available sources.
    pub fn new(cycle_id: OrchestratorCycleId, objective_id: ObjectiveId) -> Self {
        Self {
            cycle_id: cycle_id.clone(),
            objective_id,
            requested_sources: vec![
                ContextSource::GraphMemory,
                ContextSource::HolographicMemory,
                ContextSource::ReservoirEcho,
                ContextSource::WorkingMemory,
            ],
            created_at: Utc::now(),
        }
    }

    /// Restrict to specific sources.
    pub fn with_sources(mut self, sources: Vec<ContextSource>) -> Self {
        self.requested_sources = sources;
        self
    }
}

// ─── ContextSource ─────────────────────────────────────────────────────────

/// Available memory/context sources for the orchestrator to query.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// Structured durable memory (facts, entities, relations).
    GraphMemory,
    /// Pattern resonance memory (symbolic associative recall).
    HolographicMemory,
    /// Short-term volatile cognitive continuity.
    ReservoirEcho,
    /// Workspace file-system perception (Tool Runtime).
    ToolRuntime,
    /// Active working memory from previous cycles.
    WorkingMemory,
}

// ─── ContextBundle ─────────────────────────────────────────────────────────

/// Advisory context bundle assembled from memory sources.
///
/// This bundle collects context from Graph Memory, Holographic Memory, Reservoir
/// Echo, and other sources. Every item is advisory only — no context in this
/// bundle may approve, authorize or execute an action.
///
/// The `advisory_warning` field is set automatically and must never be removed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextBundle {
    /// Unique identifier for this context bundle.
    pub id: ContextBundleId,
    /// The orchestrator cycle this bundle belongs to.
    pub cycle_id: OrchestratorCycleId,
    /// The objective this context was assembled for.
    pub objective_id: ObjectiveId,
    /// Context items retrieved from Graph Memory (advisory).
    pub graph_memory_items: Vec<ContextItem>,
    /// Context items from Holographic Memory resonance (advisory).
    pub holographic_resonance_items: Vec<ContextItem>,
    /// Active traces from Reservoir Echo (advisory).
    pub reservoir_traces: Vec<ContextItem>,
    /// Sources that were unavailable or returned no results.
    pub unavailable_sources: Vec<ContextSource>,
    /// Static warning — this bundle is advisory only, non-authorizing.
    pub advisory_warning: String,
    /// Timestamp of assembly.
    pub created_at: DateTime<Utc>,
}

impl ContextBundle {
    /// Create an empty advisory context bundle with the warning set.
    pub fn new(
        id: ContextBundleId,
        cycle_id: OrchestratorCycleId,
        objective_id: ObjectiveId,
    ) -> Self {
        Self {
            id,
            cycle_id,
            objective_id,
            graph_memory_items: vec![],
            holographic_resonance_items: vec![],
            reservoir_traces: vec![],
            unavailable_sources: vec![],
            advisory_warning: CONTEXT_BUNDLE_ADVISORY_WARNING.to_owned(),
            created_at: Utc::now(),
        }
    }

    /// Add Graph Memory context items (advisory).
    pub fn with_graph_memory(mut self, items: Vec<ContextItem>) -> Self {
        self.graph_memory_items = items;
        self
    }

    /// Add Holographic Memory resonance items (advisory).
    pub fn with_holographic_resonance(mut self, items: Vec<ContextItem>) -> Self {
        self.holographic_resonance_items = items;
        self
    }

    /// Add Reservoir Echo traces (advisory).
    pub fn with_reservoir_traces(mut self, traces: Vec<ContextItem>) -> Self {
        self.reservoir_traces = traces;
        self
    }

    /// Mark sources as unavailable.
    pub fn with_unavailable_sources(mut self, sources: Vec<ContextSource>) -> Self {
        self.unavailable_sources = sources;
        self
    }

    /// Return the total number of context items across all sources.
    pub fn total_items(&self) -> usize {
        self.graph_memory_items.len()
            + self.holographic_resonance_items.len()
            + self.reservoir_traces.len()
    }

    /// Return true if any context items are present.
    pub fn has_context(&self) -> bool {
        self.total_items() > 0
    }

    /// Return true if the bundle is empty (no context from any source).
    pub fn is_empty(&self) -> bool {
        self.total_items() == 0
    }
}

/// Static warning embedded in every ContextBundle.
///
/// This warning must never be removed, silenced, or rendered optional.
/// Context bundles are advisory by design.
pub const CONTEXT_BUNDLE_ADVISORY_WARNING: &str =
    "Advisory only — context, memory recall and resonance are non-authorizing. \
     No context in this bundle may approve, authorize or execute an action.";

// ─── ComputeRouteRequest ───────────────────────────────────────────────────

/// Request for compute route advice from the Compute Reservoir.
///
/// The Compute Reservoir returns a route recommendation (local, cloud, small,
/// large model). This recommendation is advisory — it does not approve actions,
/// execute tools, or authorize side effects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeRouteRequest {
    /// The orchestrator cycle this request belongs to.
    pub cycle_id: OrchestratorCycleId,
    /// The objective being processed.
    pub objective_id: ObjectiveId,
    /// The context bundle that was assembled.
    pub context_bundle_id: ContextBundleId,
    /// The requesting workspace.
    pub workspace_id: WorkspaceId,
    /// The requesting agent.
    pub agent_id: AgentId,
    /// Timestamp of the request.
    pub created_at: DateTime<Utc>,
}

impl ComputeRouteRequest {
    /// Create a new compute route request.
    pub fn new(
        cycle_id: OrchestratorCycleId,
        objective_id: ObjectiveId,
        context_bundle_id: ContextBundleId,
        workspace_id: WorkspaceId,
        agent_id: AgentId,
    ) -> Self {
        Self {
            cycle_id,
            objective_id,
            context_bundle_id,
            workspace_id,
            agent_id,
            created_at: Utc::now(),
        }
    }
}

// ─── ComputeRouteResult ────────────────────────────────────────────────────

/// Advisory result of a compute route request.
///
/// This is the orchestrator's view of a Compute Reservoir allocation. It is
/// advisory only — it does not approve actions, execute tools, or authorize
/// side effects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeRouteResult {
    /// The orchestrator-level compute route ID for cross-linking.
    pub id: ComputeRouteId,
    /// The request this result responds to.
    pub cycle_id: OrchestratorCycleId,
    pub objective_id: ObjectiveId,
    pub context_bundle_id: ContextBundleId,
    /// Descriptive label for the selected resource.
    pub selected_route_label: String,
    /// Whether local compute is preferred.
    pub local_preferred: bool,
    /// Human-readable justification for the route selection.
    pub justification: String,
    /// Static warning that this result is advisory only.
    pub advisory_warning: String,
    /// Timestamp of the result.
    pub created_at: DateTime<Utc>,
}

impl ComputeRouteResult {
    /// Create a new advisory compute route result with the warning set.
    pub fn new(
        id: ComputeRouteId,
        cycle_id: OrchestratorCycleId,
        objective_id: ObjectiveId,
        context_bundle_id: ContextBundleId,
        selected_route_label: impl Into<String>,
        local_preferred: bool,
        justification: impl Into<String>,
    ) -> Self {
        Self {
            id,
            cycle_id,
            objective_id,
            context_bundle_id,
            selected_route_label: selected_route_label.into(),
            local_preferred,
            justification: justification.into(),
            advisory_warning: COMPUTE_ROUTE_ADVISORY_WARNING.to_owned(),
            created_at: Utc::now(),
        }
    }
}

/// Static warning embedded in every ComputeRouteResult.
pub const COMPUTE_ROUTE_ADVISORY_WARNING: &str =
    "Advisory only — compute route recommendation is non-authorizing. \
     It does not approve actions, execute tools, or authorize side effects.";

// ─── ProposalRequest ───────────────────────────────────────────────────────

/// Request for a proposal from an agent (human, LLM, or deterministic logic).
///
/// The proposal request bundles the objective, advisory context, and advisory
/// compute route into a single request. The recipient may produce a
/// ProposedAction, a ToolCallIntent, or a refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposalRequest {
    /// Unique identifier for this proposal request.
    pub id: ProposalRequestId,
    /// The orchestrator cycle this request belongs to.
    pub cycle_id: OrchestratorCycleId,
    /// The objective being processed.
    pub objective_id: ObjectiveId,
    /// The context bundle assembled for this cycle.
    pub context_bundle_id: ContextBundleId,
    /// The compute route result (if computed).
    pub compute_route_id: Option<ComputeRouteId>,
    /// Human-readable explanation of the compute route (advisory).
    pub compute_route_explanation: Option<String>,
    /// The requesting workspace.
    pub workspace_id: WorkspaceId,
    /// The requesting agent.
    pub agent_id: AgentId,
    /// Timestamp of the request.
    pub created_at: DateTime<Utc>,
}

impl ProposalRequest {
    /// Create a new proposal request.
    pub fn new(
        cycle_id: OrchestratorCycleId,
        objective_id: ObjectiveId,
        context_bundle_id: ContextBundleId,
        workspace_id: WorkspaceId,
        agent_id: AgentId,
    ) -> Self {
        Self {
            id: ProposalRequestId::new(format!("pr-{}", cycle_id)),
            cycle_id,
            objective_id,
            context_bundle_id,
            compute_route_id: None,
            compute_route_explanation: None,
            workspace_id,
            agent_id,
            created_at: Utc::now(),
        }
    }

    /// Attach a compute route result (advisory).
    pub fn with_compute_route(mut self, route: &ComputeRouteResult) -> Self {
        self.compute_route_id = Some(route.id.clone());
        self.compute_route_explanation = Some(route.justification.clone());
        self
    }
}

// ─── OrchestratorOutcome ───────────────────────────────────────────────────

/// The linked outcome of one orchestrated work cycle.
///
/// Every step of the cycle is linkable through explicit IDs:
///   objective_id → context_bundle_id → compute_route_id → proposal_request_id
///   → proposed_action_id / tool_call_intent → decision_id → audit_event_ids
///
/// This outcome is always non-authorizing. It records what happened during the
/// cycle but does not grant execution rights. The Decision Gate outcome and
/// audit events carry the actual governance state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorOutcome {
    /// The orchestrator cycle this outcome belongs to.
    pub cycle_id: OrchestratorCycleId,
    /// The objective that drove this cycle.
    pub objective_id: ObjectiveId,
    /// The context bundle that was assembled.
    pub context_bundle_id: ContextBundleId,
    /// The compute route that was selected (if any).
    pub compute_route_id: Option<ComputeRouteId>,
    /// The proposal request that was sent.
    pub proposal_request_id: ProposalRequestId,
    /// The proposed action ID (if a proposal was created).
    pub proposed_action_id: Option<ProposedActionId>,
    /// The tool call intent (if direct tool-call was requested).
    pub tool_call_intent: Option<ToolCallIntent>,
    /// The decision from the Decision Gate (if gated).
    pub decision_id: Option<DecisionId>,
    /// All audit event IDs recorded during this cycle.
    pub audit_event_ids: Vec<AuditEventId>,
    /// Human-readable summary of the cycle outcome.
    pub summary: String,
    /// Whether this outcome went through the Decision Gate.
    pub gate_was_applied: bool,
    /// The lifecycle status of this cycle.
    pub cycle_status: CycleStatus,
    /// Invariant: orchestrator outcomes are always non-authorizing.
    /// This field is set to true at construction and must never be set to false.
    pub non_authorizing: bool,
    /// Timestamp of the outcome.
    pub created_at: DateTime<Utc>,
}

impl OrchestratorOutcome {
    /// Create a new OrchestratorOutcome with non_authorizing set to true.
    ///
    /// The outcome starts with only cycle_id, objective_id, context_bundle_id
    /// and summary. Other fields should be set via builder methods as the
    /// cycle progresses.
    pub fn new(
        cycle_id: OrchestratorCycleId,
        objective_id: ObjectiveId,
        context_bundle_id: ContextBundleId,
        summary: impl Into<String>,
        cycle_status: CycleStatus,
    ) -> Self {
        Self {
            cycle_id,
            objective_id,
            context_bundle_id,
            compute_route_id: None,
            proposal_request_id: ProposalRequestId::new(format!(
                "pr-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or(0)
            )),
            proposed_action_id: None,
            tool_call_intent: None,
            decision_id: None,
            audit_event_ids: vec![],
            summary: summary.into(),
            gate_was_applied: false,
            cycle_status,
            non_authorizing: true,
            created_at: Utc::now(),
        }
    }

    /// Attach a compute route result to this outcome.
    pub fn with_compute_route(mut self, route_id: ComputeRouteId) -> Self {
        self.compute_route_id = Some(route_id);
        self
    }

    /// Attach a proposed action ID.
    pub fn with_proposed_action(mut self, action_id: ProposedActionId) -> Self {
        self.proposed_action_id = Some(action_id);
        self
    }

    /// Attach a tool call intent.
    pub fn with_tool_call_intent(mut self, intent: ToolCallIntent) -> Self {
        self.tool_call_intent = Some(intent);
        self
    }

    /// Attach a Decision Gate decision.
    pub fn with_decision(mut self, decision_id: DecisionId) -> Self {
        self.decision_id = Some(decision_id);
        self.gate_was_applied = true;
        self
    }

    /// Add audit event IDs recorded during this cycle.
    pub fn with_audit_events(mut self, event_ids: Vec<AuditEventId>) -> Self {
        self.audit_event_ids = event_ids;
        self
    }

    /// Return true if this outcome has a Decision Gate link.
    pub fn has_decision(&self) -> bool {
        self.decision_id.is_some()
    }

    /// Return true if this outcome produced a proposed action.
    pub fn has_proposed_action(&self) -> bool {
        self.proposed_action_id.is_some() || self.tool_call_intent.is_some()
    }

    /// Return the number of audit events linked to this outcome.
    pub fn audit_event_count(&self) -> usize {
        self.audit_event_ids.len()
    }
}

// ─── Builder for orchestrator cycle chain ──────────────────────────────────

/// Build a complete set of orchestrator domain objects from an objective input.
///
/// This is a pure, non-authorizing helper that creates the full chain of
/// domain objects: ObjectiveInput → Objective → ContextBundle →
/// ComputeRouteResult (mock) → ProposalRequest → OrchestratorOutcome.
///
/// No I/O, no LLM calls, no persistence, no execution.
pub fn build_demo_orchestrator_cycle(
    input: &ObjectiveInput,
) -> (
    Objective,
    ContextBundle,
    ComputeRouteResult,
    ProposalRequest,
    OrchestratorOutcome,
) {
    let objective = input.to_objective();
    let now = Utc::now();

    let context_bundle = ContextBundle::new(
        ContextBundleId::new(format!("cb-{}", now.timestamp())),
        input.cycle_id.clone(),
        objective.id.clone(),
    );

    let compute_route = ComputeRouteResult::new(
        ComputeRouteId::new(format!("cr-{}", now.timestamp())),
        input.cycle_id.clone(),
        objective.id.clone(),
        context_bundle.id.clone(),
        "local_deterministic",
        true,
        "Local deterministic compute selected by default for V0 demo cycle.",
    );

    let proposal_request = ProposalRequest::new(
        input.cycle_id.clone(),
        objective.id.clone(),
        context_bundle.id.clone(),
        input.workspace_id.clone(),
        input.agent_id.clone(),
    );

    let outcome = OrchestratorOutcome::new(
        input.cycle_id.clone(),
        objective.id.clone(),
        context_bundle.id.clone(),
        "Orchestrator demo cycle completed (no governance applied).",
        CycleStatus::Completed,
    )
    .with_compute_route(compute_route.id.clone());

    (
        objective,
        context_bundle,
        compute_route,
        proposal_request,
        outcome,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ─── Serialization tests ───────────────────────────────────────────

    #[test]
    fn objective_input_serializes_and_deserializes() {
        let now = Utc::now();
        let input = ObjectiveInput::new(
            "Analyse project structure",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            now,
        )
        .with_domain(ObjectiveDomain::Engineering)
        .with_context("crates/core/src");

        let encoded = serde_json::to_value(&input).expect("should serialize");
        assert_eq!(encoded["text"], json!("Analyse project structure"));
        assert_eq!(encoded["domain_hint"], json!("engineering"));

        let decoded: ObjectiveInput = serde_json::from_value(encoded).expect("should deserialize");
        assert_eq!(decoded.text, input.text);
        assert_eq!(decoded.domain_hint, input.domain_hint);
    }

    #[test]
    fn objective_input_to_objective_is_non_authorizing() {
        let input = ObjectiveInput::new(
            "Research market trends",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            Utc::now(),
        );
        let objective = input.to_objective();

        assert_eq!(objective.title, "Research market trends");
        assert_eq!(objective.status, ObjectiveStatus::Proposed);
        // A proposed objective has no execution rights
        assert!(objective.success_criteria.is_empty());
    }

    #[test]
    fn context_bundle_serializes_and_is_advisory() {
        let now = Utc::now();
        let bundle = ContextBundle::new(
            ContextBundleId::new("cb-1"),
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
        )
        .with_graph_memory(vec![ContextItem {
            key: "project_name".to_owned(),
            value: "ARPAGONA".to_owned(),
            source: "graph_memory".to_owned(),
        }])
        .with_unavailable_sources(vec![ContextSource::HolographicMemory]);

        assert!(bundle.has_context());
        assert_eq!(bundle.total_items(), 1);
        assert!(bundle.advisory_warning.contains("non-authorizing"));
        assert!(bundle.holographic_resonance_items.is_empty());

        let encoded = serde_json::to_value(&bundle).expect("should serialize");
        assert_eq!(encoded["advisory_warning"], bundle.advisory_warning);
        assert_eq!(
            encoded["graph_memory_items"][0]["key"],
            json!("project_name")
        );
    }

    #[test]
    fn empty_context_bundle_is_empty() {
        let bundle = ContextBundle::new(
            ContextBundleId::new("cb-empty"),
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
        );
        assert!(bundle.is_empty());
        assert!(!bundle.has_context());
        assert_eq!(bundle.total_items(), 0);
    }

    #[test]
    fn compute_route_result_is_advisory() {
        let route = ComputeRouteResult::new(
            ComputeRouteId::new("cr-1"),
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            "local_ollama_qwen3.5",
            true,
            "Sensitive data requires local processing.",
        );

        assert!(route.advisory_warning.contains("non-authorizing"));
        assert_eq!(route.selected_route_label, "local_ollama_qwen3.5");
        assert!(route.local_preferred);

        let encoded = serde_json::to_value(&route).expect("should serialize");
        assert_eq!(encoded["local_preferred"], json!(true));
        assert_eq!(
            encoded["selected_route_label"],
            json!("local_ollama_qwen3.5")
        );
    }

    #[test]
    fn proposal_request_links_all_cycle_ids() {
        let request = ProposalRequest::new(
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
        );

        assert_eq!(request.cycle_id.as_str(), "oc-1");
        assert_eq!(request.objective_id.as_str(), "obj-1");
        assert_eq!(request.context_bundle_id.as_str(), "cb-1");
        assert!(request.compute_route_id.is_none());
    }

    #[test]
    fn proposal_request_accepts_compute_route() {
        let route = ComputeRouteResult::new(
            ComputeRouteId::new("cr-1"),
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            "local_deterministic",
            true,
            "Demo route.",
        );

        let request = ProposalRequest::new(
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
        )
        .with_compute_route(&route);

        assert_eq!(request.compute_route_id, Some(ComputeRouteId::new("cr-1")));
        assert!(request
            .compute_route_explanation
            .as_ref()
            .unwrap()
            .contains("Demo"));
    }

    // ─── OrchestratorOutcome tests ──────────────────────────────────────

    #[test]
    fn orchestrator_outcome_is_always_non_authorizing() {
        let outcome = OrchestratorOutcome::new(
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            "Cycle complete",
            CycleStatus::Completed,
        );

        assert!(outcome.non_authorizing);
        // The invariant is structural: non_authorizing is set at construction
        // and there is no setter to change it.
        assert!(!outcome.has_decision());
        assert!(!outcome.has_proposed_action());
    }

    #[test]
    fn orchestrator_outcome_builds_full_chain() {
        let decision_id = DecisionId::new("dg-1");
        let action_id = ProposedActionId::new("pa-1");

        let outcome = OrchestratorOutcome::new(
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            "Full cycle completed with governance",
            CycleStatus::Completed,
        )
        .with_compute_route(ComputeRouteId::new("cr-1"))
        .with_proposed_action(action_id.clone())
        .with_decision(decision_id.clone())
        .with_audit_events(vec![
            AuditEventId::new("audit-1"),
            AuditEventId::new("audit-2"),
        ]);

        assert!(outcome.non_authorizing);
        assert!(outcome.has_decision());
        assert!(outcome.has_proposed_action());
        assert!(outcome.gate_was_applied);
        assert_eq!(outcome.audit_event_count(), 2);
        assert_eq!(outcome.proposed_action_id, Some(action_id));
        assert_eq!(outcome.decision_id, Some(decision_id));
    }

    #[test]
    fn orchestrator_outcome_serializes_with_ids() {
        let outcome = OrchestratorOutcome::new(
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            "Demo cycle",
            CycleStatus::Completed,
        )
        .with_compute_route(ComputeRouteId::new("cr-1"));

        let encoded = serde_json::to_value(&outcome).expect("should serialize");
        assert_eq!(encoded["cycle_id"], json!("oc-1"));
        assert_eq!(encoded["objective_id"], json!("obj-1"));
        assert_eq!(encoded["context_bundle_id"], json!("cb-1"));
        assert_eq!(encoded["compute_route_id"], json!("cr-1"));
        assert_eq!(encoded["non_authorizing"], json!(true));

        let decoded: OrchestratorOutcome =
            serde_json::from_value(encoded).expect("should deserialize");
        assert_eq!(decoded.cycle_id.as_str(), "oc-1");
        assert_eq!(decoded.objective_id.as_str(), "obj-1");
    }

    // ─── Demo cycle builder tests ───────────────────────────────────────

    #[test]
    fn demo_orchestrator_cycle_creates_full_domain_chain() {
        let now = Utc::now();
        let input = ObjectiveInput::new(
            "Review project documentation",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-alpha"),
            now,
        );

        let (objective, bundle, route, request, outcome) = build_demo_orchestrator_cycle(&input);

        // Chain: objective → bundle → route → request → outcome
        assert_eq!(objective.id.as_str(), bundle.objective_id.as_str());
        assert_eq!(bundle.id.as_str(), route.context_bundle_id.as_str());
        assert_eq!(route.cycle_id.as_str(), request.cycle_id.as_str());
        assert_eq!(outcome.context_bundle_id, bundle.id);
        assert_eq!(outcome.compute_route_id, Some(route.id));

        // All are non-authorizing
        assert!(bundle.advisory_warning.contains("non-authorizing"));
        assert!(route.advisory_warning.contains("non-authorizing"));
        assert!(outcome.non_authorizing);
    }

    #[test]
    fn objective_input_with_domain_and_context_converts_correctly() {
        let now = Utc::now();
        let input = ObjectiveInput::new(
            "Business strategy 2026",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            now,
        )
        .with_domain(ObjectiveDomain::Business)
        .with_context("Quarterly revenue data");

        assert_eq!(input.domain_hint, Some(ObjectiveDomain::Business));
        assert_eq!(
            input.context_hint,
            Some("Quarterly revenue data".to_owned())
        );

        let objective = input.to_objective();
        assert_eq!(objective.domain, ObjectiveDomain::Business);
        assert_eq!(objective.title, "Business strategy 2026");
    }

    // ─── Advisory invariant tests ───────────────────────────────────────

    #[test]
    fn context_bundle_never_authorizes_actions() {
        // Prove that ContextBundle has no approval/authorization/execution fields
        let bundle = ContextBundle::new(
            ContextBundleId::new("cb-test"),
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
        );

        // The bundle contains advisory context only
        assert!(bundle.graph_memory_items.is_empty());
        assert!(bundle.holographic_resonance_items.is_empty());
        assert!(bundle.reservoir_traces.is_empty());

        // Verify structural absence of authorization fields
        // (compile-time check via field access)
        let bundle_json = serde_json::to_value(&bundle).expect("serialize");
        assert!(bundle_json.get("approved").is_none());
        assert!(bundle_json.get("authorized").is_none());
        assert!(bundle_json.get("execution_token").is_none());
        assert!(bundle_json.get("permit").is_none());
    }

    #[test]
    fn compute_route_result_never_authorizes_actions() {
        let route = ComputeRouteResult::new(
            ComputeRouteId::new("cr-test"),
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            ContextBundleId::new("cb-test"),
            "local",
            true,
            "Test route.",
        );

        let route_json = serde_json::to_value(&route).expect("serialize");
        assert!(route_json.get("approved").is_none());
        assert!(route_json.get("authorized").is_none());
        assert!(route_json.get("execution_token").is_none());
    }

    #[test]
    fn orchestrator_outcome_has_no_approval_fields() {
        let outcome = OrchestratorOutcome::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            ContextBundleId::new("cb-test"),
            "Test outcome",
            CycleStatus::Completed,
        );

        let outcome_json = serde_json::to_value(&outcome).expect("serialize");
        assert!(outcome_json.get("approved").is_none());
        assert!(outcome_json.get("executed").is_none());
        assert!(outcome_json.get("authorization").is_none());
        assert!(outcome_json.get("execution_permit").is_none());
    }

    // ─── Linkage integrity tests ────────────────────────────────────────

    #[test]
    fn orchestrator_outcome_ids_are_consistent_across_builders() {
        let cycle_id = OrchestratorCycleId::new("oc-42");
        let obj_id = ObjectiveId::new("obj-42");
        let cb_id = ContextBundleId::new("cb-42");

        let outcome = OrchestratorOutcome::new(
            cycle_id.clone(),
            obj_id.clone(),
            cb_id.clone(),
            "Linkage test",
            CycleStatus::Completed,
        );

        assert_eq!(outcome.cycle_id, cycle_id);
        assert_eq!(outcome.objective_id, obj_id);
        assert_eq!(outcome.context_bundle_id, cb_id);
        assert!(outcome.proposal_request_id.as_str().starts_with("pr-"));
    }
}
