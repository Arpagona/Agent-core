//! Pure domain types for ARPAGONA Agent Core.
//!
//! This crate intentionally contains no LLM, database, API, shell, tool
//! execution logic, or Decision Gate governance rules. Agents propose actions;
//! they do not execute them.

pub mod action;
pub mod agent;
pub mod audit;
pub mod cognitive;
pub mod cognitive_work;
pub mod decision;
pub mod episode;
pub mod errors;
pub mod failure_insight;
pub mod goal;
pub mod graph;
pub mod holographic;
pub mod ids;
pub mod memory;
pub mod observation;
pub mod permission;
pub mod policy;
pub mod risk;
pub mod source;
pub mod task;
pub mod tool;
pub mod workspace;

pub use action::*;
pub use agent::*;
pub use audit::*;
pub use cognitive::*;
pub use cognitive_work::*;
pub use decision::*;
pub use episode::*;
pub use errors::*;
pub use failure_insight::*;
pub use goal::*;
pub use graph::*;
pub use holographic::*;
pub use ids::*;
pub use memory::*;
pub use observation::*;
pub use permission::*;
pub use policy::*;
pub use risk::*;
pub use source::*;
pub use task::*;
pub use tool::*;
pub use workspace::*;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn creates_low_risk_proposed_action() {
        let action = ProposedAction {
            id: ProposedActionId::new("action-1"),
            workspace_id: WorkspaceId::new("workspace-1"),
            task_id: Some(TaskId::new("task-1")),
            proposed_by: AgentId::new("agent-1"),
            action_type: ActionType::ReadDocument,
            target: Some("document:handbook".to_owned()),
            payload: json!({"path": "handbook.md"}),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::ReadMemory],
            rationale: "Read a controlled internal document.".to_owned(),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: Utc::now(),
        };

        assert_eq!(action.risk_level, RiskLevel::Low);
    }

    #[test]
    fn creates_decision_needing_human_approval() {
        let decision = Decision {
            id: DecisionId::new("decision-1"),
            proposed_action_id: ProposedActionId::new("action-1"),
            status: DecisionStatus::NeedsHumanApproval,
            reason: "The action touches sensitive data.".to_owned(),
            risk_level: RiskLevel::High,
            policies_applied: vec![PolicyId::new("policy-1")],
            decided_by: None,
            created_at: Utc::now(),
        };

        assert_eq!(decision.status, DecisionStatus::NeedsHumanApproval);
    }

    #[test]
    fn serializes_and_deserializes_fact_json() {
        let fact = Fact {
            id: FactId::new("fact-1"),
            entity_type: "company".to_owned(),
            entity_id: "arpagona".to_owned(),
            attribute: "positioning".to_owned(),
            value: json!("local-first applied AI lab"),
            source_id: Some(SourceId::new("source-1")),
            confidence: 0.95,
            valid_from: None,
            valid_to: None,
            status: FactStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let encoded = serde_json::to_string(&fact).expect("fact should serialize");
        let decoded: Fact = serde_json::from_str(&encoded).expect("fact should deserialize");

        assert_eq!(decoded.id, fact.id);
        assert_eq!(decoded.value, fact.value);
    }

    #[test]
    fn revoked_fact_status_can_be_represented() {
        let status = FactStatus::Revoked;
        assert_eq!(status, FactStatus::Revoked);
    }

    #[test]
    fn creates_action_proposed_audit_event() {
        let event = AuditEvent {
            id: AuditEventId::new("audit-1"),
            event_type: AuditEventType::ActionProposed,
            actor: ActorRef::Agent(AgentId::new("agent-1")),
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: None,
            proposed_action_id: Some(ProposedActionId::new("action-1")),
            decision_id: None,
            payload: json!({"action_type": "read_document"}),
            created_at: Utc::now(),
        };

        assert_eq!(event.event_type, AuditEventType::ActionProposed);
    }
}
