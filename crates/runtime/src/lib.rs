//! Experimental safe cognitive runtime for ARPAGONA Agent Core.
//!
//! The runtime stitches together the cognitive-domain primitives and LLM
//! providers, but deliberately stops at `ProposedAction`. It does not execute
//! tools, does not call the Decision Gate automatically, and does not mutate
//! external state.

mod governed_tool_executor;

pub use governed_tool_executor::{govern_and_execute_tool_call, GovernedToolCallResult};

use arpagona_agent_core::{
    AgentId, CognitiveCycleInput, CognitiveCyclePlan, CognitiveLayer, CognitivePulse,
    ProposedAction, ProposedActionId, ProposedActionStatus, ReservoirState, RippleKind, TaskId,
    WorkspaceId,
};
use arpagona_llm::{LlmActionRequest, LlmError, LlmProvider, ProposedActionDraft};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Configuration for a single safe runtime cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub reservoir_capacity: usize,
    pub reservoir_decay: f32,
    pub proposer_agent_id: AgentId,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            reservoir_capacity: 16,
            reservoir_decay: 0.15,
            proposer_agent_id: AgentId::new("agent-proposer-v0"),
        }
    }
}

/// Observable output of one cognitive cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCycleOutput {
    pub input: CognitiveCycleInput,
    pub pulse: CognitivePulse,
    pub strongest_reservoir_traces: Vec<String>,
    pub proposed_action: ProposedAction,
    pub cycle_plan: CognitiveCyclePlan,
    pub notes: Vec<String>,
}

/// Minimal runtime state.
///
/// The reservoir is intentionally volatile. Persisted memory belongs to Graph
/// Memory; execution belongs to future tool workers behind the Decision Gate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitiveRuntimeState {
    pub reservoir: ReservoirState,
    pub cycle_plan: CognitiveCyclePlan,
    pub next_action_index: u64,
}

impl CognitiveRuntimeState {
    pub fn new(config: &RuntimeConfig) -> Self {
        Self {
            reservoir: ReservoirState::new(config.reservoir_capacity, config.reservoir_decay),
            cycle_plan: CognitiveCyclePlan::alpha_safe_default(),
            next_action_index: 1,
        }
    }

    pub fn next_action_id(&mut self) -> ProposedActionId {
        let id = ProposedActionId::new(format!("action-runtime-{}", self.next_action_index));
        self.next_action_index += 1;
        id
    }
}

/// Safe mini Hermes-like runtime.
///
/// The generic provider lets tests use `MockProvider` and deployments use an
/// OpenAI provider. In all cases the provider can only return a draft; this
/// runtime materializes a `ProposedAction` that still requires explicit Decision
/// Gate evaluation elsewhere.
#[derive(Clone, Debug)]
pub struct CognitiveRuntime<P> {
    pub provider: P,
    pub config: RuntimeConfig,
    pub state: CognitiveRuntimeState,
}

impl<P> CognitiveRuntime<P> {
    pub fn new(provider: P, config: RuntimeConfig) -> Self {
        let state = CognitiveRuntimeState::new(&config);
        Self {
            provider,
            config,
            state,
        }
    }
}

impl<P> CognitiveRuntime<P>
where
    P: LlmProvider,
{
    /// Run one safe proposal cycle.
    ///
    /// This method never executes tools and never calls the Decision Gate. The
    /// returned action must still be evaluated by the Decision Gate before any
    /// future execution layer can proceed.
    pub async fn propose_once(
        &mut self,
        input: CognitiveCycleInput,
    ) -> Result<RuntimeCycleOutput, RuntimeError> {
        let pulse = pulse_from_input(&input);
        self.state.reservoir.absorb(pulse.clone());
        self.state.reservoir.decay_tick();

        let llm_request = LlmActionRequest {
            prompt: build_provider_prompt(&input, &self.state),
        };
        let draft = self.provider.propose_action(llm_request).await?;
        let proposed_action = materialize_draft(
            draft,
            input.workspace_id.clone(),
            input.task_id.clone(),
            self.config.proposer_agent_id.clone(),
            self.state.next_action_id(),
        );

        let strongest_reservoir_traces = self
            .state
            .reservoir
            .strongest_traces(5)
            .into_iter()
            .map(|trace| trace.content)
            .collect();

        Ok(RuntimeCycleOutput {
            input,
            pulse,
            strongest_reservoir_traces,
            proposed_action,
            cycle_plan: self.state.cycle_plan.clone(),
            notes: vec![
                "Runtime stopped at ProposedAction.".to_owned(),
                "Decision Gate must be called explicitly by the host application.".to_owned(),
                "No Decision was created.".to_owned(),
                "No AuditEvent was created.".to_owned(),
                "No tool execution occurred.".to_owned(),
            ],
        })
    }
}

pub fn pulse_from_input(input: &CognitiveCycleInput) -> CognitivePulse {
    CognitivePulse {
        kind: RippleKind::Stimulus,
        layer: CognitiveLayer::Input,
        content: input.user_prompt.clone(),
        tags: derive_tags(&input.user_prompt),
        context_refs: input.context_refs.clone(),
        strength: 1.0,
        created_at: input.created_at,
    }
}

pub fn build_provider_prompt(input: &CognitiveCycleInput, state: &CognitiveRuntimeState) -> String {
    let reservoir_context = state
        .reservoir
        .strongest_traces(5)
        .into_iter()
        .map(|trace| format!("- {} (activation {:.2})", trace.content, trace.activation))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "User request:\n{}\n\nActive reservoir echoes:\n{}\n\nReturn a safe ProposedActionDraft only. Do not execute anything.",
        input.user_prompt,
        if reservoir_context.is_empty() {
            "- none".to_owned()
        } else {
            reservoir_context
        }
    )
}

pub fn materialize_draft(
    draft: ProposedActionDraft,
    workspace_id: WorkspaceId,
    task_id: Option<TaskId>,
    proposed_by: AgentId,
    action_id: ProposedActionId,
) -> ProposedAction {
    let action = draft.into_proposed_action(workspace_id, task_id, proposed_by, action_id);
    debug_assert_eq!(action.status, ProposedActionStatus::PendingDecision);
    action
}

fn derive_tags(prompt: &str) -> Vec<String> {
    let mut tags = vec![];
    let lower = prompt.to_ascii_lowercase();
    for candidate in [
        "email", "client", "devis", "document", "task", "mémoire", "memory",
    ] {
        if lower.contains(candidate) {
            tags.push(candidate.to_owned());
        }
    }
    if tags.is_empty() {
        tags.push("general".to_owned());
    }
    tags
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Provider(LlmError),
}

impl From<LlmError> for RuntimeError {
    fn from(error: LlmError) -> Self {
        Self::Provider(error)
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Provider(error) => write!(f, "provider error: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

pub fn runtime_cycle_input(
    workspace_id: impl Into<String>,
    task_id: Option<impl Into<String>>,
    agent_id: impl Into<String>,
    user_prompt: impl Into<String>,
    now: DateTime<Utc>,
) -> CognitiveCycleInput {
    CognitiveCycleInput {
        workspace_id: WorkspaceId::new(workspace_id.into()),
        task_id: task_id.map(|id| TaskId::new(id.into())),
        agent_id: AgentId::new(agent_id.into()),
        user_prompt: user_prompt.into(),
        context_refs: vec![],
        metadata: runtime_metadata(),
        created_at: now,
    }
}

pub fn runtime_metadata() -> Value {
    json!({
        "runtime": "cognitive-runtime-v0",
        "executes_tools": false,
        "calls_decision_gate_automatically": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::{ActionType, Permission, RiskLevel};
    use arpagona_llm::{MockProvider, ProposedActionDraft};

    fn mock_provider() -> MockProvider {
        MockProvider::new(ProposedActionDraft {
            action_type: ActionType::SimulateEmail,
            target: Some("client-response-draft".to_owned()),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::SimulateEmail],
            rationale: "Prepare a draft only.".to_owned(),
            payload: json!({"body": "Bonjour"}),
        })
    }

    #[tokio::test]
    async fn runtime_proposes_pending_action_without_decision_gate() {
        let mut runtime = CognitiveRuntime::new(mock_provider(), RuntimeConfig::default());
        let input = runtime_cycle_input(
            "workspace-alpha",
            Some("task-1"),
            "agent-alpha",
            "Prépare un email client pour le devis",
            Utc::now(),
        );

        let output = runtime
            .propose_once(input)
            .await
            .expect("mock runtime should propose");

        assert_eq!(
            output.proposed_action.status,
            ProposedActionStatus::PendingDecision
        );
        assert_eq!(
            output.proposed_action.action_type,
            ActionType::SimulateEmail
        );
        assert!(output
            .notes
            .iter()
            .any(|note| note.contains("No tool execution")));
        assert!(output.notes.iter().any(|note| note.contains("No Decision")));
        assert!(output
            .notes
            .iter()
            .any(|note| note.contains("No AuditEvent")));
        assert!(
            output.cycle_plan.proposal_index().unwrap()
                < output.cycle_plan.decision_gate_index().unwrap()
        );
        assert_eq!(output.input.metadata["executes_tools"], false);
        assert_eq!(
            output.input.metadata["calls_decision_gate_automatically"],
            false
        );
    }

    #[test]
    fn pulse_from_input_derives_basic_tags() {
        let input = runtime_cycle_input(
            "workspace-alpha",
            None::<String>,
            "agent-alpha",
            "Prépare un email client",
            Utc::now(),
        );

        let pulse = pulse_from_input(&input);

        assert!(pulse.tags.contains(&"email".to_owned()));
        assert!(pulse.tags.contains(&"client".to_owned()));
    }

    #[test]
    fn runtime_metadata_declares_no_execution() {
        let metadata = runtime_metadata();

        assert_eq!(metadata["executes_tools"], false);
        assert_eq!(metadata["calls_decision_gate_automatically"], false);
    }
}
