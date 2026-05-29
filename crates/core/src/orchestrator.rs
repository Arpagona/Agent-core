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

use crate::failure_insight::{
    CorrectionTarget, DetectionSignal, DetectionSignalType, FailureClass, FailureInsight,
    InsightSeverity,
};
use crate::ids::FailureInsightId;

use crate::action::ToolCallIntent;
use crate::cognitive_work::{
    ContextItem, CycleStatus, Objective, ObjectiveDomain, ObjectiveStatus,
};
use crate::ids::{
    AgentId, AuditEventId, ComputeRouteId, ContextBundleId, DecisionId, ObjectiveId,
    OrchestratorCycleId, ProposalRequestId, ProposedActionId, WorkspaceId,
};
use crate::observation::{FailureInsightCandidate, FailureInsightCandidateKind};

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
    /// Temporally enriched memory retrieval via Compressed Cognitive Attention.
    CompressedCognitiveAttention,
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

// ─── CycleTrace — structured causal trace with context assembly metadata ────

/// Per-source summary for context assembly metadata in a cycle trace.
///
/// Shows how many items a given source contributed, whether it was available,
/// and a sample item (if any) for operator inspection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextSourceSummary {
    /// The context source name (e.g. "graph_memory", "holographic_memory").
    pub source: String,
    /// Number of items contributed by this source.
    pub item_count: usize,
    /// Whether this source was available.
    pub available: bool,
    /// A preview of the first item key-value pair (if any).
    pub sample_key: Option<String>,
    /// A preview of the first item value (truncated to 120 chars).
    pub sample_value_preview: Option<String>,
}

impl ContextSourceSummary {
    /// Create a new context source summary.
    pub fn new(source: impl Into<String>, item_count: usize, available: bool) -> Self {
        Self {
            source: source.into(),
            item_count,
            available,
            sample_key: None,
            sample_value_preview: None,
        }
    }

    /// Attach a sample item preview.
    pub fn with_sample(mut self, key: String, value: &str) -> Self {
        self.sample_key = Some(key);
        let truncated = if value.len() > 120 {
            format!("{}...", &value[..117])
        } else {
            value.to_owned()
        };
        self.sample_value_preview = Some(truncated);
        self
    }
}

/// Structured causal trace for an orchestrated work cycle.
///
/// A CycleTrace records the full causal chain of one orchestrator cycle:
/// objective → context assembly metadata → compute route → proposal →
/// decision → audit → outcome — with per-source context assembly details.
///
/// Every field is non-authorizing. The trace is evidence for operator
/// supervision and future Failure-to-Insight, not authorization to act.
///
/// # Safety invariants
/// - All context metadata is advisory (non-authorizing)
/// - No execution tokens, approval fields or action authorization
/// - Pure serializable struct — no I/O, no persistence coupling
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CycleTrace {
    /// The orchestrator cycle this trace belongs to.
    pub cycle_id: OrchestratorCycleId,
    /// The objective text that drove this cycle.
    pub objective_text: String,
    /// The objective domain (if classified).
    pub objective_domain: Option<String>,
    /// Per-source context assembly summaries.
    pub context_source_summaries: Vec<ContextSourceSummary>,
    /// Total context items across all sources.
    pub total_context_items: usize,
    /// Sources that were queried but unavailable.
    pub unavailable_sources: Vec<String>,
    /// The compute route label (advisory).
    pub compute_route_label: Option<String>,
    /// The compute route justification (advisory).
    pub compute_route_justification: Option<String>,
    /// Whether local compute was preferred for this cycle (advisory).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compute_local_preferred: Option<bool>,
    /// The action type proposed (if any).
    pub action_type: Option<String>,
    /// The Decision Gate status (if evaluated).
    pub decision_status: Option<String>,
    /// Number of audit events recorded.
    pub audit_event_count: usize,
    /// Whether the cycle went through the Decision Gate.
    pub gate_was_applied: bool,
    /// The final cycle status.
    pub cycle_status: String,
    /// Human-readable summary of the cycle outcome.
    pub summary: String,
    /// Invariant: traces are always non-authorizing.
    pub non_authorizing: bool,
    /// Failure insight candidates detected during this cycle (advisory, non-authorizing).
    #[serde(default)]
    pub failure_insight_candidates: Vec<FailureInsightCandidate>,
    /// Timestamp of the trace.
    pub created_at: DateTime<Utc>,
}

impl CycleTrace {
    /// Create a new CycleTrace with non_authorizing set to true.
    pub fn new(
        cycle_id: OrchestratorCycleId,
        objective_text: impl Into<String>,
        cycle_status: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            cycle_id,
            objective_text: objective_text.into(),
            objective_domain: None,
            context_source_summaries: vec![],
            total_context_items: 0,
            unavailable_sources: vec![],
            compute_route_label: None,
            compute_route_justification: None,
            compute_local_preferred: None,
            action_type: None,
            decision_status: None,
            audit_event_count: 0,
            gate_was_applied: false,
            cycle_status: cycle_status.into(),
            summary: summary.into(),
            non_authorizing: true,
            failure_insight_candidates: vec![],
            created_at: Utc::now(),
        }
    }

    /// Attach failure insight candidates to this trace.
    ///
    /// These candidates are advisory and non-authorizing. They represent
    /// detected signals (weak context, blocked decisions, routing issues)
    /// that may warrant operator attention but do not imply corrective action.
    pub fn with_failure_insight_candidates(
        mut self,
        candidates: Vec<FailureInsightCandidate>,
    ) -> Self {
        self.failure_insight_candidates = candidates;
        self
    }

    /// Scan this trace for failure insight candidates based on the context
    /// assembly, compute route and decision state.
    ///
    /// Returns new candidates that were not previously attached. This method
    /// is read-only and non-authorizing — it merely inspects the existing
    /// metadata and flags patterns that may warrant operator attention.
    ///
    /// Detected signals:
    /// - Zero total context items → ContextAssemblyWeak
    /// - Unavailable context sources → ContextAssemblyWeak (per source)
    /// - Blocked or NeedsReview decision_status → ContextAssemblyWeak
    pub fn detect_failure_candidates(&self) -> Vec<FailureInsightCandidate> {
        let mut candidates: Vec<FailureInsightCandidate> = vec![];

        // 1. Zero total context items
        if self.total_context_items == 0 && !self.unavailable_sources.is_empty() {
            let src_list = self.unavailable_sources.join(", ");
            candidates.push(FailureInsightCandidate {
                kind: FailureInsightCandidateKind::ContextAssemblyWeak,
                summary: format!(
                    "All {} context source(s) unavailable",
                    self.unavailable_sources.len()
                ),
                reason: format!(
                    "All context sources unavailable: {}. Cannot assemble context.",
                    src_list
                ),
                tool_name: String::new(),
                is_positive_signal: false,
            });
        } else if self.total_context_items == 0 {
            candidates.push(FailureInsightCandidate {
                kind: FailureInsightCandidateKind::ContextAssemblyWeak,
                summary: "Context assembly returned zero items".to_owned(),
                reason: "Context assembly returned zero items across all sources.".to_owned(),
                tool_name: String::new(),
                is_positive_signal: false,
            });
        }

        // 2. Each unavailable source (skipped if zero-total was already reported)
        if self.total_context_items > 0 {
            for src in &self.unavailable_sources {
                candidates.push(FailureInsightCandidate {
                    kind: FailureInsightCandidateKind::ContextAssemblyWeak,
                    summary: format!("Source '{}' unavailable", src),
                    reason: format!("Source '{}' was unavailable during context assembly.", src),
                    tool_name: String::new(),
                    is_positive_signal: false,
                });
            }
        }

        // 3. Blocked or NeedsReview decision
        if let Some(ref ds) = self.decision_status {
            let ds_lower = ds.to_lowercase();
            if ds_lower.contains("blocked") || ds_lower.contains("needs_review") {
                candidates.push(FailureInsightCandidate {
                    kind: FailureInsightCandidateKind::ContextAssemblyWeak,
                    summary: format!("Decision Gate: {}", ds),
                    reason: format!(
                        "Decision Gate status '{}' indicates a blocked or stuck cycle.",
                        ds
                    ),
                    tool_name: String::new(),
                    is_positive_signal: false,
                });
            }
        }

        candidates
    }

    /// Return a human-readable formatted trace string.
    pub fn format(&self) -> String {
        let mut lines = vec![];
        lines.push(format!("Cycle:       {}", self.cycle_id));
        lines.push(format!("Objective:   {}", self.objective_text));
        if let Some(ref domain) = self.objective_domain {
            lines.push(format!("Domain:      {}", domain));
        }
        lines.push("Context:".to_string());

        // Per-source breakdown
        for src in &self.context_source_summaries {
            let status = if src.available { "✓" } else { "✗" };
            let sample =
                if let (Some(ref k), Some(ref v)) = (&src.sample_key, &src.sample_value_preview) {
                    format!(" (e.g., {}: \"{}\")", k, v)
                } else {
                    String::new()
                };
            lines.push(format!(
                "  ├─ {} {} items={}{}",
                status, src.source, src.item_count, sample
            ));
        }
        lines.push(format!("  └─ Total: {} items", self.total_context_items));

        if !self.unavailable_sources.is_empty() {
            lines.push(format!(
                "  Unavailable: {}",
                self.unavailable_sources.join(", ")
            ));
        }

        if let Some(ref label) = self.compute_route_label {
            lines.push(format!("Compute:     {}", label));
            if let Some(ref justification) = self.compute_route_justification {
                lines.push(format!("  Why:       {}", justification));
            }
        }

        if let Some(ref action) = self.action_type {
            lines.push(format!("Action:      {}", action));
        }
        if let Some(ref status) = self.decision_status {
            lines.push(format!("Decision:    {}", status));
        }
        lines.push(format!("Audit:       {} events", self.audit_event_count));
        lines.push(format!("Gate:        {}", self.gate_was_applied));
        lines.push(format!("Non-auth:    {}", self.non_authorizing));
        lines.push(format!("Status:      {}", self.cycle_status));
        lines.push(format!("Summary:     {}", self.summary));

        if !self.failure_insight_candidates.is_empty() {
            lines.push("Failure candidates:".to_string());
            for fc in &self.failure_insight_candidates {
                let kind_str = serde_json::to_string(&fc.kind).unwrap_or_default();
                lines.push(format!("  - kind={} summary={}", kind_str, fc.summary));
            }
        }

        lines.join("\n")
    }

    /// Build context source summaries from a ContextBundle.
    pub fn from_context_bundle(bundle: &ContextBundle) -> Vec<ContextSourceSummary> {
        let mut summaries = vec![];

        // GraphMemory
        {
            let count = bundle.graph_memory_items.len();
            let mut summary = ContextSourceSummary::new("graph_memory", count, true);
            if let Some(first) = bundle.graph_memory_items.first() {
                summary = summary.with_sample(first.key.clone(), &first.value);
            }
            summaries.push(summary);
        }

        // HolographicMemory
        {
            let count = bundle.holographic_resonance_items.len();
            let mut summary = ContextSourceSummary::new("holographic_memory", count, true);
            if let Some(first) = bundle.holographic_resonance_items.first() {
                summary = summary.with_sample(first.key.clone(), &first.value);
            }
            summaries.push(summary);
        }

        // ReservoirEcho
        {
            let count = bundle.reservoir_traces.len();
            let mut summary = ContextSourceSummary::new("reservoir_echo", count, true);
            if let Some(first) = bundle.reservoir_traces.first() {
                summary = summary.with_sample(first.key.clone(), &first.value);
            }
            summaries.push(summary);
        }

        summaries
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

/// Analyze a CycleTrace for Failure-to-Insight candidates.
///
/// This is a pure, deterministic analysis that inspects the cycle trace
/// for signals that should produce FailureInsight candidates:
///
/// - Unavailable memory sources → `InsufficientObservability`
/// - Available sources returning zero items → `MissingContext`
/// - Blocked/rejected decisions → `BlockedWithoutExplanation`
/// - Zero context assembled at all → `MissingContext`
/// - Failed/error cycle status → `InsufficientObservability`
///
/// # Safety
///
/// Returned insights are always `status: Proposed`. They are advisory and
/// non-authorizing. No insight may be interpreted as approval, authorization,
/// or execution permission. The function is pure and deterministic — it does
/// no I/O, no persistence writes, no LLM calls, and no tool execution.
pub fn analyze_cycle_trace_for_insights(trace: &CycleTrace) -> Vec<FailureInsight> {
    let mut insights: Vec<FailureInsight> = Vec::new();
    let now = Utc::now();

    // 1. Unavailable sources → InsufficientObservability
    for unavailable in &trace.unavailable_sources {
        insights.push(FailureInsight::new(
            FailureInsightId::new(format!("fi-{}-unavailable-{}", trace.cycle_id, unavailable)),
            FailureClass::InsufficientObservability,
            InsightSeverity::Medium,
            CorrectionTarget::Memory,
            format!(
                "Memory source '{}' was unavailable during cycle {}",
                unavailable, trace.cycle_id
            ),
            format!(
                "The orchestrator queried '{}' but the source was unreachable.",
                unavailable
            ),
            format!(
                "Missing '{}' may reduce proposal quality. Only {} item(s) were assembled.",
                unavailable, trace.total_context_items
            ),
            format!(
                "Investigate why '{}' was unavailable. Check adapter health and configuration.",
                unavailable
            ),
            "Context Assembly / Memory Adapters",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                format!(
                    "Cycle {} reported unavailable source '{}'",
                    trace.cycle_id, unavailable
                ),
            ),
            0.85,
            now,
        ));
    }

    // 2. Available sources with zero items → MissingContext
    for summary in &trace.context_source_summaries {
        if summary.available && summary.item_count == 0 {
            insights.push(FailureInsight::new(
                FailureInsightId::new(format!("fi-{}-empty-{}", trace.cycle_id, summary.source)),
                FailureClass::MissingContext,
                InsightSeverity::Low,
                CorrectionTarget::Memory,
                format!(
                    "Source '{}' returned zero items although reachable",
                    summary.source
                ),
                format!(
                    "Source '{}' was available but produced no context for '{}'",
                    summary.source, trace.objective_text
                ),
                format!(
                    "Empty results from '{}' reduce proposal quality.",
                    summary.source
                ),
                format!(
                    "Check if '{}' has relevant data for this domain.",
                    summary.source
                ),
                "Context Assembly / Memory Adapters",
                DetectionSignal::new(
                    DetectionSignalType::RuntimeObservation,
                    format!(
                        "Cycle {}: source '{}' returned 0 of {} items",
                        trace.cycle_id, summary.source, trace.total_context_items
                    ),
                ),
                0.7,
                now,
            ));
        }
    }

    // 3. Blocked/rejected decision → BlockedWithoutExplanation
    if let Some(ref status) = trace.decision_status {
        if status.contains("Denied") || status.contains("Blocked") || status.contains("Rejected") {
            insights.push(FailureInsight::new(
                FailureInsightId::new(format!("fi-{}-blocked", trace.cycle_id)),
                FailureClass::BlockedWithoutExplanation,
                InsightSeverity::Medium,
                CorrectionTarget::Policy,
                format!(
                    "Decision was blocked during cycle {}: {}",
                    trace.cycle_id, status
                ),
                format!(
                    "Decision status '{}' indicates the action was not approved.",
                    status
                ),
                format!(
                    "Summary: {}. A blocked action with incomplete explanation may indicate a policy gap.",
                    trace.summary
                ),
                format!("Review Decision Gate rules for this action type. Ensure clear rejection reasons."),
                "Decision Gate / Policy",
                DetectionSignal::new(
                    DetectionSignalType::RuntimeObservation,
                    format!(
                        "Cycle {} decision status: {}",
                        trace.cycle_id, status
                    ),
                ),
                0.75,
                now,
            ));
        }
    }

    // 4. No context at all from configured sources → MissingContext (high severity)
    // This fires when the context assembly pipeline ran (has source summaries)
    // but produced zero items total, and NO sources were unavailable.
    // This is the case where sources exist with data but nothing matched.
    if !trace.context_source_summaries.is_empty()
        && trace.total_context_items == 0
        && trace.unavailable_sources.is_empty()
    {
        insights.push(FailureInsight::new(
            FailureInsightId::new(format!("fi-{}-no-context", trace.cycle_id)),
            FailureClass::MissingContext,
            InsightSeverity::High,
            CorrectionTarget::Memory,
            format!(
                "No context was assembled at all for cycle {}",
                trace.cycle_id
            ),
            format!("The orchestrator produced zero context items. No sources were queried."),
            format!("Without context, proposals will be generic."),
            format!("Verify context adapters are registered in MultiAdapterContextAssembler."),
            "Context Assembly / Orchestration",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                format!("Cycle {}: 0 items, 0 source summaries", trace.cycle_id),
            ),
            0.9,
            now,
        ));
    }

    // 5. Failed/error cycle → InsufficientObservability
    if trace.cycle_status.contains("Failed") || trace.cycle_status.contains("Error") {
        insights.push(FailureInsight::new(
            FailureInsightId::new(format!("fi-{}-failed", trace.cycle_id)),
            FailureClass::InsufficientObservability,
            InsightSeverity::Medium,
            CorrectionTarget::Code,
            format!(
                "Cycle {} ended with status '{}'",
                trace.cycle_id, trace.cycle_status
            ),
            format!(
                "The cycle did not complete successfully. Status: '{}'.",
                trace.cycle_status
            ),
            format!("Failed cycles cannot produce useful proposals."),
            format!("Inspect orchestrator error path. Check context, compute route failures."),
            "Orchestrator / Runtime",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                format!("Cycle {} failed: {}", trace.cycle_id, trace.cycle_status),
            ),
            0.8,
            now,
        ));
    }

    // 6. Compute efficiency analysis
    insights.extend(analyze_compute_efficiency(trace));

    insights
}

/// Analyze a CycleTrace for compute efficiency and routing quality signals.
///
/// This pure function inspects the compute route metadata in a CycleTrace
/// and produces FailureInsight candidates when suboptimal routing decisions
/// are detected:
///
/// - Missing compute route → `InsufficientObservability` (routing gap)
/// - Route label contains fallback cues → `WrongComputeChoice` (fallback)
/// - Route justification mentions "No suitable" → `WrongComputeChoice` (no resource)
/// - Route exists but justification is missing → `WrongComputeChoice` (opaque)
/// - Cycle failed despite having a compute route → `WrongComputeChoice` (ineffective)
///
/// # Safety
///
/// All insights are `status: Proposed` — advisory and non-authorizing.
/// No insight may be interpreted as approval, authorization, or execution
/// permission. The function is pure and deterministic.
pub fn analyze_compute_efficiency(trace: &CycleTrace) -> Vec<FailureInsight> {
    let mut insights: Vec<FailureInsight> = Vec::new();
    let now = Utc::now();

    // 1. Missing compute route → InsufficientObservability
    let route_exists = trace.compute_route_label.is_some();
    if !route_exists && trace.cycle_status.contains("Completed") {
        insights.push(FailureInsight::new(
            FailureInsightId::new(format!("fi-{}-missing-route", trace.cycle_id)),
            FailureClass::InsufficientObservability,
            InsightSeverity::Low,
            CorrectionTarget::Code,
            format!("Compute route not recorded for cycle {}", trace.cycle_id),
            format!("The cycle completed but no compute route label was set on the trace."),
            format!("Without route metadata, cost/quality analysis is impossible."),
            format!(
                "Wire Compute Reservoir route selection into the CycleTrace before creating it."
            ),
            "Compute Reservoir / Orchestrator",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                format!("Cycle {}: compute_route_label is None", trace.cycle_id),
            ),
            0.6,
            now,
        ));
    }

    // 2. Route label contains fallback cues → WrongComputeChoice
    if let Some(ref label) = trace.compute_route_label {
        let lower = label.to_lowercase();
        if lower.contains("fallback") || lower.contains("no_suitable") {
            insights.push(FailureInsight::new(
                FailureInsightId::new(format!("fi-{}-fallback-route", trace.cycle_id)),
                FailureClass::WrongComputeChoice,
                InsightSeverity::Low,
                CorrectionTarget::Code,
                format!(
                    "Suboptimal compute route '{}' for cycle {}",
                    label, trace.cycle_id
                ),
                format!(
                    "Route '{}' indicates a fallback or degraded path was taken.",
                    label
                ),
                format!("Suboptimal routing may reduce proposal quality or increase latency."),
                format!("Review Compute Reservoir resource availability and routing heuristics."),
                "Compute Reservoir / Orchestrator",
                DetectionSignal::new(
                    DetectionSignalType::RuntimeObservation,
                    format!("Cycle {}: fallback route '{}'", trace.cycle_id, label),
                ),
                0.5,
                now,
            ));
        }
    }

    // 3. Route justification mentions "No suitable" → WrongComputeChoice
    if let Some(ref justification) = trace.compute_route_justification {
        if justification.to_lowercase().contains("no suitable") {
            insights.push(FailureInsight::new(
                FailureInsightId::new(format!("fi-{}-no-suitable-resource", trace.cycle_id)),
                FailureClass::WrongComputeChoice,
                InsightSeverity::Medium,
                CorrectionTarget::Code,
                format!(
                    "No suitable compute resource for cycle {}",
                    trace.cycle_id
                ),
                format!("Justification: '{}'", justification),
                format!("No suitable resource may indicate capacity or configuration issues."),
                format!("Check available models, local/cloud provider status, and Compute Reservoir configuration."),
                "Compute Reservoir / Infrastructure",
                DetectionSignal::new(
                    DetectionSignalType::RuntimeObservation,
                    format!("Cycle {}: 'no suitable' in justification", trace.cycle_id),
                ),
                0.7,
                now,
            ));
        }
    }

    // 4. Route exists but justification is missing → WrongComputeChoice (opaque)
    if route_exists && trace.compute_route_justification.is_none() {
        insights.push(FailureInsight::new(
            FailureInsightId::new(format!("fi-{}-no-justification", trace.cycle_id)),
            FailureClass::WrongComputeChoice,
            InsightSeverity::Low,
            CorrectionTarget::Code,
            format!(
                "Compute route '{}' has no justification for cycle {}",
                trace.compute_route_label.as_deref().unwrap_or("?"),
                trace.cycle_id
            ),
            format!("A compute route label was recorded but no justification was provided."),
            format!("Without justification, operators cannot assess the routing decision."),
            format!("Ensure Compute Reservoir provides a justification when selecting a route."),
            "Compute Reservoir / Orchestrator",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                format!(
                    "Cycle {}: route label present, justification missing",
                    trace.cycle_id
                ),
            ),
            0.4,
            now,
        ));
    }

    // 5. Cycle failed despite having a compute route → WrongComputeChoice (ineffective)
    if route_exists
        && (trace.cycle_status.contains("Failed") || trace.cycle_status.contains("Error"))
    {
        insights.push(FailureInsight::new(
            FailureInsightId::new(format!("fi-{}-failed-with-route", trace.cycle_id)),
            FailureClass::WrongComputeChoice,
            InsightSeverity::Medium,
            CorrectionTarget::Code,
            format!(
                "Compute route '{}' did not prevent failure for cycle {}",
                trace.compute_route_label.as_deref().unwrap_or("?"),
                trace.cycle_id
            ),
            format!("Cycle failed despite being assigned a compute route with justification."),
            format!("The chosen compute resource may be unsuitable for this task type."),
            format!("Review compute route selection criteria and cycle failure logs."),
            "Compute Reservoir / Orchestrator",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                format!(
                    "Cycle {}: route '{}' + status '{}'",
                    trace.cycle_id,
                    trace.compute_route_label.as_deref().unwrap_or("?"),
                    trace.cycle_status
                ),
            ),
            0.65,
            now,
        ));
    }

    insights
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
        let _now = Utc::now();
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

    // ─── CycleTrace tests ───────────────────────────────────────────────

    #[test]
    fn cycle_trace_new_is_non_authorizing() {
        let trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-1"),
            "Test objective",
            "Completed",
            "Test summary",
        );
        assert!(trace.non_authorizing);
        assert_eq!(trace.objective_text, "Test objective");
        assert_eq!(trace.cycle_status, "Completed");
        assert_eq!(trace.summary, "Test summary");
        assert!(trace.context_source_summaries.is_empty());
        assert_eq!(trace.total_context_items, 0);
    }

    #[test]
    fn cycle_trace_serializes_and_deserializes() {
        let trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-42"),
            "Serialize test",
            "Completed",
            "Done",
        );

        let json = serde_json::to_value(&trace).expect("serialize");
        assert_eq!(json["objective_text"], "Serialize test");
        assert_eq!(json["non_authorizing"], true);
        assert!(json.get("approved").is_none());

        let decoded: CycleTrace = serde_json::from_value(json).expect("deserialize");
        assert_eq!(decoded.cycle_id, trace.cycle_id);
        assert!(decoded.non_authorizing);
    }

    #[test]
    fn cycle_trace_format_shows_context_sources() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-1"),
            "Format test",
            "Completed",
            "Done",
        );

        trace.context_source_summaries = vec![
            ContextSourceSummary::new("graph_memory", 2, true)
                .with_sample("key1".to_owned(), "value1"),
            ContextSourceSummary::new("holographic_memory", 0, false),
        ];
        trace.total_context_items = 2;
        trace.compute_route_label = Some("local".to_owned());

        let output = trace.format();
        assert!(output.contains("oc-1"));
        assert!(output.contains("Format test"));
        assert!(output.contains("graph_memory"));
        assert!(output.contains("holographic_memory"));
        assert!(output.contains("items=2"));
        assert!(output.contains("items=0"));
        assert!(output.contains("Total: 2 items"));
        assert!(output.contains("Compute:     local"));
        assert!(output.contains("Non-auth:    true"));
    }

    #[test]
    fn context_source_summary_with_sample_truncates_long_values() {
        let long_value = "a".repeat(200);
        let summary = ContextSourceSummary::new("test", 1, true)
            .with_sample("long_key".to_owned(), &long_value);

        assert_eq!(summary.sample_key, Some("long_key".to_owned()));
        let preview = summary.sample_value_preview.expect("should have preview");
        assert!(preview.len() <= 123); // 120 + "..."
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn from_context_bundle_maps_all_sources() {
        let bundle = ContextBundle::new(
            ContextBundleId::new("cb-1"),
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
        )
        .with_graph_memory(vec![ContextItem {
            key: "fact_1".to_owned(),
            value: "Value 1".to_owned(),
            source: "graph_memory".to_owned(),
        }])
        .with_holographic_resonance(vec![ContextItem {
            key: "trace_1".to_owned(),
            value: "Pattern match".to_owned(),
            source: "holographic_memory".to_owned(),
        }])
        .with_reservoir_traces(vec![]);

        let summaries = CycleTrace::from_context_bundle(&bundle);
        assert_eq!(summaries.len(), 3);

        let gm = summaries
            .iter()
            .find(|s| s.source == "graph_memory")
            .unwrap();
        assert_eq!(gm.item_count, 1);
        assert_eq!(gm.sample_key, Some("fact_1".to_owned()));

        let hm = summaries
            .iter()
            .find(|s| s.source == "holographic_memory")
            .unwrap();
        assert_eq!(hm.item_count, 1);
        assert_eq!(hm.sample_value_preview, Some("Pattern match".to_owned()));

        let re = summaries
            .iter()
            .find(|s| s.source == "reservoir_echo")
            .unwrap();
        assert_eq!(re.item_count, 0);
        assert!(re.sample_key.is_none());
        assert!(re.sample_value_preview.is_none());
    }

    // ─── detect_failure_candidates tests ───────────────────────────────────

    #[test]
    fn detect_no_candidates_when_context_is_healthy() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c1"),
            "Healthy objective",
            "Completed",
            "All good",
        );
        trace.total_context_items = 5;

        let candidates = trace.detect_failure_candidates();
        assert!(
            candidates.is_empty(),
            "healthy trace should not produce any candidates, got: {:?}",
            candidates
        );
    }

    #[test]
    fn detect_candidates_when_all_sources_unavailable() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c2"),
            "Test objective",
            "NeedsReview",
            "No context available",
        );
        trace.total_context_items = 0;
        trace.unavailable_sources =
            vec!["graph_memory".to_owned(), "holographic_memory".to_owned()];

        let candidates = trace.detect_failure_candidates();
        assert_eq!(
            candidates.len(),
            1,
            "should produce 1 candidate for all-unavailable"
        );
        assert_eq!(
            candidates[0].kind,
            FailureInsightCandidateKind::ContextAssemblyWeak
        );
        assert!(
            candidates[0].summary.contains("unavailable"),
            "summary should mention unavailable sources: {}",
            candidates[0].summary
        );
    }

    #[test]
    fn detect_candidates_when_zero_context_items() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c3"),
            "Zero items",
            "Completed",
            "No items retrieved",
        );
        trace.total_context_items = 0;

        let candidates = trace.detect_failure_candidates();
        assert_eq!(
            candidates.len(),
            1,
            "should produce 1 candidate for zero items"
        );
        assert_eq!(
            candidates[0].kind,
            FailureInsightCandidateKind::ContextAssemblyWeak
        );
        assert!(
            candidates[0].summary.contains("zero"),
            "summary should mention zero items"
        );
    }

    // ─── analyze_cycle_trace_for_insights tests ────────────────────────

    #[test]
    fn analyze_insights_empty_trace_yields_no_insights() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-empty"),
            "Test objective",
            "Completed",
            "Cycle completed successfully",
        );
        trace.compute_route_label = Some("local_deterministic".to_owned());
        trace.compute_route_justification = Some("Default route".to_owned());
        trace.compute_local_preferred = Some(true);
        let insights = analyze_cycle_trace_for_insights(&trace);
        assert!(
            insights.is_empty(),
            "Empty trace should yield 0 insights, got {}",
            insights.len()
        );
    }

    #[test]
    fn detect_candidates_when_decision_blocked() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c4"),
            "Blocked objective",
            "NeedsReview",
            "Blocked by Decision Gate",
        );
        trace.total_context_items = 3;
        trace.decision_status = Some("Blocked".to_owned());

        let candidates = trace.detect_failure_candidates();
        assert!(
            !candidates.is_empty(),
            "blocked decision should produce candidates"
        );
        assert!(
            candidates.iter().any(|c| c.summary.contains("Blocked")),
            "at least one candidate should mention Blocked: {:?}",
            candidates
        );
    }

    #[test]
    fn detect_candidates_serialization_round_trip_with_candidates() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c5"),
            "Serialization test",
            "Completed",
            "Testing round-trip with candidates",
        );
        trace.total_context_items = 0;
        let candidates = trace.detect_failure_candidates();
        trace.failure_insight_candidates = candidates;

        let json = serde_json::to_value(&trace).expect("should serialize");
        let decoded: CycleTrace = serde_json::from_value(json).expect("should deserialize");

        assert!(
            !decoded.failure_insight_candidates.is_empty(),
            "candidates should survive round-trip serialization"
        );
        assert_eq!(
            decoded.failure_insight_candidates[0].kind,
            FailureInsightCandidateKind::ContextAssemblyWeak
        );
    }

    #[test]
    fn detect_no_candidates_for_empty_trace_with_items() {
        // A trace with context items, no unavailable sources,
        // and a healthy decision should produce zero candidates.
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c6"),
            "Healthy",
            "Completed",
            "Normal cycle",
        );
        trace.total_context_items = 10;
        trace.decision_status = Some("Approved".to_owned());

        let candidates = trace.detect_failure_candidates();
        assert!(
            candidates.is_empty(),
            "healthy trace should be clean, got {:?}",
            candidates
        );
    }

    #[test]
    fn detect_candidates_with_partially_unavailable_sources() {
        // When some sources have items but others are unavailable,
        // we should get per-source candidates for the unavailable ones.
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("c7"),
            "Partial availability test",
            "Completed",
            "Some sources unavailable",
        );
        trace.total_context_items = 3;
        trace.unavailable_sources = vec!["holographic_memory".to_owned()];

        let candidates = trace.detect_failure_candidates();
        assert_eq!(
            candidates.len(),
            1,
            "should produce 1 candidate for partial unavailability"
        );
        assert!(
            candidates[0].summary.contains("holographic_memory"),
            "should mention the unavailable source: {}",
            candidates[0].summary
        );
    }

    // ─── analyze_cycle_trace_for_insights tests (P3-15) ───────────────

    #[test]
    fn analyze_insights_unavailable_source_yields_insight() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-unavail"),
            "Check memory sources",
            "Completed",
            "One source was unavailable",
        );
        trace.unavailable_sources.push("graph_memory".to_owned());
        trace.compute_route_label = Some("local_deterministic".to_owned());
        trace.compute_route_justification = Some("Default route".to_owned());

        let insights = analyze_cycle_trace_for_insights(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(
            insights[0].failure_class,
            FailureClass::InsufficientObservability
        );
        assert_eq!(
            insights[0].status,
            crate::failure_insight::InsightStatus::Proposed
        );
        assert!(insights[0].summary.contains("unavailable"));
        assert!(insights[0].summary.contains("graph_memory"));
    }

    #[test]
    fn analyze_insights_available_source_zero_items_yields_insight() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-empty-source"),
            "Find relevant facts",
            "Completed",
            "Source returned nothing",
        );
        trace.context_source_summaries =
            vec![ContextSourceSummary::new("holographic_memory", 0, true)];
        trace.compute_route_label = Some("local_ollama_qwen3.5".to_owned());
        trace.compute_route_justification = Some("Default route".to_owned());

        let insights = analyze_cycle_trace_for_insights(&trace);
        // 2 insights: check #2 (empty source: MissingContext Low) + check #4 (no context at all: MissingContext High)
        assert_eq!(insights.len(), 2);
        assert!(insights
            .iter()
            .any(|i| i.failure_class == FailureClass::MissingContext));
        assert!(insights
            .iter()
            .any(|i| i.summary.contains("holographic_memory")));
    }

    #[test]
    fn analyze_insights_blocked_decision_yields_insight() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-blocked"),
            "Delete production data",
            "Completed",
            "Action was blocked by Decision Gate",
        );
        trace.decision_status = Some("Denied: requires human approval".to_owned());
        // Add context items so check #4 doesn't fire
        trace.context_source_summaries = vec![ContextSourceSummary::new("graph_memory", 3, true)];
        trace.total_context_items = 3;
        trace.compute_route_label = Some("local_deterministic".to_owned());
        trace.compute_route_justification = Some("Default route".to_owned());

        let insights = analyze_cycle_trace_for_insights(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(
            insights[0].failure_class,
            FailureClass::BlockedWithoutExplanation
        );
        assert!(insights[0].summary.contains("blocked"));
        assert!(insights[0].summary.contains("oc-blocked"));
    }

    #[test]
    fn analyze_insights_no_context_at_all_yields_high_severity() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-noctx"),
            "Something",
            "Completed",
            "No context assembled",
        );
        // Source exists but returned 0 items — triggers check #4 (high severity no-context)
        // and check #2 (missing context, low severity)
        trace.context_source_summaries = vec![ContextSourceSummary::new("graph_memory", 0, true)];
        trace.compute_route_label = Some("local_deterministic".to_owned());
        trace.compute_route_justification = Some("Default route".to_owned());
        let insights = analyze_cycle_trace_for_insights(&trace);
        // 2 insights: check #2 (low, empty source) + check #4 (high, no context at all)
        assert_eq!(insights.len(), 2);
        // At least one has High severity (check #4)
        assert!(insights.iter().any(|i| i.severity == InsightSeverity::High));
        assert!(insights
            .iter()
            .any(|i| i.failure_class == FailureClass::MissingContext));
    }

    #[test]
    fn analyze_insights_failed_cycle_yields_insight() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-failed"),
            "Do complex task",
            "Failed",
            "Cycle failed during context assembly",
        );
        trace.context_source_summaries = vec![ContextSourceSummary::new("graph_memory", 3, true)];
        trace.total_context_items = 3;

        let insights = analyze_cycle_trace_for_insights(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(
            insights[0].failure_class,
            FailureClass::InsufficientObservability
        );
        assert_eq!(
            insights[0].status,
            crate::failure_insight::InsightStatus::Proposed
        );
    }

    #[test]
    fn analyze_insights_all_insights_are_non_authorizing() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-all"),
            "Test all signals",
            "Failed",
            "Multiple issues detected",
        );
        trace.unavailable_sources.push("graph_memory".to_owned());
        trace.context_source_summaries = vec![
            ContextSourceSummary::new("reservoir_echo", 0, true),
            ContextSourceSummary::new("holographic_memory", 2, true),
        ];
        trace.total_context_items = 2;
        trace.decision_status = Some("Blocked".to_owned());

        let insights = analyze_cycle_trace_for_insights(&trace);
        // Expect: unavailable (1) + empty source (1) + blocked (1) + failed (1) = 4
        assert_eq!(insights.len(), 4);

        // Every insight must be Proposed (not auto-applied)
        for insight in &insights {
            assert_eq!(
                insight.status,
                crate::failure_insight::InsightStatus::Proposed,
                "Insight '{}' must start as Proposed, not auto-applied",
                insight.id
            );
            // Verify no authorization fields exist in JSON serialization
            let json = serde_json::to_value(insight).expect("serialize insight");
            assert!(json.get("approved").is_none());
            assert!(json.get("authorized").is_none());
            assert!(json.get("executed").is_none());
        }
    }

    // ─── analyze_compute_efficiency tests (P3-16) ──────────────────────

    #[test]
    fn compute_efficiency_missing_route_on_completed_cycle() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-no-route-eff"),
            "Test",
            "Completed",
            "No route recorded",
        );
        let insights = analyze_compute_efficiency(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(
            insights[0].failure_class,
            FailureClass::InsufficientObservability
        );
        assert_eq!(
            insights[0].status,
            crate::failure_insight::InsightStatus::Proposed
        );
        assert!(insights[0].summary.contains("not recorded"));
    }

    #[test]
    fn compute_efficiency_fallback_route_detected() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-fallback"),
            "Test",
            "Completed",
            "Fallback used",
        );
        trace.compute_route_label = Some("fallback_local_qwen3.5".to_owned());
        trace.compute_route_justification = Some("Default fallback".to_owned());
        let insights = analyze_compute_efficiency(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].failure_class, FailureClass::WrongComputeChoice);
        assert!(insights[0].summary.contains("Suboptimal"));
    }

    #[test]
    fn compute_efficiency_unjustified_route_detected() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-unjustified"),
            "Test",
            "Completed",
            "No justification",
        );
        trace.compute_route_label = Some("local_deterministic".to_owned());
        let insights = analyze_compute_efficiency(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].failure_class, FailureClass::WrongComputeChoice);
        assert!(insights[0].summary.contains("no justification"));
    }

    #[test]
    fn compute_efficiency_no_suitable_resource_detected() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-no-res"),
            "Test",
            "Completed",
            "No resource",
        );
        trace.compute_route_label = Some("no_suitable_resource".to_owned());
        trace.compute_route_justification = Some("No suitable compute available".to_owned());
        let insights = analyze_compute_efficiency(&trace);
        assert_eq!(insights.len(), 2);
        assert!(insights
            .iter()
            .any(|i| i.failure_class == FailureClass::WrongComputeChoice));
        assert!(insights.iter().any(|i| i.summary.contains("Suboptimal")));
        assert!(insights
            .iter()
            .any(|i| i.summary.contains("No suitable compute")));
    }

    #[test]
    fn compute_efficiency_failed_cycle_with_route() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-fail-route"),
            "Test",
            "Failed",
            "Cycle failed despite route",
        );
        trace.compute_route_label = Some("cloud_gpt4".to_owned());
        trace.compute_route_justification = Some("Chose strong model".to_owned());
        let insights = analyze_compute_efficiency(&trace);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].failure_class, FailureClass::WrongComputeChoice);
        assert!(insights[0].summary.contains("did not prevent"));
    }

    #[test]
    fn compute_efficiency_all_insights_are_non_authorizing() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-all-eff"),
            "Test",
            "Failed",
            "Multiple compute issues",
        );
        trace.compute_route_label = Some("fallback_ollama".to_owned());
        trace.compute_route_justification = Some("No suitable model, falling back".to_owned());
        let insights = analyze_compute_efficiency(&trace);
        assert!(!insights.is_empty(), "Should have insights");
        for insight in &insights {
            assert_eq!(
                insight.status,
                crate::failure_insight::InsightStatus::Proposed,
                "Insight '{}' must start as Proposed",
                insight.id
            );
            let json = serde_json::to_value(insight).expect("serialize");
            assert!(json.get("approved").is_none());
            assert!(json.get("authorized").is_none());
            assert!(json.get("executed").is_none());
        }
    }

    #[test]
    fn compute_efficiency_no_signals_for_well_configured_completed_cycle() {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-well-configured"),
            "Test",
            "Completed",
            "Well configured cycle",
        );
        trace.compute_route_label = Some("local_ollama_qwen3.5".to_owned());
        trace.compute_route_justification = Some("Local model for sensitive data".to_owned());
        let insights = analyze_compute_efficiency(&trace);
        assert!(
            insights.is_empty(),
            "Well-configured cycle should have no compute efficiency insights"
        );
    }
}

// ─── MemoryQueryRequest ──────────────────────────────────────────────────

/// Request to query memory sources for advisory context.
///
/// The orchestrator sends this request to the context assembly pipeline.
/// Each memory source adapter (GraphMemory, HolographicMemory, etc.) processes
/// the request and returns a MemoryQueryResponse.
///
/// Pure domain: no I/O, no execution, no authorization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryQueryRequest {
    /// The orchestrator cycle ID.
    pub cycle_id: OrchestratorCycleId,
    /// The objective being processed.
    pub objective_id: ObjectiveId,
    /// The objective text (used as query for all adapters).
    pub objective_text: String,
    /// The workspace to scope queries within.
    pub workspace_id: WorkspaceId,
    /// Which sources to query (default: all available).
    pub requested_sources: Vec<ContextSource>,
    /// Maximum items per source (prevents overstuffing).
    pub max_items_per_source: usize,
    /// Optional compute route label hint for compute-aware context assembly.
    /// When set, adapters may use this signal to prioritize/filter sources
    /// relevant to the selected resource type (e.g. local-small prefers
    /// Reservoir Echo traces, cloud-strong prefers Graph Memory facts).
    pub compute_route_label: Option<String>,
    /// Optional local preference hint from the compute route result.
    /// Adapters may use this to favor local-memory sources (Reservoir Echo,
    /// local tool results) when true.
    pub local_preferred: Option<bool>,
    /// Timestamp of the request.
    pub created_at: DateTime<Utc>,
}

impl MemoryQueryRequest {
    /// Create a new MemoryQueryRequest requesting all available sources.
    pub fn new(
        cycle_id: OrchestratorCycleId,
        objective_id: ObjectiveId,
        objective_text: impl Into<String>,
        workspace_id: WorkspaceId,
    ) -> Self {
        Self {
            cycle_id,
            objective_id,
            objective_text: objective_text.into(),
            workspace_id,
            requested_sources: vec![
                ContextSource::GraphMemory,
                ContextSource::HolographicMemory,
                ContextSource::ReservoirEcho,
                ContextSource::ToolRuntime,
                ContextSource::WorkingMemory,
                ContextSource::CompressedCognitiveAttention,
            ],
            max_items_per_source: 10,
            compute_route_label: None,
            local_preferred: None,
            created_at: Utc::now(),
        }
    }

    /// Restrict to specific sources.
    pub fn with_sources(mut self, sources: Vec<ContextSource>) -> Self {
        self.requested_sources = sources;
        self
    }

    /// Override the max items per source.
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items_per_source = max;
        self
    }

    /// Attach compute route hints for compute-aware context assembly.
    ///
    /// Adapters may use these hints to prioritize sources relevant to the
    /// selected resource type (e.g. local-small prefers Reservoir Echo traces,
    /// cloud-strong prefers Graph Memory facts).
    ///
    /// # Safety
    ///
    /// These hints are advisory only. No adapter may treat them as
    /// authorization, approval, or execution instructions.
    pub fn with_compute_route(
        mut self,
        route_label: Option<impl Into<String>>,
        local_preferred: Option<bool>,
    ) -> Self {
        self.compute_route_label = route_label.map(|s| s.into());
        self.local_preferred = local_preferred;
        self
    }
}

// ─── MemoryQueryResponse ─────────────────────────────────────────────────

/// Advisory response from a memory source adapter.
///
/// Each source adapter returns one response containing the context items
/// it retrieved. All items are advisory and non-authorizing — no item in
/// this response may approve, authorize or execute an action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryQueryResponse {
    /// The source this response came from.
    pub source: ContextSource,
    /// Advisory context items retrieved from this source.
    pub items: Vec<ContextItem>,
    /// Whether the source was available for querying.
    pub available: bool,
    /// Human-readable explanation of the query result.
    pub explanation: String,
}

impl MemoryQueryResponse {
    /// Create a new MemoryQueryResponse.
    pub fn new(source: ContextSource) -> Self {
        let explanation = format!("No items retrieved from {:?}", &source);
        Self {
            source,
            items: vec![],
            available: true,
            explanation,
        }
    }

    /// Add items to this response.
    pub fn with_items(mut self, items: Vec<ContextItem>) -> Self {
        let count = items.len();
        self.items = items;
        self.explanation = format!("Retrieved {} item(s) from {:?}", count, self.source);
        self
    }

    /// Mark this source as unavailable.
    pub fn with_unavailable(mut self) -> Self {
        self.available = false;
        self.explanation = format!("{:?} is unavailable", self.source);
        self
    }

    /// Return true if any items are present.
    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }

    /// Return the number of items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}
