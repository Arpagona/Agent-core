use crate::ids::ToolId;
use crate::permission::Permission;
use crate::risk::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Available,
    Disabled,
    Deprecated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub required_permissions: Vec<Permission>,
    pub default_risk_level: RiskLevel,
    pub status: ToolStatus,
}
