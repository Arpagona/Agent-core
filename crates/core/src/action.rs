use crate::graph::GraphRef;
use crate::ids::{AgentId, ProposedActionId, TaskId, WorkspaceId};
use crate::permission::Permission;
use crate::risk::RiskLevel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    ReadMemory,
    ReadTasks,
    ReadProposedActions,
    ReadPendingActions,
    ReadDecisions,
    ReadAudit,
    ReadStatus,
    SystemCheck,
    WriteMemory,
    ReadDocument,
    WriteDocument,
    ProposeToolUse,
    SimulateEmail,
    ManageTask,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedActionStatus {
    PendingDecision,
    Approved,
    Blocked,
    NeedsHumanApproval,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposedAction {
    pub id: ProposedActionId,
    pub workspace_id: WorkspaceId,
    pub task_id: Option<TaskId>,
    pub proposed_by: AgentId,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub payload: Value,
    pub risk_level: RiskLevel,
    pub required_permissions: Vec<Permission>,
    pub rationale: String,
    pub context_refs: Vec<GraphRef>,
    pub status: ProposedActionStatus,
    pub created_at: DateTime<Utc>,
}
