use crate::graph::GraphRef;
use crate::ids::{AgentId, TaskId, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Named cognitive layers used by the ARPAGONA agent runtime.
///
/// These layers are domain markers only. They do not execute tools, call LLMs,
/// open network connections, or mutate external state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveLayer {
    Input,
    IntentParsing,
    WorkingMemory,
    ReservoirEcho,
    GraphMemory,
    HolographicMemory,
    AgentProposal,
    DecisionGate,
    HumanBoundary,
    ExecutionBoundary,
    Audit,
    Reflection,
    Other(String),
}

/// High-level phase of a safe agentic loop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLoopPhase {
    ReceiveInput,
    RecallContext,
    EchoReservoir,
    DraftIntent,
    ProposeAction,
    DecisionGate,
    AwaitHumanIfNeeded,
    Audit,
    Reflect,
}

/// Type of ripple moving through the cognitive substrate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RippleKind {
    Stimulus,
    Echo,
    Stabilization,
    Decay,
    Consolidation,
}

/// A lightweight signal that can activate short-lived reservoir traces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitivePulse {
    pub kind: RippleKind,
    pub layer: CognitiveLayer,
    pub content: String,
    pub tags: Vec<String>,
    pub context_refs: Vec<GraphRef>,
    pub strength: f32,
    pub created_at: DateTime<Utc>,
}

impl CognitivePulse {
    pub fn stimulus(content: impl Into<String>, tags: Vec<String>, now: DateTime<Utc>) -> Self {
        Self {
            kind: RippleKind::Stimulus,
            layer: CognitiveLayer::Input,
            content: content.into(),
            tags,
            context_refs: vec![],
            strength: 1.0,
            created_at: now,
        }
    }
}

/// A decaying activation trace.
///
/// This is a software analogue of the "echo" idea behind reservoir computing:
/// recent signals keep a transient influence without becoming permanent facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservoirTrace {
    pub content: String,
    pub tags: Vec<String>,
    pub context_refs: Vec<GraphRef>,
    pub activation: f32,
    pub decay: f32,
    pub created_at: DateTime<Utc>,
    pub last_echo_at: DateTime<Utc>,
}

impl ReservoirTrace {
    pub fn from_pulse(pulse: CognitivePulse, decay: f32) -> Self {
        let bounded_decay = decay.clamp(0.0, 1.0);
        let bounded_activation = pulse.strength.clamp(0.0, 1.0);
        Self {
            content: pulse.content,
            tags: pulse.tags,
            context_refs: pulse.context_refs,
            activation: bounded_activation,
            decay: bounded_decay,
            created_at: pulse.created_at,
            last_echo_at: pulse.created_at,
        }
    }

    pub fn echo(&mut self, strength: f32, now: DateTime<Utc>) {
        self.activation = (self.activation + strength.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        self.last_echo_at = now;
    }

    pub fn decay_once(&mut self) {
        self.activation *= 1.0 - self.decay;
        if self.activation < 0.0001 {
            self.activation = 0.0;
        }
    }
}

/// Bounded in-memory reservoir state.
///
/// It is intentionally deterministic and pure-domain. Persistent memory remains
/// the responsibility of Graph Memory; this reservoir represents short-lived
/// continuity, attention and recent cognitive echoes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservoirState {
    pub traces: Vec<ReservoirTrace>,
    pub capacity: usize,
    pub default_decay: f32,
    pub tick: u64,
}

impl ReservoirState {
    pub fn new(capacity: usize, default_decay: f32) -> Self {
        Self {
            traces: vec![],
            capacity: capacity.max(1),
            default_decay: default_decay.clamp(0.0, 1.0),
            tick: 0,
        }
    }

    pub fn absorb(&mut self, pulse: CognitivePulse) {
        let now = pulse.created_at;
        if let Some(trace) = self.find_related_trace_mut(&pulse.tags) {
            trace.echo(pulse.strength, now);
            return;
        }

        self.traces
            .push(ReservoirTrace::from_pulse(pulse, self.default_decay));
        self.prune_to_capacity();
    }

    pub fn decay_tick(&mut self) {
        self.tick += 1;
        for trace in &mut self.traces {
            trace.decay_once();
        }
        self.traces.retain(|trace| trace.activation > 0.0);
    }

    pub fn strongest_traces(&self, limit: usize) -> Vec<ReservoirTrace> {
        let mut traces = self.traces.clone();
        traces.sort_by(|left, right| {
            right
                .activation
                .partial_cmp(&left.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        traces.truncate(limit);
        traces
    }

    fn find_related_trace_mut(&mut self, tags: &[String]) -> Option<&mut ReservoirTrace> {
        self.traces.iter_mut().find(|trace| {
            trace
                .tags
                .iter()
                .any(|existing| tags.iter().any(|incoming| incoming == existing))
        })
    }

    fn prune_to_capacity(&mut self) {
        if self.traces.len() <= self.capacity {
            return;
        }

        self.traces.sort_by(|left, right| {
            right
                .activation
                .partial_cmp(&left.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.traces.truncate(self.capacity);
    }
}

/// Input for planning one safe cognitive cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitiveCycleInput {
    pub workspace_id: WorkspaceId,
    pub task_id: Option<TaskId>,
    pub agent_id: AgentId,
    pub user_prompt: String,
    pub context_refs: Vec<GraphRef>,
    #[serde(default)]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

/// Static plan describing how a mini Hermes-like loop should proceed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveCyclePlan {
    pub phases: Vec<AgentLoopPhase>,
    pub layers: Vec<CognitiveLayer>,
    pub safety_invariants: Vec<String>,
}

impl CognitiveCyclePlan {
    pub fn alpha_safe_default() -> Self {
        Self {
            phases: vec![
                AgentLoopPhase::ReceiveInput,
                AgentLoopPhase::RecallContext,
                AgentLoopPhase::EchoReservoir,
                AgentLoopPhase::DraftIntent,
                AgentLoopPhase::ProposeAction,
                AgentLoopPhase::DecisionGate,
                AgentLoopPhase::AwaitHumanIfNeeded,
                AgentLoopPhase::Audit,
                AgentLoopPhase::Reflect,
            ],
            layers: vec![
                CognitiveLayer::Input,
                CognitiveLayer::WorkingMemory,
                CognitiveLayer::ReservoirEcho,
                CognitiveLayer::GraphMemory,
                CognitiveLayer::AgentProposal,
                CognitiveLayer::DecisionGate,
                CognitiveLayer::HumanBoundary,
                CognitiveLayer::Audit,
                CognitiveLayer::Reflection,
            ],
            safety_invariants: vec![
                "LLM providers may propose actions but must not execute them.".to_owned(),
                "Every ProposedAction must remain PendingDecision until Decision Gate evaluation."
                    .to_owned(),
                "ExecutionBoundary is not entered in alpha runtime.".to_owned(),
                "HumanBoundary is required for sensitive or uncertain actions.".to_owned(),
                "Audit must record decisions before any future execution layer.".to_owned(),
            ],
        }
    }

    pub fn contains_phase(&self, phase: AgentLoopPhase) -> bool {
        self.phases.contains(&phase)
    }

    pub fn decision_gate_index(&self) -> Option<usize> {
        self.phases
            .iter()
            .position(|phase| phase == &AgentLoopPhase::DecisionGate)
    }

    pub fn proposal_index(&self) -> Option<usize> {
        self.phases
            .iter()
            .position(|phase| phase == &AgentLoopPhase::ProposeAction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cycle_keeps_decision_gate_after_proposal() {
        let plan = CognitiveCyclePlan::alpha_safe_default();
        let proposal_index = plan.proposal_index().expect("proposal phase");
        let decision_index = plan.decision_gate_index().expect("decision gate phase");

        assert!(proposal_index < decision_index);
        assert!(plan.contains_phase(AgentLoopPhase::AwaitHumanIfNeeded));
        assert!(plan
            .safety_invariants
            .iter()
            .any(|invariant| invariant.contains("must not execute")));
    }

    #[test]
    fn reservoir_absorbs_and_reinforces_related_pulses() {
        let now = Utc::now();
        let mut reservoir = ReservoirState::new(4, 0.25);

        reservoir.absorb(CognitivePulse::stimulus(
            "Prepare a client reply",
            vec!["client".to_owned(), "email".to_owned()],
            now,
        ));
        reservoir.absorb(CognitivePulse::stimulus(
            "Draft response safely",
            vec!["email".to_owned()],
            now,
        ));

        assert_eq!(reservoir.traces.len(), 1);
        assert_eq!(reservoir.traces[0].activation, 1.0);
    }

    #[test]
    fn reservoir_decay_reduces_activation_without_persistence() {
        let now = Utc::now();
        let mut reservoir = ReservoirState::new(4, 0.5);
        reservoir.absorb(CognitivePulse::stimulus(
            "Transient idea",
            vec!["idea".to_owned()],
            now,
        ));

        reservoir.decay_tick();

        assert_eq!(reservoir.tick, 1);
        assert!(reservoir.traces[0].activation < 1.0);
    }

    #[test]
    fn reservoir_keeps_strongest_traces_within_capacity() {
        let now = Utc::now();
        let mut reservoir = ReservoirState::new(2, 0.1);

        reservoir.absorb(CognitivePulse::stimulus("A", vec!["a".to_owned()], now));
        reservoir.absorb(CognitivePulse::stimulus("B", vec!["b".to_owned()], now));
        reservoir.decay_tick();
        reservoir.absorb(CognitivePulse::stimulus("C", vec!["c".to_owned()], now));

        assert_eq!(reservoir.traces.len(), 2);
        assert!(reservoir
            .strongest_traces(1)
            .first()
            .is_some_and(|trace| trace.content == "C"));
    }
}
