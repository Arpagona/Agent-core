use crate::action::{ActionType, ProposedAction};
use crate::audit::{ActorRef, AuditEvent, AuditEventType};
use crate::decision::{Decision, DecisionActor, DecisionStatus};
use crate::ids::{AuditEventId, DecisionId, PolicyId};
use crate::permission::Permission;
use crate::policy::Policy;
use crate::risk::RiskLevel;
use chrono::Utc;
use serde_json::json;

/// Evaluate a proposed action with pure domain rules.
///
/// The Decision Gate does not execute tools, call LLMs, perform I/O, or mutate
/// state. It turns an agent proposal plus the current policy/permission context
/// into an explicit, auditable decision.
pub fn evaluate_proposed_action(
    action: &ProposedAction,
    policies: &[Policy],
    granted_permissions: &[Permission],
) -> Decision {
    let active_applicable_policies = applicable_policies(action, policies);
    let policies_applied = active_applicable_policies
        .iter()
        .map(|policy| policy.id.clone())
        .collect::<Vec<PolicyId>>();

    let (status, reason) = if let Some(missing_permission) = action
        .required_permissions
        .iter()
        .find(|required| !granted_permissions.contains(required))
    {
        (
            DecisionStatus::Blocked,
            format!(
                "Blocked because required permission {:?} was not granted.",
                missing_permission
            ),
        )
    } else if matches!(action.action_type, ActionType::Custom(_))
        && !has_explicit_action_policy(action, policies)
    {
        (
            DecisionStatus::NeedsHumanApproval,
            "Custom action needs human approval because it is not explicitly allowed by an active action policy."
                .to_owned(),
        )
    } else if active_applicable_policies
        .iter()
        .any(|policy| policy.requires_human_approval)
    {
        (
            DecisionStatus::NeedsHumanApproval,
            format!(
                "Needs human approval because active policy requires it for {:?} at {:?} risk.",
                action.action_type, action.risk_level
            ),
        )
    } else if matches!(action.risk_level, RiskLevel::High | RiskLevel::Critical)
        && active_applicable_policies
            .iter()
            .any(|policy| !policy.requires_human_approval)
    {
        (
            DecisionStatus::Blocked,
            format!(
                "Blocked because active policy denies {:?} action at {:?} risk.",
                action.action_type, action.risk_level
            ),
        )
    } else {
        match action.risk_level {
            RiskLevel::Informational | RiskLevel::Low => (
                DecisionStatus::Approved,
                format!(
                    "Approved because {:?} risk actions are allowed when permissions are granted and no active policy requires escalation.",
                    action.risk_level
                ),
            ),
            RiskLevel::Medium => (
                DecisionStatus::NeedsHumanApproval,
                "Medium risk actions require human approval in alpha.".to_owned(),
            ),
            RiskLevel::High | RiskLevel::Critical => (
                DecisionStatus::NeedsHumanApproval,
                format!(
                    "{:?} risk actions require human approval unless an active policy blocks them.",
                    action.risk_level
                ),
            ),
        }
    };

    Decision {
        id: DecisionId::new(format!("decision-{}", action.id.as_str())),
        proposed_action_id: action.id.clone(),
        status,
        reason,
        risk_level: action.risk_level.clone(),
        policies_applied,
        decided_by: Some(DecisionActor::System),
        created_at: Utc::now(),
    }
}

/// Create the audit event that records a Decision Gate output.
pub fn audit_event_for_decision(action: &ProposedAction, decision: &Decision) -> AuditEvent {
    AuditEvent {
        id: AuditEventId::new(format!("audit-decision-{}", action.id.as_str())),
        event_type: AuditEventType::DecisionCreated,
        actor: ActorRef::System,
        workspace_id: Some(action.workspace_id.clone()),
        task_id: action.task_id.clone(),
        proposed_action_id: Some(action.id.clone()),
        decision_id: Some(decision.id.clone()),
        payload: json!({
            "action_type": action.action_type.clone(),
            "target": action.target.clone(),
            "risk_level": action.risk_level.clone(),
            "decision_status": decision.status.clone(),
            "reason": decision.reason.clone(),
            "policies_applied": decision.policies_applied.clone(),
        }),
        created_at: Utc::now(),
    }
}

fn applicable_policies<'a>(action: &ProposedAction, policies: &'a [Policy]) -> Vec<&'a Policy> {
    policies
        .iter()
        .filter(|policy| policy.enabled)
        .filter(|policy| policy_matches_action(action, policy))
        .filter(|policy| policy_matches_risk(action, policy))
        .collect()
}

fn has_explicit_action_policy(action: &ProposedAction, policies: &[Policy]) -> bool {
    policies.iter().any(|policy| {
        policy.enabled
            && policy
                .applies_to_action_type
                .as_ref()
                .is_some_and(|action_type| action_type == &action.action_type)
    })
}

fn policy_matches_action(action: &ProposedAction, policy: &Policy) -> bool {
    policy
        .applies_to_action_type
        .as_ref()
        .map_or(true, |action_type| action_type == &action.action_type)
}

fn policy_matches_risk(action: &ProposedAction, policy: &Policy) -> bool {
    policy.risk_threshold.as_ref().map_or(true, |threshold| {
        risk_rank(&action.risk_level) >= risk_rank(threshold)
    })
}

fn risk_rank(risk: &RiskLevel) -> u8 {
    match risk {
        RiskLevel::Informational => 0,
        RiskLevel::Low => 1,
        RiskLevel::Medium => 2,
        RiskLevel::High => 3,
        RiskLevel::Critical => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ProposedActionStatus;
    use crate::ids::{AgentId, PolicyId, ProposedActionId, TaskId, WorkspaceId};
    use serde_json::json;

    fn proposed_action(action_type: ActionType, risk_level: RiskLevel) -> ProposedAction {
        ProposedAction {
            id: ProposedActionId::new("action-1"),
            workspace_id: WorkspaceId::new("workspace-1"),
            task_id: Some(TaskId::new("task-1")),
            proposed_by: AgentId::new("agent-1"),
            action_type,
            target: Some("document:alpha".to_owned()),
            payload: json!({"path": "alpha.md"}),
            risk_level,
            required_permissions: vec![Permission::ReadDocument],
            rationale: "Agent proposes a controlled test action.".to_owned(),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: Utc::now(),
        }
    }

    fn policy(
        id: &str,
        action_type: Option<ActionType>,
        risk_threshold: Option<RiskLevel>,
        requires_human_approval: bool,
    ) -> Policy {
        policy_with_enabled(
            id,
            action_type,
            risk_threshold,
            requires_human_approval,
            true,
        )
    }

    fn policy_with_enabled(
        id: &str,
        action_type: Option<ActionType>,
        risk_threshold: Option<RiskLevel>,
        requires_human_approval: bool,
        enabled: bool,
    ) -> Policy {
        Policy {
            id: PolicyId::new(id),
            name: id.to_owned(),
            description: "test policy".to_owned(),
            applies_to_action_type: action_type,
            risk_threshold,
            requires_human_approval,
            enabled,
        }
    }

    #[test]
    fn low_risk_approved() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);

        assert_eq!(decision.status, DecisionStatus::Approved);
        assert_eq!(decision.id, DecisionId::new("decision-action-1"));
        assert_eq!(decision.proposed_action_id, action.id);
        assert!(decision.reason.contains("Approved"));
    }

    #[test]
    fn medium_needs_human_approval() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Medium);

        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);

        assert_eq!(decision.status, DecisionStatus::NeedsHumanApproval);
        assert!(decision.reason.contains("Medium risk"));
    }

    #[test]
    fn missing_permission_blocked() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(decision.status, DecisionStatus::Blocked);
        assert!(decision.reason.contains("required permission"));
    }

    #[test]
    fn high_risk_handled_by_policy() {
        let mut action = proposed_action(ActionType::WriteDocument, RiskLevel::High);
        action.required_permissions = vec![Permission::WriteDocument];
        let policies = vec![policy(
            "block-high-write-document",
            Some(ActionType::WriteDocument),
            Some(RiskLevel::High),
            false,
        )];

        let decision = evaluate_proposed_action(&action, &policies, &[Permission::WriteDocument]);

        assert_eq!(decision.status, DecisionStatus::Blocked);
        assert_eq!(
            decision.policies_applied,
            vec![PolicyId::new("block-high-write-document")]
        );
        assert!(decision.reason.contains("active policy"));
    }

    #[test]
    fn disabled_policy_is_ignored() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Low);
        let policies = vec![policy_with_enabled(
            "disabled-read-document-approval",
            Some(ActionType::ReadDocument),
            Some(RiskLevel::Low),
            true,
            false,
        )];

        let decision = evaluate_proposed_action(&action, &policies, &[Permission::ReadDocument]);

        assert_eq!(decision.status, DecisionStatus::Approved);
        assert!(decision.policies_applied.is_empty());
    }

    #[test]
    fn custom_action_needs_human_approval() {
        let action = proposed_action(
            ActionType::Custom("refresh_index".to_owned()),
            RiskLevel::Low,
        );

        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);

        assert_eq!(decision.status, DecisionStatus::NeedsHumanApproval);
        assert!(decision.reason.contains("Custom action"));
    }

    #[test]
    fn audit_event_created_after_decision() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Low);
        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);

        let event = audit_event_for_decision(&action, &decision);

        assert_eq!(event.event_type, AuditEventType::DecisionCreated);
        assert_eq!(event.id, AuditEventId::new("audit-decision-action-1"));
        assert_eq!(event.actor, ActorRef::System);
        assert_eq!(event.workspace_id, Some(action.workspace_id));
        assert_eq!(event.task_id, action.task_id);
        assert_eq!(event.proposed_action_id, Some(action.id));
        assert_eq!(event.decision_id, Some(decision.id));
        assert_eq!(event.payload["decision_status"], json!("approved"));
    }
}
