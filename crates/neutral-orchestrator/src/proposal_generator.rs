//! ProposalGenerator trait and implementations for the Neutral Orchestrator.
//!
//! This module defines the abstract interface for generating proposals from
//! the orchestrator's assembled context. It replaces the previous hard-coded
//! `create_simulation_action` with a pluggable generator, enabling the
//! orchestrator to source proposals from deterministic logic, mock providers,
//! or real LLM providers — all in proposal-only mode (no tool execution).
//!
//! # Invariants
//!
//! - Every generated proposal has `status: ProposedActionStatus::PendingDecision`.
//! - No generator may approve, authorize, or execute an action.
//! - The Decision Gate is always a separate step after generation.
//!
//! # Available generators
//!
//! | Generator | Description | Feature gate |
//! |-----------|-------------|-------------|
//! | `SimulatedProposalGenerator` | Deterministic ReadDocument at Low risk (default) | none |
//! | `LlmProposalGenerator` | Wraps an `LlmProvider` to produce proposals from context | `llm-provider` |

use arpagona_agent_core::action::{ActionType, ProposedAction, ProposedActionStatus};
use arpagona_agent_core::cognitive_work::Objective;
#[cfg(feature = "llm-provider")]
use arpagona_agent_core::ids::AgentId;
use arpagona_agent_core::ids::ProposedActionId;
#[cfg(feature = "llm-provider")]
use arpagona_agent_core::ids::WorkspaceId;
use arpagona_agent_core::orchestrator::{
    ComputeRouteResult, ContextBundle, ObjectiveInput, ProposalRequest,
};
use arpagona_agent_core::permission::Permission;
use arpagona_agent_core::risk::RiskLevel;
use chrono::Utc;
use serde_json::json;
use std::fmt;

// ─── ProposalError ─────────────────────────────────────────────────────────

/// Errors that can occur during proposal generation.
#[derive(Debug, Clone)]
pub enum ProposalError {
    /// The proposal generator could not produce a valid proposal.
    GenerationFailed(String),
    /// The objective text was empty.
    EmptyObjective,
}

impl fmt::Display for ProposalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProposalError::GenerationFailed(msg) => write!(f, "proposal generation failed: {msg}"),
            ProposalError::EmptyObjective => {
                write!(f, "cannot generate proposal for empty objective")
            }
        }
    }
}

impl std::error::Error for ProposalError {}

// ─── ProposalGenerator trait ───────────────────────────────────────────────

/// Abstract interface for generating a ProposalAction from orchestrator context.
///
/// Implementations receive the full cycle context (objective, context bundle,
/// compute route, proposal request) and return a single ProposedAction.
///
/// # Safety
///
/// - The returned action must have `status: ProposedActionStatus::PendingDecision`.
/// - The returned action must not contain approval, authorization, or execution tokens.
/// - The generator must not bypass the Decision Gate.
pub trait ProposalGenerator: Send + Sync {
    /// Generate a ProposedAction from the orchestrator's cycle context.
    fn generate(
        &self,
        input: &ObjectiveInput,
        objective: &Objective,
        context_bundle: &ContextBundle,
        proposal_request: &ProposalRequest,
        compute_route: &ComputeRouteResult,
    ) -> Result<ProposedAction, ProposalError>;
}

// ─── SimulatedProposalGenerator ────────────────────────────────────────────

/// Deterministic proposal generator that always produces a ReadDocument action
/// at Low risk. This is the default generator for the OrchestratorEngine and
/// matches the previous hard-coded simulation behavior.
///
/// This generator is:
/// - Deterministic: same input → same output (except timestamps/IDs)
/// - Non-authorizing: every proposal is `PendingDecision`
/// - I/O-free: no LLM calls, no persistence, no external effects
pub struct SimulatedProposalGenerator {
    /// Override risk level (defaults to Low).
    risk_level: RiskLevel,
    /// Override action type (defaults to ReadDocument).
    action_type: ActionType,
}

impl Default for SimulatedProposalGenerator {
    fn default() -> Self {
        Self {
            risk_level: RiskLevel::Low,
            action_type: ActionType::ReadDocument,
        }
    }
}

impl SimulatedProposalGenerator {
    /// Create a new SimulatedProposalGenerator with default settings.
    ///
    /// Default: ActionType::ReadDocument at RiskLevel::Low.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the risk level for generated proposals.
    pub fn with_risk_level(mut self, risk_level: RiskLevel) -> Self {
        self.risk_level = risk_level;
        self
    }

    /// Override the action type for generated proposals.
    pub fn with_action_type(mut self, action_type: ActionType) -> Self {
        self.action_type = action_type;
        self
    }
}

impl ProposalGenerator for SimulatedProposalGenerator {
    fn generate(
        &self,
        input: &ObjectiveInput,
        objective: &Objective,
        _context_bundle: &ContextBundle,
        _proposal_request: &ProposalRequest,
        _compute_route: &ComputeRouteResult,
    ) -> Result<ProposedAction, ProposalError> {
        if input.text.trim().is_empty() {
            return Err(ProposalError::EmptyObjective);
        }

        let now = Utc::now();
        let action_id =
            ProposedActionId::new(format!("pa-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        Ok(ProposedAction {
            id: action_id,
            workspace_id: input.workspace_id.clone(),
            task_id: None,
            proposed_by: input.agent_id.clone(),
            action_type: self.action_type.clone(),
            target: Some(format!("objective:{}", objective.id)),
            payload: json!({
                "objective": input.text,
                "cycle_id": input.cycle_id,
                "generator": "simulated",
            }),
            risk_level: self.risk_level.clone(),
            required_permissions: vec![Permission::ReadDocument],
            rationale: format!("Simulated proposal for objective: {}", input.text),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: now,
        })
    }
}

// ─── LlmProposalGenerator (behind `llm-provider` feature) ──────────────────

/// Proposal generator backed by an `LlmProvider`.
///
/// This generator takes the orchestrator's cycle context, builds a prompt that
/// includes the objective, domain hint, and context bundle summary, then calls
/// the LLM provider's `propose_action` to produce a `ProposedActionDraft`.
///
/// The generated proposal is always `PendingDecision` and must pass through the
/// Decision Gate before approval.
///
/// # Feature gate
///
/// This generator requires the `llm-provider` feature on `arpagona-neutral-orchestrator`.
#[cfg(feature = "llm-provider")]
pub struct LlmProposalGenerator {
    provider: Box<dyn arpagona_llm::LlmProvider>,
    default_workspace_id: WorkspaceId,
    default_agent_id: AgentId,
}

#[cfg(feature = "llm-provider")]
impl LlmProposalGenerator {
    /// Create a new LlmProposalGenerator with the given LLM provider.
    ///
    /// The `default_workspace_id` and `default_agent_id` are used when the
    /// proposal draft does not override them.
    pub fn new(
        provider: Box<dyn arpagona_llm::LlmProvider>,
        default_workspace_id: WorkspaceId,
        default_agent_id: AgentId,
    ) -> Self {
        Self {
            provider,
            default_workspace_id,
            default_agent_id,
        }
    }

    /// Build a prompt string from the cycle context for the LLM provider.
    fn build_prompt(
        &self,
        input: &ObjectiveInput,
        _objective: &Objective,
        context_bundle: &ContextBundle,
        compute_route: &ComputeRouteResult,
    ) -> String {
        let domain = input
            .domain_hint
            .as_ref()
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|| "General".to_owned());

        let context_summary = format!(
            "Context assembled from {} sources:\n\
             - Graph Memory items: {}\n\
             - Holographic Resonance items: {}\n\
             - Reservoir Echo traces: {}\n\
             - Compute route: {} ({})",
            context_bundle.total_items(),
            context_bundle.graph_memory_items.len(),
            context_bundle.holographic_resonance_items.len(),
            context_bundle.reservoir_traces.len(),
            compute_route.selected_route_label,
            compute_route.justification,
        );

        format!(
            "Objective: {objective}\n\
             Domain: {domain}\n\
             \n\
             {context_summary}\n\
             \n\
             Based on the objective and context above, propose ONE action. \
             The action must be in proposal-only mode — do not execute tools \
             or approve execution.",
            objective = input.text,
            domain = domain,
            context_summary = context_summary,
        )
    }
}

#[cfg(feature = "llm-provider")]
impl ProposalGenerator for LlmProposalGenerator {
    fn generate(
        &self,
        input: &ObjectiveInput,
        objective: &Objective,
        context_bundle: &ContextBundle,
        _proposal_request: &ProposalRequest,
        compute_route: &ComputeRouteResult,
    ) -> Result<ProposedAction, ProposalError> {
        if input.text.trim().is_empty() {
            return Err(ProposalError::EmptyObjective);
        }

        let prompt = self.build_prompt(input, objective, context_bundle, compute_route);
        let llm_request = arpagona_llm::LlmActionRequest { prompt };

        // Call the LLM provider (proposal-only mode)
        let draft = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.provider.propose_action(llm_request))
                .map_err(|e| ProposalError::GenerationFailed(e.to_string()))
        })?;

        // Convert the draft to a ProposedAction with PendingDecision status
        let now = Utc::now();
        let action_id =
            ProposedActionId::new(format!("llm-pa-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        Ok(ProposedAction {
            id: action_id,
            workspace_id: input.workspace_id.clone(),
            task_id: None,
            proposed_by: input.agent_id.clone(),
            action_type: draft.action_type,
            target: draft.target,
            payload: draft.payload,
            risk_level: draft.risk_level,
            required_permissions: draft.required_permissions,
            rationale: format!(
                "LLM-proposed action for objective '{}': {}",
                input.text, draft.rationale
            ),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: now,
        })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::cognitive_work::ObjectiveDomain;
    use arpagona_agent_core::decision::DecisionStatus;
    use arpagona_agent_core::ids::{AgentId, ContextBundleId, ObjectiveId, WorkspaceId};
    use arpagona_agent_core::orchestrator::ObjectiveInput;
    use arpagona_decision_gate::evaluate_proposed_action;

    // ─── Helper: make a standard objective input ─────────────────────────

    fn make_input(text: &str) -> ObjectiveInput {
        ObjectiveInput::new(
            text,
            WorkspaceId::new("ws-test"),
            AgentId::new("agent-test"),
            Utc::now(),
        )
        .with_domain(ObjectiveDomain::General)
        .with_context("test context hint")
    }

    fn make_objective(input: &ObjectiveInput) -> Objective {
        input.to_objective()
    }

    fn make_context_bundle(input: &ObjectiveInput, objective: &Objective) -> ContextBundle {
        ContextBundle::new(
            ContextBundleId::new("cb-test"),
            input.cycle_id.clone(),
            objective.id.clone(),
        )
        .with_graph_memory(vec![])
        .with_holographic_resonance(vec![])
        .with_reservoir_traces(vec![])
    }

    fn make_compute_route(input: &ObjectiveInput, bundle: &ContextBundle) -> ComputeRouteResult {
        ComputeRouteResult::new(
            arpagona_agent_core::ids::ComputeRouteId::new("cr-test"),
            input.cycle_id.clone(),
            make_objective(input).id.clone(),
            bundle.id.clone(),
            "local_deterministic",
            true,
            "Test route for P3-7",
        )
    }

    // ─── SimulatedProposalGenerator tests ────────────────────────────────

    #[test]
    fn simulated_generator_produces_read_document() {
        let input = make_input("Review test documentation");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let generator = SimulatedProposalGenerator::new();
        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("simulated generation should succeed");

        assert_eq!(action.action_type, ActionType::ReadDocument);
        assert_eq!(action.risk_level, RiskLevel::Low);
        assert_eq!(action.status, ProposedActionStatus::PendingDecision);
        assert!(action
            .required_permissions
            .contains(&Permission::ReadDocument));
        assert!(action
            .payload
            .get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("Review test documentation"));
    }

    #[test]
    fn simulated_generator_empty_objective_returns_error() {
        let input = make_input("");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let generator = SimulatedProposalGenerator::new();
        let result = generator.generate(&input, &objective, &bundle, &proposal_request, &route);
        assert!(result.is_err());
        match result {
            Err(ProposalError::EmptyObjective) => {} // expected
            _ => panic!("expected EmptyObjective error"),
        }
    }

    #[test]
    fn simulated_generator_with_configurable_action_type() {
        let input = make_input("Test configurable action");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let generator = SimulatedProposalGenerator::new()
            .with_action_type(ActionType::ReadMemory)
            .with_risk_level(RiskLevel::Informational);
        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("generation should succeed");

        assert_eq!(action.action_type, ActionType::ReadMemory);
        assert_eq!(action.risk_level, RiskLevel::Informational);
    }

    #[test]
    fn simulated_generator_never_approves_action() {
        let input = make_input("Test non-approval invariant");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let generator = SimulatedProposalGenerator::new();
        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("generation should succeed");

        assert_eq!(action.status, ProposedActionStatus::PendingDecision);
        let payload_json = serde_json::to_value(&action).expect("should serialize");
        assert!(payload_json.get("approved").is_none());
        assert!(payload_json.get("authorized").is_none());
        assert!(payload_json.get("execution_token").is_none());
    }

    #[test]
    fn simulated_action_passes_through_decision_gate() {
        let input = make_input("Gate integration test");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let generator = SimulatedProposalGenerator::new();
        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("generation should succeed");

        // Run through Decision Gate
        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);
        assert_eq!(decision.status, DecisionStatus::Approved);
        assert!(decision.reason.contains("Approved"));
    }

    #[test]
    fn simulated_action_blocked_by_missing_permissions() {
        let input = make_input("Blocked by permissions test");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let generator = SimulatedProposalGenerator::new();
        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("generation should succeed");

        // Run through Decision Gate without required permissions
        let decision = evaluate_proposed_action(&action, &[], &[]);
        assert_eq!(decision.status, DecisionStatus::RequiresOverride);
        assert!(decision.reason.contains("Requires override"));
    }

    // ─── LlmProposalGenerator tests (behind `llm-provider` feature) ──────

    #[cfg(feature = "llm-provider")]
    #[test]
    fn llm_generator_wraps_mock_provider() {
        use arpagona_llm::{LlmProvider, MockProvider, ProposedActionDraft};

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime for LLM test");
        let _guard = rt.enter();

        let mock_draft = ProposedActionDraft {
            action_type: ActionType::ReadMemory,
            target: Some("memory:test".to_owned()),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::ReadMemory],
            rationale: "LLM-generated proposal for testing".to_owned(),
            payload: json!({"llm_executed": false, "test": true}),
        };
        let provider = MockProvider::new(mock_draft);
        let ws_id = WorkspaceId::new("ws-llm-test");
        let agent_id = AgentId::new("agent-llm-test");

        let generator = LlmProposalGenerator::new(Box::new(provider), ws_id, agent_id);

        let input = make_input("LLM proposal routing test");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("LLM-backed generation should succeed");

        assert_eq!(action.action_type, ActionType::ReadMemory);
        assert_eq!(action.status, ProposedActionStatus::PendingDecision);
        assert!(action.rationale.contains("LLM-proposed action"));
        assert_eq!(
            action.payload.get("llm_executed").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[cfg(feature = "llm-provider")]
    #[test]
    fn llm_generator_action_passes_through_decision_gate() {
        use arpagona_llm::{MockProvider, ProposedActionDraft};

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime for LLM test");
        let _guard = rt.enter();

        let mock_draft = ProposedActionDraft {
            action_type: ActionType::ReadDocument,
            target: Some("doc:llm-test".to_owned()),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::ReadDocument],
            rationale: "LLM test action".to_owned(),
            payload: json!({"llm_executed": false}),
        };
        let provider = MockProvider::new(mock_draft);
        let ws_id = WorkspaceId::new("ws-llm-test");
        let agent_id = AgentId::new("agent-llm-test");

        let generator = LlmProposalGenerator::new(Box::new(provider), ws_id, agent_id);

        let input = make_input("LLM gate integration test");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let action = generator
            .generate(&input, &objective, &bundle, &proposal_request, &route)
            .expect("generation should succeed");

        // Decision Gate still governs the action
        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);
        assert_eq!(decision.status, DecisionStatus::Approved);
        assert!(decision.reason.contains("Approved"));
    }

    #[cfg(feature = "llm-provider")]
    #[test]
    fn llm_generator_empty_objective_returns_error() {
        use arpagona_llm::MockProvider;

        let provider = MockProvider::safe_default();
        let ws_id = WorkspaceId::new("ws-llm-test");
        let agent_id = AgentId::new("agent-llm-test");

        let generator = LlmProposalGenerator::new(Box::new(provider), ws_id, agent_id);

        let input = make_input("");
        let objective = make_objective(&input);
        let bundle = make_context_bundle(&input, &objective);
        let route = make_compute_route(&input, &bundle);
        let proposal_request = ProposalRequest::new(
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle.id.clone(),
            input.workspace_id.clone(),
            input.agent_id.clone(),
        );

        let result = generator.generate(&input, &objective, &bundle, &proposal_request, &route);
        assert!(result.is_err());
    }
}
