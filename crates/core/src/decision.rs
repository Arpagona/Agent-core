use crate::ids::{AgentId, DecisionId, PolicyId, ProposedActionId};
use crate::risk::RiskLevel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Approved,
    Blocked,
    NeedsHumanApproval,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionActor {
    System,
    Human(String),
    Agent(AgentId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    pub id: DecisionId,
    pub proposed_action_id: ProposedActionId,
    pub status: DecisionStatus,
    pub reason: String,
    pub risk_level: RiskLevel,
    pub policies_applied: Vec<PolicyId>,
    pub decided_by: Option<DecisionActor>,
    pub created_at: DateTime<Utc>,
}
