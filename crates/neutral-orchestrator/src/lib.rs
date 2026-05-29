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

use arpagona_agent_core::action::ProposedAction;
use arpagona_agent_core::audit::{ActorRef, AuditEvent};
use arpagona_agent_core::cognitive_work::CycleStatus;
use arpagona_agent_core::decision::Decision;
use arpagona_agent_core::ids::{AgentId, AuditEventId, ContextBundleId, WorkspaceId};
#[cfg(test)]
use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId};
#[cfg(test)]
use arpagona_agent_core::orchestrator::ComputeRouteRequest;
use arpagona_agent_core::orchestrator::{
    ComputeRouteResult, ContextBundle, ContextSource, CycleTrace, ObjectiveInput,
    OrchestratorOutcome, ProposalRequest,
};
use arpagona_agent_core::permission::Permission;
use chrono::{DateTime, Utc};

pub mod compressed_cognitive_attention_adapter;
pub mod compute_reservoir_adapter;
pub mod context_assembler;
pub mod graph_memory_adapter;
pub mod holographic_memory_adapter;
pub mod multi_adapter;
pub mod proposal_generator;
pub mod reservoir_echo_adapter;
pub mod tool_runtime_adapter;

pub use compressed_cognitive_attention_adapter::CompressedCognitiveAttentionAdapter;
pub use compute_reservoir_adapter::ComputeReservoirResolver;
pub use context_assembler::{ContextAssembler, SimulatedContextAssembler};
pub use graph_memory_adapter::GraphMemoryAdapter;
pub use holographic_memory_adapter::HolographicMemoryAdapter;
pub use multi_adapter::MultiAdapterContextAssembler;
#[cfg(feature = "llm-provider")]
pub use proposal_generator::LlmProposalGenerator;
pub use proposal_generator::{ProposalError, ProposalGenerator, SimulatedProposalGenerator};
pub use reservoir_echo_adapter::ReservoirEchoAdapter;
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

        // Compute route cost/quality metadata (structured)
        trace.compute_route_expected_cost_cents = self.compute_route_result.expected_cost_cents;
        trace.compute_route_expected_latency_ms = self.compute_route_result.expected_latency_ms;
        trace.compute_route_resource_kind = self.compute_route_result.resource_kind.clone();

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

        // Failure insight candidates from context assembly and decision state
        let candidates = trace.detect_failure_candidates();
        trace.failure_insight_candidates = candidates;

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
    /// Compute Reservoir resolver for real resource-aware route allocation.
    /// Defaults to a resolver with the standard compute node inventory and
    /// a local-first ComputePolicy.
    compute_reservoir_resolver: ComputeReservoirResolver,
    /// Pluggable context assembler for gathering advisory context from memory sources.
    /// Defaults to SimulatedContextAssembler (no-op, deterministic, zero I/O).
    context_assembler: Box<dyn ContextAssembler>,
    /// Pluggable proposal generator for creating ProposedActions from cycle context.
    /// Defaults to SimulatedProposalGenerator (deterministic ReadDocument at Low risk).
    proposal_generator: Box<dyn ProposalGenerator>,
}

impl Default for OrchestratorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestratorEngine {
    /// Create a new orchestrator engine with default settings.
    ///
    /// Default compute route: real ComputeReservoirResolver with local-first policy.
    /// Default context assembler: SimulatedContextAssembler (no-op, zero I/O).
    /// Default proposal generator: SimulatedProposalGenerator (deterministic ReadDocument at Low risk).
    pub fn new() -> Self {
        Self {
            compute_reservoir_resolver: ComputeReservoirResolver::new(),
            context_assembler: Box::new(SimulatedContextAssembler::new()),
            proposal_generator: Box::new(SimulatedProposalGenerator::new()),
        }
    }

    /// Configure the compute reservoir resolver for resource-aware route allocation.
    ///
    /// Use this to plug in a custom resolver with different compute nodes,
    /// policies, or heuristic strategies. Defaults to `ComputeReservoirResolver::new()`.
    ///
    /// # Safety
    ///
    /// The resolver must return advisory compute route results only. No result
    /// may contain approval, authorization, or execution tokens.
    pub fn with_compute_reservoir_resolver(mut self, resolver: ComputeReservoirResolver) -> Self {
        self.compute_reservoir_resolver = resolver;
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

    /// Configure a custom ProposalGenerator for creating ProposedActions.
    ///
    /// Use this to plug in real LLM-backed generators or custom deterministic
    /// logic. Defaults to SimulatedProposalGenerator.
    ///
    /// # Safety
    ///
    /// The provided generator must produce proposals with
    /// `status: ProposedActionStatus::PendingDecision`. No proposal may
    /// contain approval, authorization or execution tokens.
    pub fn with_proposal_generator(mut self, generator: Box<dyn ProposalGenerator>) -> Self {
        self.proposal_generator = generator;
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

        // Step 2: Generate a shared bundle ID for cross-linking
        let bundle_id =
            ContextBundleId::new(format!("cb-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        // Step 3: Create ComputeRouteResult — now computed BEFORE context
        // assembly so that the compute route info (label, local_preferred)
        // can be propagated into the context assembly pipeline.
        let compute_route = self.create_compute_route(&input, &objective, &bundle_id, now);

        // Step 4: Assemble advisory ContextBundle, now with compute route hints
        let context_bundle =
            self.assemble_context(&input, &objective, &compute_route, bundle_id, now);

        // Step 5: Create ProposalRequest
        let proposal_request =
            self.create_proposal_request(&input, &objective, &context_bundle, &compute_route);

        // Step 6: Generate a ProposedAction via the pluggable proposal generator
        // The default SimulatedProposalGenerator produces a ReadDocument action
        // at Low risk. An LlmProposalGenerator would produce a context-aware
        // proposal from the LLM provider. All proposals are PendingDecision
        // and must pass through the Decision Gate.
        let proposed_action = self.proposal_generator.generate(
            &input,
            &objective,
            &context_bundle,
            &proposal_request,
            &compute_route,
        )?;

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
    /// The compute route result is passed into the MemoryQueryRequest so that
    /// adapters can prioritize sources relevant to the selected resource type.
    /// For example, a "local-small" route may favor Reservoir Echo traces,
    /// while a "cloud-strong" route may favor Graph Memory facts.
    ///
    /// Every item in the bundle is advisory. The advisory_warning field is
    /// set at construction and must never be removed.
    fn assemble_context(
        &self,
        input: &ObjectiveInput,
        objective: &arpagona_agent_core::cognitive_work::Objective,
        compute_route: &ComputeRouteResult,
        bundle_id: ContextBundleId,
        _now: DateTime<Utc>,
    ) -> ContextBundle {
        let mut bundle =
            ContextBundle::new(bundle_id, input.cycle_id.clone(), objective.id.clone());

        // Step 1: Create a MemoryQueryRequest with compute route hints
        let request = arpagona_agent_core::orchestrator::MemoryQueryRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            &input.text,
            input.workspace_id.clone(),
        )
        .with_compute_route(
            Some(compute_route.selected_route_label.clone()),
            Some(compute_route.local_preferred),
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

    /// Create a ComputeRouteResult using the ComputeReservoirResolver.
    ///
    /// This replaces the old hard-coded deterministic label with a real
    /// allocation from the compute-reservoir crate, producing explainable
    /// cost/latency/sensitivity/capability trade-offs.
    fn create_compute_route(
        &self,
        input: &ObjectiveInput,
        objective: &arpagona_agent_core::cognitive_work::Objective,
        bundle_id: &ContextBundleId,
        now: DateTime<Utc>,
    ) -> ComputeRouteResult {
        self.compute_reservoir_resolver
            .resolve(input, objective, bundle_id, now)
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
    /// Proposal generation failed.
    ProposalFailed(String),
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
            OrchestratorError::ProposalFailed(msg) => {
                write!(f, "Proposal generation failed: {msg}")
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<ProposalError> for OrchestratorError {
    fn from(err: ProposalError) -> Self {
        OrchestratorError::ProposalFailed(err.to_string())
    }
}

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
    fn test_engine_with_custom_compute_reservoir_resolver() {
        // Create a custom resolver with explicit nodes that guarantees
        // a specific route outcome.
        let custom_node = arpagona_compute_reservoir::ComputeNode {
            id: arpagona_compute_reservoir::ComputeNodeId::new("custom-worker"),
            label: "Custom worker".to_owned(),
            kind: arpagona_compute_reservoir::ComputeResourceKind::RemoteWorker,
            status: arpagona_compute_reservoir::ComputeNodeStatus::Available,
            capabilities: vec![arpagona_compute_reservoir::ComputeCapability::SimpleReasoning],
            max_data_sensitivity: arpagona_compute_reservoir::DataSensitivity::Public,
            expected_cost_cents: 10,
            expected_latency_ms: 300,
            is_local: false,
            strength: 5,
        };

        // Also add a local node to guarantee a selection with local-first budget
        let local_node = arpagona_compute_reservoir::ComputeNode {
            id: arpagona_compute_reservoir::ComputeNodeId::new("custom-local"),
            label: "Custom local".to_owned(),
            kind: arpagona_compute_reservoir::ComputeResourceKind::LocalLlm,
            status: arpagona_compute_reservoir::ComputeNodeStatus::Available,
            capabilities: vec![arpagona_compute_reservoir::ComputeCapability::SimpleReasoning],
            max_data_sensitivity: arpagona_compute_reservoir::DataSensitivity::Public,
            expected_cost_cents: 0,
            expected_latency_ms: 100,
            is_local: true,
            strength: 4,
        };

        let resolver = ComputeReservoirResolver::new().with_nodes(vec![custom_node, local_node]);

        let engine = OrchestratorEngine::new().with_compute_reservoir_resolver(resolver);

        let input = ObjectiveInput::new(
            "Simple analysis task",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-1"),
            Utc::now(),
        );

        let cycle = engine
            .run_cycle(input, &[Permission::ReadDocument])
            .expect("cycle should succeed");

        // The custom resolver should select the custom local node
        // since the budget is local-first
        assert!(cycle.compute_route_result.local_preferred);
        assert!(cycle
            .compute_route_result
            .selected_route_label
            .contains("custom"));
        assert!(cycle
            .compute_route_result
            .justification
            .contains("Compute Reservoir"));
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
