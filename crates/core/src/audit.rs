use crate::action::{ActionType, ProposedAction};
use crate::decision::{Decision, DecisionStatus};
use crate::ids::{AgentId, AuditEventId, DecisionId, ProposedActionId, TaskId, WorkspaceId};
use crate::permission::Permission;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTraceSummary {
    pub event_count: usize,
    pub first_event_id: Option<AuditEventId>,
    pub last_event_id: Option<AuditEventId>,
    pub first_event_at: Option<DateTime<Utc>>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub workspace_id: Option<WorkspaceId>,
    pub task_id: Option<TaskId>,
    pub proposed_action_id: Option<ProposedActionId>,
    pub decision_id: Option<DecisionId>,
    pub has_action_proposed: bool,
    pub has_decision_created: bool,
    pub has_human_approval_request: bool,
    pub has_human_outcome: bool,
    pub has_execution_event: bool,
}

impl AuditTraceSummary {
    /// Build a compact readback summary from already-queryable audit events.
    ///
    /// This is a human-supervision helper only. It does not approve actions,
    /// execute tools, infer permissions, or turn Graph Memory into authorization.
    pub fn from_events(events: &[AuditEvent]) -> Self {
        Self {
            event_count: events.len(),
            first_event_id: events.first().map(|event| event.id.clone()),
            last_event_id: events.last().map(|event| event.id.clone()),
            first_event_at: events.first().map(|event| event.created_at),
            last_event_at: events.last().map(|event| event.created_at),
            workspace_id: first_some(events.iter().map(|event| event.workspace_id.clone())),
            task_id: first_some(events.iter().map(|event| event.task_id.clone())),
            proposed_action_id: first_some(
                events.iter().map(|event| event.proposed_action_id.clone()),
            ),
            decision_id: first_some(events.iter().map(|event| event.decision_id.clone())),
            has_action_proposed: events
                .iter()
                .any(|event| event.event_type == AuditEventType::ActionProposed),
            has_decision_created: events
                .iter()
                .any(|event| event.event_type == AuditEventType::DecisionCreated),
            has_human_approval_request: events
                .iter()
                .any(|event| event.event_type == AuditEventType::HumanApprovalRequested),
            has_human_outcome: events.iter().any(|event| {
                matches!(
                    event.event_type,
                    AuditEventType::HumanApproved | AuditEventType::HumanRejected
                )
            }),
            has_execution_event: events.iter().any(|event| {
                matches!(
                    event.event_type,
                    AuditEventType::ExecutionStarted
                        | AuditEventType::ExecutionSucceeded
                        | AuditEventType::ExecutionFailed
                )
            }),
        }
    }
}

fn first_some<T>(values: impl IntoIterator<Item = Option<T>>) -> Option<T> {
    values.into_iter().flatten().next()
}

impl AuditEvent {
    /// Build the canonical alpha audit event for a Decision Gate output.
    ///
    /// This helper keeps the causal links explicit and queryable without
    /// introducing execution, orchestration, or GraphRelation mirroring.
    pub fn decision_created(
        id: AuditEventId,
        actor: ActorRef,
        workspace_id: WorkspaceId,
        task_id: Option<TaskId>,
        decision: &Decision,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            event_type: AuditEventType::DecisionCreated,
            actor,
            workspace_id: Some(workspace_id),
            task_id,
            proposed_action_id: Some(decision.proposed_action_id.clone()),
            decision_id: Some(decision.id.clone()),
            payload: json!({
                "causal_trace": {
                    "proposed_action_id": decision.proposed_action_id,
                    "decision_id": decision.id,
                    "decision_status": decision.status,
                    "reason": decision.reason,
                    "risk_level": decision.risk_level,
                    "policies_applied": decision.policies_applied,
                }
            }),
            created_at,
        }
    }

    /// Build an explanatory decision audit event from both the proposed action
    /// and the Decision Gate output.
    ///
    /// The alpha CLI/API read this payload back verbatim for supervision. Keep
    /// it explicit enough that a blocked action explains whether it was caused
    /// by missing permission, missing policy, deny-by-default, unavailable
    /// backend, or required confirmation rather than requiring humans to infer
    /// from ids alone.
    pub fn decision_created_for_action(
        id: AuditEventId,
        actor: ActorRef,
        action: &ProposedAction,
        decision: &Decision,
        created_at: DateTime<Utc>,
    ) -> Self {
        let explanation = DecisionAuditExplanation::from_action_and_decision(action, decision);

        Self {
            id,
            event_type: AuditEventType::DecisionCreated,
            actor,
            workspace_id: Some(action.workspace_id.clone()),
            task_id: action.task_id.clone(),
            proposed_action_id: Some(action.id.clone()),
            decision_id: Some(decision.id.clone()),
            payload: json!({
                "causal_trace": {
                    "proposed_action_id": decision.proposed_action_id,
                    "decision_id": decision.id,
                    "decision_status": decision.status,
                    "decision_outcome": decision.status,
                    "reason": decision.reason,
                    "explicit_reason": decision.reason,
                    "action_type": action.action_type,
                    "risk_level": decision.risk_level,
                    "risk": decision.risk_level,
                    "policies_applied": decision.policies_applied,
                    "matched_policy_or_fallback_rule": explanation.matched_policy_or_fallback_rule,
                    "block_reason_category": explanation.block_reason_category,
                    "required_permissions": action.required_permissions,
                    "required_permission": explanation.required_permission,
                    "timestamp": created_at,
                    "suggested_next_action": explanation.suggested_next_action,
                }
            }),
            created_at,
        }
    }
}

#[derive(Debug)]
struct DecisionAuditExplanation {
    matched_policy_or_fallback_rule: String,
    block_reason_category: Option<&'static str>,
    required_permission: Option<Permission>,
    suggested_next_action: String,
}

impl DecisionAuditExplanation {
    fn from_action_and_decision(action: &ProposedAction, decision: &Decision) -> Self {
        let required_permission = action.required_permissions.first().cloned();
        let matched_policy_or_fallback_rule = if decision.policies_applied.is_empty() {
            fallback_rule_for_decision(action, decision).to_owned()
        } else {
            decision
                .policies_applied
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let block_reason_category = block_reason_category(action, decision);
        let suggested_next_action = suggested_next_action(action, decision, block_reason_category);

        Self {
            matched_policy_or_fallback_rule,
            block_reason_category,
            required_permission,
            suggested_next_action,
        }
    }
}

fn fallback_rule_for_decision(action: &ProposedAction, decision: &Decision) -> &'static str {
    if matches!(decision.status, DecisionStatus::Blocked) {
        let reason = decision.reason.to_ascii_lowercase();
        if reason.contains("required permission") {
            "missing_permission"
        } else if reason.contains("backend") && reason.contains("unavailable") {
            "unavailable_backend"
        } else {
            "deny_by_default"
        }
    } else if matches!(action.action_type, ActionType::Custom(_))
        && matches!(decision.status, DecisionStatus::NeedsHumanApproval)
    {
        "missing_policy"
    } else if matches!(decision.status, DecisionStatus::NeedsHumanApproval) {
        "required_confirmation"
    } else {
        "permission_granted_default_allow"
    }
}

fn block_reason_category(action: &ProposedAction, decision: &Decision) -> Option<&'static str> {
    match decision.status {
        DecisionStatus::Blocked => {
            let reason = decision.reason.to_ascii_lowercase();
            if reason.contains("required permission") {
                Some("missing_permission")
            } else if reason.contains("backend") && reason.contains("unavailable") {
                Some("unavailable_backend")
            } else if decision.policies_applied.is_empty() {
                Some("deny_by_default")
            } else {
                Some("matched_policy")
            }
        }
        DecisionStatus::NeedsHumanApproval
            if matches!(action.action_type, ActionType::Custom(_)) =>
        {
            Some("missing_policy")
        }
        DecisionStatus::NeedsHumanApproval => Some("required_confirmation"),
        DecisionStatus::Approved => None,
    }
}

fn suggested_next_action(
    action: &ProposedAction,
    decision: &Decision,
    block_reason_category: Option<&'static str>,
) -> String {
    match (&decision.status, block_reason_category) {
        (DecisionStatus::Blocked, Some("missing_permission")) => action
            .required_permissions
            .first()
            .map(|permission| {
                format!(
                    "Grant {:?} only if appropriate, then re-evaluate the proposed action.",
                    permission
                )
            })
            .unwrap_or_else(|| "Review required permissions and re-evaluate.".to_owned()),
        (DecisionStatus::Blocked, Some("unavailable_backend")) => {
            "Restore the unavailable backend or retry with an available backend before re-evaluating."
                .to_owned()
        }
        (DecisionStatus::Blocked, Some("deny_by_default")) => {
            "Create an explicit allow/escalation policy or replace the action with a safer proposal."
                .to_owned()
        }
        (DecisionStatus::Blocked, Some("matched_policy")) => {
            "Review the matched policy; do not execute unless policy or proposal changes."
                .to_owned()
        }
        (DecisionStatus::NeedsHumanApproval, Some("missing_policy")) => {
            "Ask a human to confirm or add an explicit active policy for this action type."
                .to_owned()
        }
        (DecisionStatus::NeedsHumanApproval, _) => {
            "Request human confirmation before any execution.".to_owned()
        }
        (DecisionStatus::Approved, _) => {
            "Proceed only through the normal execution path; audit readback is not execution."
                .to_owned()
        }
        _ => "Review the decision gate output before proceeding.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{Decision, DecisionStatus};
    use crate::ids::PolicyId;
    use crate::risk::RiskLevel;

    #[test]
    fn decision_created_event_keeps_causal_links_queryable() {
        let decision = Decision {
            id: DecisionId::new("decision-1"),
            proposed_action_id: ProposedActionId::new("action-1"),
            status: DecisionStatus::NeedsHumanApproval,
            reason: "Sensitive action requires a human.".to_owned(),
            risk_level: RiskLevel::High,
            policies_applied: vec![PolicyId::new("policy-human-approval")],
            decided_by: None,
            created_at: Utc::now(),
        };

        let event = AuditEvent::decision_created(
            AuditEventId::new("audit-1"),
            ActorRef::System,
            WorkspaceId::new("workspace-1"),
            Some(TaskId::new("task-1")),
            &decision,
            Utc::now(),
        );

        assert_eq!(event.event_type, AuditEventType::DecisionCreated);
        assert_eq!(event.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(event.task_id, Some(TaskId::new("task-1")));
        assert_eq!(
            event.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(event.decision_id, Some(DecisionId::new("decision-1")));
        assert_eq!(
            event.payload["causal_trace"]["proposed_action_id"],
            "action-1"
        );
        assert_eq!(event.payload["causal_trace"]["decision_id"], "decision-1");
        assert_eq!(
            event.payload["causal_trace"]["decision_status"],
            "needs_human_approval"
        );
        assert_eq!(
            event.payload["causal_trace"]["policies_applied"][0],
            "policy-human-approval"
        );
    }

    #[test]
    fn audit_trace_summary_makes_causal_trace_readable() {
        let decision = Decision {
            id: DecisionId::new("decision-1"),
            proposed_action_id: ProposedActionId::new("action-1"),
            status: DecisionStatus::NeedsHumanApproval,
            reason: "Sensitive action requires a human.".to_owned(),
            risk_level: RiskLevel::High,
            policies_applied: vec![PolicyId::new("policy-human-approval")],
            decided_by: None,
            created_at: Utc::now(),
        };
        let proposed = AuditEvent {
            id: AuditEventId::new("audit-proposed"),
            event_type: AuditEventType::ActionProposed,
            actor: ActorRef::Agent(AgentId::new("agent-1")),
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(TaskId::new("task-1")),
            proposed_action_id: Some(ProposedActionId::new("action-1")),
            decision_id: None,
            payload: json!({}),
            created_at: Utc::now(),
        };
        let decided = AuditEvent::decision_created(
            AuditEventId::new("audit-decision"),
            ActorRef::System,
            WorkspaceId::new("workspace-1"),
            Some(TaskId::new("task-1")),
            &decision,
            Utc::now(),
        );

        let proposed_at = proposed.created_at;
        let decided_at = decided.created_at;

        let summary = AuditTraceSummary::from_events(&[proposed, decided]);

        assert_eq!(summary.event_count, 2);
        assert_eq!(
            summary.first_event_id,
            Some(AuditEventId::new("audit-proposed"))
        );
        assert_eq!(
            summary.last_event_id,
            Some(AuditEventId::new("audit-decision"))
        );
        assert_eq!(summary.first_event_at, Some(proposed_at));
        assert_eq!(summary.last_event_at, Some(decided_at));
        assert_eq!(summary.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(summary.task_id, Some(TaskId::new("task-1")));
        assert_eq!(
            summary.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-1")));
        assert!(summary.has_action_proposed);
        assert!(summary.has_decision_created);
        assert!(!summary.has_execution_event);
    }

    #[test]
    fn audit_trace_summary_preserves_chronological_boundaries_without_approval() {
        let first_at = "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let last_at = "2026-01-01T00:05:00Z".parse::<DateTime<Utc>>().unwrap();
        let proposed = AuditEvent {
            id: AuditEventId::new("audit-1"),
            event_type: AuditEventType::ActionProposed,
            actor: ActorRef::Agent(AgentId::new("agent-1")),
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(TaskId::new("task-1")),
            proposed_action_id: Some(ProposedActionId::new("action-1")),
            decision_id: None,
            payload: json!({}),
            created_at: first_at,
        };
        let decided = AuditEvent {
            id: AuditEventId::new("audit-2"),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(TaskId::new("task-1")),
            proposed_action_id: Some(ProposedActionId::new("action-1")),
            decision_id: Some(DecisionId::new("decision-1")),
            payload: json!({}),
            created_at: last_at,
        };

        let summary = AuditTraceSummary::from_events(&[proposed, decided]);

        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.first_event_id, Some(AuditEventId::new("audit-1")));
        assert_eq!(summary.last_event_id, Some(AuditEventId::new("audit-2")));
        assert_eq!(summary.first_event_at, Some(first_at));
        assert_eq!(summary.last_event_at, Some(last_at));
        assert_eq!(summary.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(summary.task_id, Some(TaskId::new("task-1")));
        assert_eq!(
            summary.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-1")));
        assert!(!summary.has_human_outcome);
        assert!(!summary.has_execution_event);
    }
}
