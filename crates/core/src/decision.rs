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
    /// The action is blocked under normal rules but could be overridden
    /// by an authorized administrator.
    RequiresOverride,
    /// The action was originally RequiresOverride but has been
    /// successfully overridden by an authorized administrator.
    ApprovedByOverride,
}

/// Whether a blocked action is eligible for administrative override.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverridePolicy {
    /// Requires a pre-shared override password/token.
    PasswordRequired,
    /// Requires explicit confirmation from an admin user.
    AdminConfirmationRequired,
    /// Requires two-step confirmation (e.g. password + second factor).
    TwoStepConfirmationRequired,
    /// This action cannot be overridden under any circumstances.
    NotOverridable,
}

impl OverridePolicy {
    pub fn is_overridable(&self) -> bool {
        !matches!(self, OverridePolicy::NotOverridable)
    }
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
    /// Human-readable hint about how to override this decision (if applicable).
    /// Only present when `status == RequiresOverride`.
    /// Examples: "PasswordRequired", "AdminConfirmationRequired"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_hint: Option<String>,
    /// Stable fingerprint of the action at decision time.
    /// Used by the override mechanism to detect if the action has changed
    /// since the RequiresOverride decision was made.
    /// Format: hash of action_type + risk_level + payload + required_permissions
    /// Only set when `status == RequiresOverride`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_fingerprint: Option<String>,
}
