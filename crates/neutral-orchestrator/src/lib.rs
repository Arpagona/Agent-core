//! Neutral Orchestrator V0 deterministic loop skeleton.
//!
//! This crate implements a deterministic in-process skeleton that wires
//! existing ARPAGONA bricks into a governed work cycle:
//!
//! ```text
//! ObjectiveInput
//!   -> ContextBundle (synthetic/advisory)
//!   -> ComputeRouteResult (deterministic)
//!   -> ProposalRequest
//!   -> ProposedAction
//!   -> Decision Gate
//!   -> AuditEvent
//!   -> OrchestratorOutcome (all IDs linked)
//! ```
//!
//! Key invariants:
//! - Deterministic, in-process, no external effects
//! - No scheduler, no autonomy, no approval semantics
//! - Every outcome is non_authorizing
//! - Context, compute route and proposal are advisory only
//! - Decision Gate outcome and audit events carry the actual governance state

use arpagona_agent_core::action::{ActionType, ProposedAction, ProposedActionStatus};
use arpagona_agent_core::audit::{ActorRef, AuditEvent};
use arpagona_agent_core::cognitive_work::CycleStatus;
use arpagona_agent_core::decision::Decision;
use arpagona_agent_core::ids::{
    AgentId, AuditEventId, ComputeRouteId, ContextBundleId, ProposedActionId, WorkspaceId,
};
#[cfg(test)]
use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId};
#[cfg(test)]
use arpagona_agent_core::orchestrator::ComputeRouteRequest;
use arpagona_agent_core::orchestrator::{
    ComputeRouteResult, ContextBundle, ContextSource, CycleTrace, ObjectiveInput,
    OrchestratorOutcome, ProposalRequest,
};
use arpagona_agent_core::permission::Permission;
use arpagona_agent_core::risk::RiskLevel;
use chrono::{DateTime, Utc};

pub mod context_assembler;
pub mod holographic_memory_adapter;
pub mod tool_runtime_adapter;

pub use context_assembler::{ContextAssembler, SimulatedContextAssembler};
pub use holographic_memory_adapter::HolographicMemoryAdapter;
pub use tool_runtime_adapter::ToolRuntimeAdapter;

// ─── OrchestratorCycle ──────────────────────────────────────────────────────

/// A single orchestrated work cycle.
///
/// This struct holds the assembled state for one full cycle through the
/// orchestrator skeleton. It is created by `OrchestratorEngine::run_cycle`.
///
/// All fields are advisory where applicable. The Decision Gate outcome and
/// audit events carry the actual governance state.
#[derive(Clone, Debug)]
pub struct OrchestratorCycle {
    /// The objective that drove this cycle.
    pub objective_input: ObjectiveInput,
    /// The advisory context bundle.
    pub context_bundle: ContextBundle,
    /// The advisory compute route result.
    pub compute_route_result: ComputeRouteResult,
    /// The proposal request sent to the agent.
    pub proposal_request: ProposalRequest,
    /// The proposed action that was evaluated (if any).
    pub proposed_action: Option<ProposedAction>,
    /// The Decision Gate outcome (if evaluated).
    pub decision: Option<Decision>,
    /// Audit events recorded during this cycle.
    pub audit_events: Vec<AuditEvent>,
    /// The final outcome with all IDs linked.
    pub outcome: OrchestratorOutcome,
}

impl OrchestratorCycle {
    /// Return the canonical causal trace as a human-readable string.
    ///
    /// This method produces a rich trace that includes per-source context
    /// assembly metadata (item count per source, sample items, unavailable
    /// sources) in addition to the core causal chain.
    pub fn causal_trace(&self) -> String {
        self.to_cycle_trace().format()
    }

    /// Convert this cycle into a structured, serializable CycleTrace.
    ///
    /// The CycleTrace captures the full causal chain with per-source context
    /// assembly metadata: which sources contributed items, how many, sample
    /// items, unavailable sources, compute route, action, decision, audit
    /// events, and outcome.
    ///
    /// # Safety
    ///
    /// The returned trace is always non-authorizing. No field in the trace
    /// may be interpreted as approval, authorization, or execution permission.
    pub fn to_cycle_trace(&self) -> CycleTrace {
        let mut trace = CycleTrace::new(
            self.outcome.cycle_id.clone(),
            &self.objective_input.text,
            format!("{:?}", self.outcome.cycle_status),
            &self.outcome.summary,
        );

        // Context assembly metadata — per-source breakdown
        let source_summaries = CycleTrace::from_context_bundle(&self.context_bundle);
        trace.context_source_summaries = source_summaries;
        trace.total_context_items = self.context_bundle.total_items();

        // Unavailable sources
        for src in &self.context_bundle.unavailable_sources {
            trace.unavailable_sources.push(format!("{:?}", src));
        }

        // Domain
        if let Some(ref domain) = self.objective_input.domain_hint {
            trace.objective_domain = Some(format!("{:?}", domain));
        }

        // Compute route
        trace.compute_route_label = Some(self.compute_route_result.selected_route_label.clone());
        trace.compute_route_justification = Some(self.compute_route_result.justification.clone());

        // Action
        if let Some(ref action) = self.proposed_action {
            trace.action_type = Some(format!("{:?}", action.action_type));
        }

        // Decision
        if let Some(ref decision) = self.decision {
            trace.decision_status = Some(format!("{:?}", decision.status));
        }

        // Audit
        trace.audit_event_count = self.outcome.audit_event_count();
        trace.gate_was_applied = self.outcome.gate_was_applied;

        trace
    }
}

// ─── OrchestratorEngine ─────────────────────────────────────────────────────

/// Deterministic orchestrator engine that wires existing bricks into a governed
/// work cycle.
///
/// This engine is:
/// - Deterministic: given the same input, it produces the same output
/// - In-process: no I/O, no LLM calls, no persistence, no external effects
/// - Non-authorizing: all outcomes are advisory; the Decision Gate carries
///   the actual governance state
///
/// Usage:
/// ```ignore
/// let engine = OrchestratorEngine::new();
/// let cycle = engine.run_cycle(input, &[Permission::ReadDocument]);
/// println!("{}", cycle.causal_trace());
/// ```
pub struct OrchestratorEngine {
    /// Human-readable label for the compute route (overridable).
    compute_route_label: String,
    /// Whether local compute is preferred.
    local_preferred: bool,
    /// The base compute route justification.
    compute_route_justification: String,
    /// Pluggable context assembler for gathering advisory context from memory sources.
    /// Defaults to SimulatedContextAssembler (no-op, deterministic, zero I/O).
    context_assembler: Box<dyn ContextAssembler>,
}

impl Default for OrchestratorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestratorEngine {
    /// Create a new orchestrator engine with default settings.
    ///
    /// Default compute route: "local_deterministic" with local-first preference.
    /// Default context assembler: SimulatedContextAssembler (no-op, zero I/O).
    pub fn new() -> Self {
        Self {
            compute_route_label: "local_deterministic".to_owned(),
            local_preferred: true,
            compute_route_justification: "Local deterministic compute selected by default for V0 deterministic loop skeleton."
                .to_owned(),
            context_assembler: Box::new(SimulatedContextAssembler::new()),
        }
    }

    /// Configure the compute route label.
    pub fn with_compute_route_label(mut self, label: impl Into<String>) -> Self {
        self.compute_route_label = label.into();
        self
    }

    /// Configure whether local compute is preferred.
    pub fn with_local_preferred(mut self, preferred: bool) -> Self {
        self.local_preferred = preferred;
        self
    }

    /// Configure the compute route justification.
    pub fn with_compute_route_justification(mut self, justification: impl Into<String>) -> Self {
        self.compute_route_justification = justification.into();
        self
    }

    /// Configure a custom ContextAssembler for memory-aware context assembly.
    ///
    /// Use this to plug in real memory adapters (GraphMemoryAdapter, etc.)
    /// once they are implemented. Defaults to SimulatedContextAssembler.
    ///
    /// # Safety
    ///
    /// The provided assembler must return advisory items only. No response
    /// may contain approval, authorization or execution tokens.
    pub fn with_context_assembler(mut self, assembler: Box<dyn ContextAssembler>) -> Self {
        self.context_assembler = assembler;
        self
    }

    /// Run a complete deterministic cycle.
    ///
    /// Steps:
    /// 1. Validate the input (rejects empty objective text)
    /// 2. Assemble a synthetic advisory `ContextBundle`
    /// 3. Create a deterministic `ComputeRouteResult`
    /// 4. Create a `ProposalRequest`
    /// 5. Create a `ProposedAction` for simulation
    /// 6. Run through Decision Gate
    /// 7. Record audit events
    /// 8. Return `OrchestratorCycle` with all links
    pub fn run_cycle(
        &self,
        input: ObjectiveInput,
        permissions: &[Permission],
    ) -> Result<OrchestratorCycle, OrchestratorError> {
        let now = Utc::now();

        // Step 1: Validate input
        if input.text.trim().is_empty() {
            return Err(OrchestratorError::EmptyObjective);
        }

        let objective = input.to_objective();

        // Step 2: Assemble synthetic advisory ContextBundle
        let context_bundle = self.assemble_context(&input, &objective, now);

        // Step 3: Create ComputeRouteResult (deterministic)
        let compute_route = self.create_compute_route(&input, &objective, &context_bundle, now);

        // Step 4: Create ProposalRequest
        let proposal_request =
            self.create_proposal_request(&input, &objective, &context_bundle, &compute_route);

        // Step 5: Create a ProposedAction for simulation
        // The skeleton always proposes a ReadDocument action at the configured
        // risk level. This is deterministic and in-process — no real agent
        // proposal is involved.
        let proposed_action = self.create_simulation_action(&input, &objective, now);

        // Step 6: Run through Decision Gate
        let (decision, audit_event) =
            self.evaluate_through_gate(&proposed_action, permissions, &input.workspace_id, now);

        let _gate_was_applied = matches!(
            decision.status,
            arpagona_agent_core::decision::DecisionStatus::Approved
                | arpagona_agent_core::decision::DecisionStatus::Blocked
                | arpagona_agent_core::decision::DecisionStatus::RequiresOverride
                | arpagona_agent_core::decision::DecisionStatus::NeedsHumanApproval
        );

        // Step 7: Build OrchestratorOutcome
        let cycle_status = match decision.status {
            arpagona_agent_core::decision::DecisionStatus::Approved
            | arpagona_agent_core::decision::DecisionStatus::ApprovedByOverride => {
                CycleStatus::Completed
            }
            arpagona_agent_core::decision::DecisionStatus::Blocked => CycleStatus::NeedsReview,
            arpagona_agent_core::decision::DecisionStatus::RequiresOverride => {
                CycleStatus::NeedsReview
            }
            arpagona_agent_core::decision::DecisionStatus::NeedsHumanApproval => {
                CycleStatus::NeedsReview
            }
        };

        let outcome = OrchestratorOutcome::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            context_bundle.id.clone(),
            format!(
                "Deterministic cycle completed — action {:?} evaluated by Decision Gate as {:?}",
                proposed_action.action_type, decision.status
            ),
            cycle_status,
        )
        .with_compute_route(compute_route.id.clone())
        .with_proposed_action(proposed_action.id.clone())
        .with_decision(decision.id.clone())
        .with_audit_events(vec![audit_event.id.clone()]);

        // Step 8: Return the full cycle
        Ok(OrchestratorCycle {
            objective_input: input,
            context_bundle,
            compute_route_result: compute_route,
            proposal_request,
            proposed_action: Some(proposed_action),
            decision: Some(decision),
            audit_events: vec![audit_event],
            outcome,
        })
    }

    // ─── Step implementations ──────────────────────────────────────────────

    /// Assemble an advisory ContextBundle using the configured ContextAssembler.
    ///
    /// The assembler queries all configured memory sources (Graph Memory,
    /// Holographic Memory, Reservoir Echo, etc.) and returns advisory context
    /// items. The `context_hint` from the ObjectiveInput is always preserved
    /// as an additional `graph_memory_item` regardless of assembler results.
    ///
    /// Every item in the bundle is advisory. The advisory_warning field is
    /// set at construction and must never be removed.
    fn assemble_context(
        &self,
        input: &ObjectiveInput,
        objective: &arpagona_agent_core::cognitive_work::Objective,
        now: DateTime<Utc>,
    ) -> ContextBundle {
        let bundle_id =
            ContextBundleId::new(format!("cb-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        let mut bundle =
            ContextBundle::new(bundle_id, input.cycle_id.clone(), objective.id.clone());

        // Step 1: Create a MemoryQueryRequest and query the assembler
        let request = arpagona_agent_core::orchestrator::MemoryQueryRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            &input.text,
            input.workspace_id.clone(),
        );

        let responses = self.context_assembler.assemble(&request);

        // Step 2: Map responses to ContextBundle fields
        for response in &responses {
            match response.source {
                ContextSource::GraphMemory => {
                    for item in &response.items {
                        bundle.graph_memory_items.push(item.clone());
                    }
                }
                ContextSource::HolographicMemory => {
                    for item in &response.items {
                        bundle.holographic_resonance_items.push(item.clone());
                    }
                }
                ContextSource::ReservoirEcho => {
                    for item in &response.items {
                        bundle.reservoir_traces.push(item.clone());
                    }
                }
                _ => {
                    // ToolRuntime, WorkingMemory, CompressedCognitiveAttention
                    // items are added to graph_memory_items as a generic
                    // bucket for now. Future milestones may add dedicated
                    // fields per source.
                    for item in &response.items {
                        bundle.graph_memory_items.push(item.clone());
                    }
                }
            }

            if !response.available {
                bundle.unavailable_sources.push(response.source.clone());
            }
        }

        // Step 3: Preserve context_hint as a special-case graph_memory_item
        // This guarantees backward compatibility with existing tests.
        if let Some(ref context_hint) = input.context_hint {
            bundle
                .graph_memory_items
                .push(arpagona_agent_core::cognitive_work::ContextItem {
                    key: "initial_context_hint".to_owned(),
                    value: context_hint.clone(),
                    source: "objective_input".to_owned(),
                });
        }

        bundle
    }

    /// Create a deterministic ComputeRouteResult.
    fn create_compute_route(
        &self,
        input: &ObjectiveInput,
        objective: &arpagona_agent_core::cognitive_work::Objective,
        bundle: &ContextBundle,
        now: DateTime<Utc>,
    ) -> ComputeRouteResult {
        let route_id =
            ComputeRouteId::new(format!("cr-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        ComputeRouteResult::new(
            route_id,
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            &self.compute_route_label,
            self.local_preferred,
            &self.compute_route_justification,
        )
    }

    /// Create a ProposalRequest linking the objective, context and compute route.
    fn create_proposal_request(
        &self,
        input: &ObjectiveInput,
        objective: &arpagona_agent_core::cognitive_work::Objective,
        bundle: &ContextBundle,
        route: &ComputeRouteResult,
    ) -> ProposalRequest {
        ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        )
        .with_compute_route(route)
    }

    /// Create a deterministic ProposedAction for simulation.
    ///
    /// The skeleton creates a ReadDocument action at Low risk. This is purely
    /// for testing the Decision Gate wiring — the orchestrator does not
    /// generate real agent proposals.
    fn create_simulation_action(
        &self,
        input: &ObjectiveInput,
        objective: &arpagona_agent_core::cognitive_work::Objective,
        now: DateTime<Utc>,
    ) -> ProposedAction {
        let action_id =
            ProposedActionId::new(format!("pa-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        ProposedAction {
            id: action_id,
            workspace_id: input.workspace_id.clone(),
            task_id: None,
            proposed_by: input.agent_id.clone(),
            action_type: ActionType::ReadDocument,
            target: Some(format!("objective:{}", objective.id)),
            payload: serde_json::json!({
                "objective": input.text,
                "cycle_id": input.cycle_id,
            }),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::ReadDocument],
            rationale: format!(
                "Deterministic simulation action for objective: {}",
                input.text
            ),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: now,
        }
    }

    /// Run a ProposedAction through the Decision Gate.
    ///
    /// Returns the Decision Gate decision + audit event pair.
    fn evaluate_through_gate(
        &self,
        action: &ProposedAction,
        permissions: &[Permission],
        _workspace_id: &WorkspaceId,
        now: DateTime<Utc>,
    ) -> (Decision, AuditEvent) {
        let decision = arpagona_decision_gate::evaluate_proposed_action(action, &[], permissions);

        let audit_event = AuditEvent::decision_created_for_action(
            AuditEventId::new(format!("audit-orch-{}", action.id.as_str())),
            ActorRef::System,
            action,
            &decision,
            now,
        );

        (decision, audit_event)
    }
}

// ─── Error types ────────────────────────────────────────────────────────────

/// Errors that can occur during orchestrator cycle execution.
#[derive(Clone, Debug, PartialEq)]
pub enum OrchestratorError {
    /// The objective text was empty after trimming.
    EmptyObjective,
    /// The objective text is too long.
    ObjectiveTooLong(usize),
    /// Malformed or invalid input.
    InvalidInput(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestratorError::EmptyObjective => {
                write!(f, "Objective text is empty after trimming")
            }
            OrchestratorError::ObjectiveTooLong(len) => {
                write!(f, "Objective text is too long ({len} chars)")
            }
            OrchestratorError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

// ─── Convenience function ───────────────────────────────────────────────────

/// Run a complete deterministic orchestrator cycle in one call.
///
/// This is a convenience wrapper around `OrchestratorEngine::run_cycle`.
/// It creates an `ObjectiveInput` from the given text, workspace and agent IDs,
/// with an optional domain hint.
///
/// ## Example
///
/// ```ignore
/// use arpagona_agent_core::ids::{AgentId, WorkspaceId};
/// use arpagona_agent_core::permission::Permission;
///
/// let result = run_deterministic_cycle(
///     "Review project documentation",
///     WorkspaceId::new("ws-1"),
///     AgentId::new("agent-alpha"),
///     &[Permission::ReadDocument],
/// ).expect("cycle should succeed");
///
/// println!("{}", result.causal_trace());
/// ```
pub fn run_deterministic_cycle(
    objective_text: impl Into<String>,
    workspace_id: WorkspaceId,
    agent_id: AgentId,
    permissions: &[Permission],
) -> Result<OrchestratorCycle, OrchestratorError> {
    let engine = OrchestratorEngine::new();
    let input = ObjectiveInput::new(objective_text, workspace_id, agent_id, Utc::now());
    engine.run_cycle(input, permissions)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::decision::DecisionStatus;

    // ─── Success paths ──────────────────────────────────────────────────

    #[test]
    fn test_deterministic_cycle_allowed_path() {
        let result = run_deterministic_cycle(
            "Review project documentation",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-alpha"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        // Decision Gate approved the low-risk ReadDocument action
        let decision = result.decision.expect("decision should exist");
        assert_eq!(decision.status, DecisionStatus::Approved);
        assert!(result.outcome.non_authorizing);
        assert!(result.outcome.gate_was_applied);
        assert!(result
            .outcome
            .summary
            .contains("evaluated by Decision Gate"));

        // All IDs are linked
        assert_eq!(
            result.outcome.objective_id.as_str(),
            result.context_bundle.objective_id.as_str()
        );
        assert_eq!(
            result.context_bundle.id,
            result.compute_route_result.context_bundle_id
        );
        assert_eq!(
            result.outcome.compute_route_id,
            Some(result.compute_route_result.id.clone())
        );
        assert!(result.outcome.proposed_action_id.is_some());
        assert_eq!(result.outcome.audit_event_count(), 1);

        // Context is advisory
        assert!(result
            .context_bundle
            .advisory_warning
            .contains("non-authorizing"));
        assert!(result
            .compute_route_result
            .advisory_warning
            .contains("non-authorizing"));
    }

    #[test]
    fn test_deterministic_cycle_with_context_hint() {
        let input = ObjectiveInput::new(
            "Business strategy analysis",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            Utc::now(),
        )
        .with_domain(arpagona_agent_core::cognitive_work::ObjectiveDomain::Business)
        .with_context("Q1 financial data");

        let engine = OrchestratorEngine::new();
        let cycle = engine
            .run_cycle(input, &[Permission::ReadDocument])
            .expect("cycle should succeed");

        // Context bundle should include the context hint
        assert!(cycle.context_bundle.has_context());
        assert_eq!(cycle.context_bundle.total_items(), 1);
        assert_eq!(
            cycle.context_bundle.graph_memory_items[0].value,
            "Q1 financial data"
        );
        assert!(cycle
            .context_bundle
            .advisory_warning
            .contains("non-authorizing"));
    }

    #[test]
    fn test_deterministic_cycle_causal_trace_format() {
        let result = run_deterministic_cycle(
            "Test causal trace",
            WorkspaceId::new("ws-trace"),
            AgentId::new("agent-trace"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        let trace = result.causal_trace();
        assert!(trace.contains("Cycle:"));
        assert!(trace.contains("Objective:"));
        assert!(trace.contains("Context:"));
        assert!(trace.contains("├─"));
        assert!(trace.contains("graph_memory"));
        assert!(trace.contains("holo"));
        assert!(trace.contains("reservoir"));
        assert!(trace.contains("Total:"));
        assert!(trace.contains("Compute:"));
        assert!(trace.contains("Action:"));
        assert!(trace.contains("Decision:"));
        assert!(trace.contains("Audit:"));
        assert!(trace.contains("Gate:"));
        assert!(trace.contains("Non-auth:"));
        assert!(trace.contains("Status:"));
        assert!(trace.contains("Summary:"));
    }

    #[test]
    fn test_orchestrator_cycle_to_cycle_trace_is_non_authorizing() {
        let result = run_deterministic_cycle(
            "Cycle trace invariant test",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        let trace = result.to_cycle_trace();
        assert!(trace.non_authorizing);
        assert_eq!(trace.cycle_id, result.outcome.cycle_id);
        assert_eq!(trace.objective_text, "Cycle trace invariant test");
        assert!(trace.context_source_summaries.len() >= 3);
        // All sources are "available" with the SimulatedContextAssembler
        for summary in &trace.context_source_summaries {
            assert!(summary.available);
        }
    }

    #[test]
    fn test_cycle_trace_serialization_round_trip() {
        let result = run_deterministic_cycle(
            "Serialization test",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        let trace = result.to_cycle_trace();
        let json = serde_json::to_value(&trace).expect("should serialize");
        assert_eq!(json["objective_text"], "Serialization test");
        assert_eq!(json["non_authorizing"], true);
        assert!(json.get("approved").is_none());
        assert!(json.get("authorized").is_none());
        assert!(json.get("executed").is_none());

        let decoded: CycleTrace = serde_json::from_value(json).expect("should deserialize");
        assert_eq!(decoded.objective_text, trace.objective_text);
        assert!(decoded.non_authorizing);
    }

    #[test]
    fn test_cycle_trace_with_context_hint_shows_sample() {
        let input = ObjectiveInput::new(
            "Context trace test",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            Utc::now(),
        )
        .with_context("Q1 financial results");

        let engine = OrchestratorEngine::new();
        let cycle = engine
            .run_cycle(input, &[Permission::ReadDocument])
            .expect("cycle should succeed");

        let trace = cycle.to_cycle_trace();
        // The context hint is added as a graph_memory_item
        let graph_mem = trace
            .context_source_summaries
            .iter()
            .find(|s| s.source == "graph_memory")
            .expect("graph_memory summary should exist");
        assert_eq!(graph_mem.item_count, 1);
        assert!(graph_mem.sample_key.is_some());
        assert!(graph_mem.sample_value_preview.is_some());
    }

    // ─── Blocked path ───────────────────────────────────────────────────

    #[test]
    fn test_deterministic_cycle_blocked_path() {
        let engine = OrchestratorEngine::new();
        let input = ObjectiveInput::new(
            "Access sensitive system configuration",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-alpha"),
            Utc::now(),
        );

        // Run with NO permissions — the Decision Gate should block
        let cycle = engine
            .run_cycle(input, &[])
            .expect("cycle should complete even when blocked");

        let decision = cycle.decision.expect("decision should exist");
        assert!(matches!(
            decision.status,
            DecisionStatus::Blocked | DecisionStatus::RequiresOverride
        ));
        assert!(cycle.outcome.gate_was_applied);

        // The outcome reflects the blocked state
        assert_eq!(cycle.outcome.cycle_status, CycleStatus::NeedsReview);
        assert!(cycle.outcome.non_authorizing);
        assert_eq!(cycle.outcome.audit_event_count(), 1);
    }

    // ─── Malformed input paths ──────────────────────────────────────────

    #[test]
    fn test_empty_objective_is_rejected() {
        let result = run_deterministic_cycle(
            "",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        );

        assert!(matches!(result, Err(OrchestratorError::EmptyObjective)));
    }

    #[test]
    fn test_whitespace_only_objective_is_rejected() {
        let result = run_deterministic_cycle(
            "   \n\t  ",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        );

        assert!(matches!(result, Err(OrchestratorError::EmptyObjective)));
    }

    #[test]
    fn test_malformed_input_rejected_before_decision_gate() {
        // Empty objective should fail before reaching the Decision Gate
        let engine = OrchestratorEngine::new();
        let input = ObjectiveInput::new(
            "",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            Utc::now(),
        );

        let result = engine.run_cycle(input, &[Permission::ReadDocument]);
        assert!(matches!(result, Err(OrchestratorError::EmptyObjective)));
    }

    // ─── Serialization tests ────────────────────────────────────────────

    #[test]
    fn test_orchestrator_error_display() {
        assert_eq!(
            format!("{}", OrchestratorError::EmptyObjective),
            "Objective text is empty after trimming"
        );
        assert_eq!(
            format!("{}", OrchestratorError::ObjectiveTooLong(10000)),
            "Objective text is too long (10000 chars)"
        );
        assert_eq!(
            format!("{}", OrchestratorError::InvalidInput("bad".to_owned())),
            "Invalid input: bad"
        );
    }

    // ─── Engine configuration tests ─────────────────────────────────────

    #[test]
    fn test_engine_custom_compute_route() {
        let engine = OrchestratorEngine::new()
            .with_compute_route_label("cloud_llm")
            .with_local_preferred(false)
            .with_compute_route_justification("Using cloud model for complex reasoning");

        let input = ObjectiveInput::new(
            "Analyze complex pattern",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            Utc::now(),
        );

        let cycle = engine
            .run_cycle(input, &[Permission::ReadDocument])
            .expect("cycle should succeed");

        assert_eq!(cycle.compute_route_result.selected_route_label, "cloud_llm");
        assert!(!cycle.compute_route_result.local_preferred);
        assert!(cycle.compute_route_result.justification.contains("complex"));
    }

    // ─── Advisory invariant tests ───────────────────────────────────────

    #[test]
    fn test_context_bundle_never_approves_actions() {
        let result = run_deterministic_cycle(
            "Advisory invariant test",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        let bundle_json = serde_json::to_value(&result.context_bundle).expect("should serialize");
        assert!(bundle_json.get("approved").is_none());
        assert!(bundle_json.get("authorized").is_none());
        assert!(bundle_json.get("execution_token").is_none());
    }

    #[test]
    fn test_compute_route_never_approves_actions() {
        let result = run_deterministic_cycle(
            "Advisory compute route test",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        let route_json =
            serde_json::to_value(&result.compute_route_result).expect("should serialize");
        assert!(route_json.get("approved").is_none());
        assert!(route_json.get("authorized").is_none());
        assert!(route_json.get("execution_token").is_none());
    }

    #[test]
    fn test_orchestrator_outcome_is_always_non_authorizing() {
        let result = run_deterministic_cycle(
            "Non-authorizing invariant",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            &[Permission::ReadDocument],
        )
        .expect("cycle should succeed");

        assert!(result.outcome.non_authorizing);
        let outcome_json = serde_json::to_value(&result.outcome).expect("should serialize");
        assert!(outcome_json.get("approved").is_none());
        assert!(outcome_json.get("executed").is_none());
        assert!(outcome_json.get("authorization").is_none());
    }

    // ─── ComputeRouteRequest test ───────────────────────────────────────

    #[test]
    fn test_compute_route_request_creation() {
        let _now = Utc::now();
        let request = ComputeRouteRequest::new(
            OrchestratorCycleId::new("oc-1"),
            ObjectiveId::new("obj-1"),
            ContextBundleId::new("cb-1"),
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
        );

        assert_eq!(request.cycle_id.as_str(), "oc-1");
        assert_eq!(request.objective_id.as_str(), "obj-1");
        assert_eq!(request.context_bundle_id.as_str(), "cb-1");

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(json["cycle_id"], "oc-1");
        assert_eq!(json["objective_id"], "obj-1");
    }
}
