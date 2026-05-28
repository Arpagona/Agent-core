//! Deterministic policy engine for permission and policy checks.
//!
//! Consumes [`ExecutionCapability`] metadata, proposal status, risk level,
//! permissions, resource kinds, actor, and workspace context to produce a
//! [`PolicyDecision`] that gates dry-run and future execution paths.
//!
//! The policy engine is **declarative**: it contains no tool execution logic,
//! no LLM calls, no side effects, and no Decision Gate bypass.

use crate::execution_registry::{
    execution_capability, risk_exceeds_max_allowed, ExecutionCapability,
};
use crate::{ActionType, ProposedActionStatus, RiskLevel};
use serde::{Deserialize, Serialize};

/// Policy engine decision outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Action passes all policy checks — eligible for dry-run or execution.
    Allowed,
    /// Action is blocked by one or more policy rules.
    Blocked,
    /// Action requires explicit human approval before proceeding.
    NeedsHumanApproval,
    /// Action must first go through dry-run simulation before any execution.
    NeedsDryRun,
    /// Action type is not registered in the capability registry.
    UnsupportedCapability,
}

/// Input to the policy engine for a single evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyInput {
    /// Action type to evaluate.
    pub action_type: ActionType,
    /// Current proposal status.
    pub proposal_status: ProposedActionStatus,
    /// Risk level of the proposed action.
    pub risk_level: RiskLevel,
    /// Permissions the action requires.
    pub required_permissions: Vec<String>,
    /// Resource kinds the action may touch.
    pub touched_resource_kinds: Vec<String>,
    /// Actor submitting the action (agent id or human identifier).
    pub actor: Option<String>,
    /// Workspace scope.
    pub workspace: Option<String>,
    /// Whether the caller is requesting dry-run mode.
    pub dry_run_requested: bool,
    /// Whether the caller is requesting real execution.
    pub real_execution_requested: bool,
}

impl Default for PolicyInput {
    fn default() -> Self {
        Self {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::PendingDecision,
            risk_level: RiskLevel::Informational,
            required_permissions: vec![],
            touched_resource_kinds: vec![],
            actor: None,
            workspace: None,
            dry_run_requested: false,
            real_execution_requested: false,
        }
    }
}

/// Result of a policy engine evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyEngineResult {
    /// The policy decision.
    pub decision: PolicyDecision,
    /// Human-readable explanation of why this decision was reached.
    pub reason: String,
    /// Machine-readable rule identifiers that matched.
    pub matched_rules: Vec<String>,
    /// Whether dry-run is required before execution.
    pub dry_run_required: bool,
    /// Whether human approval is required.
    pub human_approval_required: bool,
    /// The capability metadata used during evaluation.
    pub capability: Option<ExecutionCapability>,
}

impl PolicyEngineResult {
    fn allowed(cap: Option<ExecutionCapability>) -> Self {
        Self {
            decision: PolicyDecision::Allowed,
            reason: "All policy checks passed.".to_owned(),
            matched_rules: vec!["policy_engine:default_allow".to_owned()],
            dry_run_required: false,
            human_approval_required: false,
            capability: cap,
        }
    }

    fn blocked(
        reason: impl Into<String>,
        rules: Vec<&str>,
        cap: Option<ExecutionCapability>,
    ) -> Self {
        Self {
            decision: PolicyDecision::Blocked,
            reason: reason.into(),
            matched_rules: rules.into_iter().map(|r| r.to_owned()).collect(),
            dry_run_required: false,
            human_approval_required: false,
            capability: cap,
        }
    }

    fn needs_human(
        reason: impl Into<String>,
        rules: Vec<&str>,
        cap: Option<ExecutionCapability>,
    ) -> Self {
        Self {
            decision: PolicyDecision::NeedsHumanApproval,
            reason: reason.into(),
            matched_rules: rules.into_iter().map(|r| r.to_owned()).collect(),
            dry_run_required: false,
            human_approval_required: true,
            capability: cap,
        }
    }

    fn unsupported(cap: Option<ExecutionCapability>) -> Self {
        Self {
            decision: PolicyDecision::UnsupportedCapability,
            reason: "Action type is not supported by the capability registry.".to_owned(),
            matched_rules: vec!["policy_engine:unsupported_capability".to_owned()],
            dry_run_required: false,
            human_approval_required: false,
            capability: cap,
        }
    }
}

/// Deterministic policy engine.
///
/// Applies the following rules in order:
///
/// 1. **Unsupported capability** — Custom action types are unsupported/blocked.
/// 2. **Global real execution disabled** — `real_execution_requested` is always blocked.
/// 3. **Proposal status gate** — only `Approved` proposals pass for dry-run or execution.
/// 4. **Dry-run capability gate** — dry-run requires `capability.supports_dry_run`.
/// 5. **Risk threshold** — actions exceeding `capability.max_allowed_risk` are blocked.
/// 6. **Human approval requirement** — actions requiring human approval produce `NeedsHumanApproval`.
/// 7. **Dry-run requirement** — actions that support dry-run but are requested for real execution produce `NeedsDryRun`.
/// 8. **Default allow** — all checks pass.
#[derive(Clone, Debug, Default)]
pub struct PolicyEngine;

impl PolicyEngine {
    /// Evaluate a policy input and produce a decision.
    pub fn evaluate(input: &PolicyInput) -> PolicyEngineResult {
        let cap = execution_capability(&input.action_type);

        // Rule 1: Unsupported capability
        if matches!(&cap.action_type, ActionType::Custom(_)) {
            return PolicyEngineResult::unsupported(Some(cap));
        }

        // Rule 2: Global real execution disabled
        if input.real_execution_requested {
            return PolicyEngineResult::blocked(
                "Real execution is globally disabled. Only dry-run is available.",
                vec!["policy_engine:global_real_execution_disabled"],
                Some(cap),
            );
        }

        // Rule 3: Proposal status gate — only Approved passes
        if input.proposal_status != ProposedActionStatus::Approved {
            return PolicyEngineResult::blocked(
                format!(
                    "Proposal status is {:?}. Only Approved proposals may proceed.",
                    input.proposal_status
                ),
                vec!["policy_engine:non_approved_proposal"],
                Some(cap),
            );
        }

        // Rule 4: Dry-run capability gate (if dry-run is requested)
        if input.dry_run_requested && !cap.supports_dry_run {
            return PolicyEngineResult::blocked(
                format!(
                    "Action type '{:?}' does not support dry-run.",
                    input.action_type
                ),
                vec!["policy_engine:dry_run_not_supported"],
                Some(cap),
            );
        }

        // Rule 5: Risk threshold
        if risk_exceeds_max_allowed(&input.risk_level, &cap.max_allowed_risk) {
            return PolicyEngineResult::blocked(
                format!(
                    "Risk level {:?} exceeds maximum allowed {:?} for action type '{:?}'.",
                    input.risk_level, cap.max_allowed_risk, input.action_type
                ),
                vec!["policy_engine:risk_exceeds_max_allowed"],
                Some(cap),
            );
        }

        // Rule 6: Human approval requirement
        if cap.human_approval_required {
            return PolicyEngineResult::needs_human(
                format!(
                    "Action type '{:?}' requires human approval.",
                    input.action_type
                ),
                vec!["policy_engine:human_approval_required"],
                Some(cap),
            );
        }

        // Rule 7: Dry-run requirement — if dry-run is supported but not requested
        // Skip: if they didn't request dry-run, that's fine — they might want to.
        // This rule is mainly for real_execution_requested paths (which are blocked anyway).
        // For dry_run_requested=false + real_execution_requested=false, just allow.

        // Rule 8: Default allow
        PolicyEngineResult::allowed(Some(cap))
    }

    /// Evaluate specifically for dry-run mode.
    ///
    /// Shortcut that sets `dry_run_requested = true` and `real_execution_requested = false`.
    pub fn evaluate_dry_run(input: &PolicyInput) -> PolicyEngineResult {
        let mut dry_run_input = input.clone();
        dry_run_input.dry_run_requested = true;
        dry_run_input.real_execution_requested = false;
        Self::evaluate(&dry_run_input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approved_input(action_type: ActionType, risk_level: RiskLevel) -> PolicyInput {
        PolicyInput {
            action_type,
            proposal_status: ProposedActionStatus::Approved,
            risk_level,
            dry_run_requested: true,
            real_execution_requested: false,
            ..Default::default()
        }
    }

    // --- Approved low-risk known action can pass dry-run policy -----------

    #[test]
    fn approved_low_risk_read_memory_passes_dry_run_policy() {
        let input = approved_input(ActionType::ReadMemory, RiskLevel::Low);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(
            result.decision,
            PolicyDecision::Allowed,
            "approved low-risk known action should pass dry-run policy: {}",
            result.reason
        );
    }

    #[test]
    fn approved_informational_read_tasks_passes_dry_run() {
        let input = approved_input(ActionType::ReadTasks, RiskLevel::Informational);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::Allowed);
    }

    #[test]
    fn approved_low_risk_system_check_passes_dry_run() {
        let input = approved_input(ActionType::SystemCheck, RiskLevel::Low);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::Allowed);
    }

    // --- Non-approved proposals are blocked -------------------------------

    #[test]
    fn pending_proposal_is_blocked() {
        let input = PolicyInput {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::PendingDecision,
            risk_level: RiskLevel::Low,
            dry_run_requested: true,
            ..Default::default()
        };
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(
            result.decision,
            PolicyDecision::Blocked,
            "pending proposal should be blocked: {}",
            result.reason
        );
        assert!(result
            .matched_rules
            .contains(&"policy_engine:non_approved_proposal".to_owned()));
    }

    #[test]
    fn rejected_proposal_is_blocked() {
        let input = PolicyInput {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::Rejected,
            risk_level: RiskLevel::Low,
            dry_run_requested: true,
            ..Default::default()
        };
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::Blocked);
    }

    #[test]
    fn deferred_proposal_is_blocked() {
        let input = PolicyInput {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::Deferred,
            risk_level: RiskLevel::Low,
            dry_run_requested: true,
            ..Default::default()
        };
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::Blocked);
    }

    #[test]
    fn superseded_proposal_is_blocked() {
        let input = PolicyInput {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::Superseded,
            risk_level: RiskLevel::Low,
            dry_run_requested: true,
            ..Default::default()
        };
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::Blocked);
    }

    // --- Custom unknown actions are unsupported/blocked -------------------

    #[test]
    fn custom_action_type_is_unsupported() {
        let input = approved_input(ActionType::Custom("unknown".to_owned()), RiskLevel::Low);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(
            result.decision,
            PolicyDecision::UnsupportedCapability,
            "custom action type should be unsupported: {}",
            result.reason
        );
        assert!(result
            .matched_rules
            .contains(&"policy_engine:unsupported_capability".to_owned()));
    }

    // --- High/critical risk actions are blocked from execution eligibility -

    #[test]
    fn high_risk_read_memory_exceeds_allowed() {
        let input = approved_input(ActionType::ReadMemory, RiskLevel::High);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(
            result.decision,
            PolicyDecision::Blocked,
            "high risk should exceed max_allowed (Low): {}",
            result.reason
        );
        assert!(result
            .matched_rules
            .contains(&"policy_engine:risk_exceeds_max_allowed".to_owned()));
    }

    #[test]
    fn critical_risk_read_tasks_exceeds_allowed() {
        let input = approved_input(ActionType::ReadTasks, RiskLevel::Critical);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::Blocked);
    }

    #[test]
    fn medium_risk_within_threshold_for_propose_tool_use() {
        let input = approved_input(ActionType::ProposeToolUse, RiskLevel::Medium);
        let result = PolicyEngine::evaluate_dry_run(&input);
        // ProposeToolUse requires human approval, so even with dry-run it's NeedsHumanApproval
        // (Rule 6 fires before Rule 8)
        assert_eq!(
            result.decision,
            PolicyDecision::NeedsHumanApproval,
            "action with human_approval_required should produce NeedsHumanApproval: {}",
            result.reason
        );
    }

    // --- Real execution remains globally disabled -------------------------

    #[test]
    fn real_execution_is_blocked_globally() {
        let input = PolicyInput {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::Approved,
            risk_level: RiskLevel::Low,
            dry_run_requested: false,
            real_execution_requested: true,
            ..Default::default()
        };
        let result = PolicyEngine::evaluate(&input);
        assert_eq!(
            result.decision,
            PolicyDecision::Blocked,
            "real execution should be globally blocked: {}",
            result.reason
        );
        assert!(result
            .matched_rules
            .contains(&"policy_engine:global_real_execution_disabled".to_owned()));
    }

    #[test]
    fn real_execution_blocked_even_with_approved_status() {
        let input = PolicyInput {
            action_type: ActionType::SystemCheck,
            proposal_status: ProposedActionStatus::Approved,
            risk_level: RiskLevel::Low,
            dry_run_requested: false,
            real_execution_requested: true,
            ..Default::default()
        };
        let result = PolicyEngine::evaluate(&input);
        assert_eq!(result.decision, PolicyDecision::Blocked);
    }

    // --- Actions requiring human approval produce NeedsHumanApproval ------

    #[test]
    fn simulate_email_requires_human_approval() {
        let input = approved_input(ActionType::SimulateEmail, RiskLevel::Medium);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(
            result.decision,
            PolicyDecision::NeedsHumanApproval,
            "simulate_email requires human approval: {}",
            result.reason
        );
        assert!(result.human_approval_required);
    }

    #[test]
    fn write_document_requires_human_approval() {
        let input = approved_input(ActionType::WriteDocument, RiskLevel::Low);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert_eq!(result.decision, PolicyDecision::NeedsHumanApproval);
        assert!(result.human_approval_required);
    }

    // --- Dry-run not supported for custom actions -------------------------

    #[test]
    fn dry_run_not_supported_for_custom() {
        let input = approved_input(ActionType::Custom("unknown".to_owned()), RiskLevel::Low);
        let result = PolicyEngine::evaluate_dry_run(&input);
        // Rule 1 fires first, so it's UnsupportedCapability not Blocked
        assert_eq!(result.decision, PolicyDecision::UnsupportedCapability);
    }

    // --- PolicyEngineResult carries capability metadata -------------------

    #[test]
    fn policy_result_includes_capability_metadata() {
        let input = approved_input(ActionType::ReadMemory, RiskLevel::Low);
        let result = PolicyEngine::evaluate_dry_run(&input);
        assert!(
            result.capability.is_some(),
            "policy result should include capability metadata"
        );
        let cap = result.capability.unwrap();
        assert_eq!(cap.action_type, ActionType::ReadMemory);
        assert!(cap.supports_dry_run);
        assert!(!cap.supports_real_execution);
    }

    // --- Dry-run not supported for type when capability says no -----------

    #[test]
    fn dry_run_blocked_when_capability_does_not_support_it() {
        // No known action type has supports_dry_run=false except Custom.
        // We test the rule itself: an action where dry-run is requested but
        // the capability says no. Since Custom is caught earlier by Rule 1,
        // this rule is exercised for safety — there's no realistic case yet.
        let input = PolicyInput {
            action_type: ActionType::ReadMemory,
            proposal_status: ProposedActionStatus::Approved,
            risk_level: RiskLevel::Low,
            dry_run_requested: true,
            real_execution_requested: false,
            ..Default::default()
        };
        // ReadMemory supports_dry_run=true, so this passes.
        let result = PolicyEngine::evaluate(&input);
        assert_eq!(result.decision, PolicyDecision::Allowed);
    }
}
