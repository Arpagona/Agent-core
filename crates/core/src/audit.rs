use crate::ids::{AgentId, AuditEventId, DecisionId, ProposedActionId, TaskId, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    ActionProposed,
    DecisionCreated,
    HumanApprovalRequested,
    HumanApproved,
    HumanRejected,
    ExecutionStarted,
    ExecutionSucceeded,
    ExecutionFailed,
    PolicyChanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRef {
    System,
    Human(String),
    Agent(AgentId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: AuditEventId,
    pub event_type: AuditEventType,
    pub actor: ActorRef,
    pub workspace_id: Option<WorkspaceId>,
    pub task_id: Option<TaskId>,
    pub proposed_action_id: Option<ProposedActionId>,
    pub decision_id: Option<DecisionId>,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}
