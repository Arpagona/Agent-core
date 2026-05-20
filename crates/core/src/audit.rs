use crate::decision::Decision;
use crate::ids::{AgentId, AuditEventId, DecisionId, ProposedActionId, TaskId, WorkspaceId};
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
}
