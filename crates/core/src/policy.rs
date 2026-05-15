use crate::action::ActionType;
use crate::ids::PolicyId;
use crate::risk::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub name: String,
    pub description: String,
    pub applies_to_action_type: Option<ActionType>,
    pub risk_threshold: Option<RiskLevel>,
    pub requires_human_approval: bool,
    pub enabled: bool,
}
