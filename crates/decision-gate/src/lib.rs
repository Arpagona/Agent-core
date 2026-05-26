//! Pure Decision Gate rules for ARPAGONA Agent Core.
//!
//! This crate intentionally contains no LLM, database, API, shell, or tool
//! execution logic. It turns proposed actions into explicit, auditable
//! decisions according to domain policies and granted permissions.

pub mod override_engine;

use arpagona_agent_core::{
    ActionType, ActorRef, AuditEvent, AuditEventId, Decision, DecisionActor, DecisionId,
    DecisionStatus, OverridePolicy, Permission, Policy, PolicyId, ProposedAction, RiskLevel,
};
use chrono::Utc;

/// Determine if an action is a read-only informational action that was explicitly
/// requested by the user.
///
/// All of these must be true:
/// 1. Action type is one of the read-only types (ReadMemory, ReadTasks, etc.)
/// 2. Risk level is Informational
/// 3. Payload explicitly marks the action as `read_only: true`
///    (set by the deterministic `read_only_turn` when the user explicitly asks)
///
/// This distinguishes explicit user-requested reads from implicit/ambiguous
/// proposals that should still go through the normal permission check.
fn is_read_only_informational_action(action: &ProposedAction) -> bool {
    if !matches!(action.risk_level, RiskLevel::Informational) {
        return false;
    }
    if !matches!(
        action.action_type,
        ActionType::ReadMemory
            | ActionType::ReadTasks
            | ActionType::ReadProposedActions
            | ActionType::ReadPendingActions
            | ActionType::ReadDecisions
            | ActionType::ReadAudit
            | ActionType::ReadStatus
    ) {
        return false;
    }
    // The read_only_turn function sets this flag for explicitly requested reads.
    // Without this flag, the action is treated as implicit/ambiguous and goes
    // through the normal permission check.
    action
        .payload
        .get("read_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

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

    let (status, reason) = if is_read_only_informational_action(action) {
        // Auto-grant required permissions for explicitly requested read-only
        // informational actions. These are harmless (no mutation), auditable,
        // and explicitly user-requested. The required permissions are treated
        // as implicitly granted for the purpose of this decision.
        //
        // This rule intentionally does NOT cover:
        // - Actions without the read_only: true payload flag (implicit/ambiguous)
        // - Higher risk levels (Low/Medium/High/Critical)
        // - Non-read action types (WriteMemory, SimulateEmail, etc.)
        (
            DecisionStatus::Approved,
            "Approved because the user explicitly requested an informational read-only action; this is harmless and fully auditable."
                .to_owned(),
        )
    } else if let Some(missing_permission) = action
        .required_permissions
        .iter()
        .find(|required| !granted_permissions.contains(required))
    {
        // Check if this blocked action is eligible for administrative override
        let policy = override_engine::classify_override_policy(action);
        if matches!(policy, OverridePolicy::PasswordRequired) {
            (
                DecisionStatus::RequiresOverride,
                format!(
                    "Requires override because required permission {:?} was not granted; override is available for this action type.",
                    missing_permission
                ),
            )
        } else {
            (
                DecisionStatus::Blocked,
                format!(
                    "Blocked because required permission {:?} was not granted; override not available for this action type.",
                    missing_permission
                ),
            )
        }
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

    let override_hint = if matches!(status, DecisionStatus::RequiresOverride) {
        Some(format!(
            "{:?}",
            override_engine::classify_override_policy(action)
        ))
    } else {
        None
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
        override_hint,
        action_fingerprint: None,
    }
}

/// Create the audit event that records a Decision Gate output.
pub fn audit_event_for_decision(action: &ProposedAction, decision: &Decision) -> AuditEvent {
    AuditEvent::decision_created_for_action(
        AuditEventId::new(format!("audit-decision-{}", action.id.as_str())),
        ActorRef::System,
        action,
        decision,
        Utc::now(),
    )
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
    use arpagona_agent_core::{
        AgentId, AuditEventType, MemoryWriteIntent, MemoryWriteKind, MemoryWriteProvenance,
        MemoryWriteTarget, PolicyId, ProposedActionId, ProposedActionStatus, SourceId, TaskId,
        WorkspaceId,
    };
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

    fn memory_write_action(kind: MemoryWriteKind, risk_level: RiskLevel) -> ProposedAction {
        let intent = MemoryWriteIntent::new(
            kind.clone(),
            MemoryWriteTarget::fact("project", "arpagona-agent-core", "current_priority"),
            MemoryWriteProvenance::new(
                Some(SourceId::new("source-focus-loop")),
                "focus loop",
                "system_observation",
                "A bounded focus-loop run selected a governed memory-write proposal path.",
            ),
            0.86,
            AgentId::new("agent-1"),
            "Remember the project-level operational priority as governed memory intent.",
            Utc::now(),
        );

        let mut action = proposed_action(kind.action_type(), risk_level);
        action.target = Some("memory:project:arpagona-agent-core".to_owned());
        action.payload =
            serde_json::to_value(intent).expect("memory write intent should serialize");
        action.required_permissions = vec![Permission::WriteMemory];
        action.rationale = "Agent proposes a governed memory write intent.".to_owned();
        action
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
    fn missing_permission_now_requires_override() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(decision.status, DecisionStatus::RequiresOverride);
        assert!(decision.reason.contains("Requires override"));
        assert!(decision.override_hint.is_some());
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
        assert_eq!(
            event.payload["causal_trace"]["decision_status"],
            json!("approved")
        );
        assert_eq!(
            event.payload["causal_trace"]["action_type"],
            json!("read_document")
        );
        assert_eq!(
            event.payload["causal_trace"]["matched_policy_or_fallback_rule"],
            json!("permission_granted_default_allow")
        );
    }

    #[test]
    fn missing_write_memory_permission_blocks_create_memory_fact() {
        let action = memory_write_action(MemoryWriteKind::CreateMemoryFact, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[]);
        let event = audit_event_for_decision(&action, &decision);
        let trace = &event.payload["causal_trace"];

        assert_eq!(decision.status, DecisionStatus::Blocked);
        assert!(decision.reason.contains("required permission"));
        assert_eq!(trace["action_type"], json!("create_memory_fact"));
        assert_eq!(trace["required_permission"], json!("write_memory"));
        assert_eq!(
            trace["matched_policy_or_fallback_rule"],
            json!("missing_permission")
        );
        assert_eq!(trace["block_reason_category"], json!("missing_permission"));
        assert!(trace["suggested_next_action"]
            .as_str()
            .unwrap()
            .contains("WriteMemory"));
    }

    #[test]
    fn medium_memory_write_with_permission_needs_human_approval() {
        let action = memory_write_action(
            MemoryWriteKind::CreateFailureInsightMemory,
            RiskLevel::Medium,
        );

        let decision = evaluate_proposed_action(&action, &[], &[Permission::WriteMemory]);
        let event = audit_event_for_decision(&action, &decision);
        let trace = &event.payload["causal_trace"];

        assert_eq!(decision.status, DecisionStatus::NeedsHumanApproval);
        assert!(decision.reason.contains("Medium risk"));
        assert_eq!(trace["action_type"], json!("create_failure_insight_memory"));
        assert_eq!(trace["required_permissions"], json!(["write_memory"]));
        assert_eq!(
            trace["matched_policy_or_fallback_rule"],
            json!("required_confirmation")
        );
        assert_eq!(
            trace["block_reason_category"],
            json!("required_confirmation")
        );
        assert!(trace["suggested_next_action"]
            .as_str()
            .unwrap()
            .contains("human confirmation"));
    }

    // ── Read-only informational auto-grant tests ──────────────────────────

    fn read_only_action(action_type: ActionType, risk_level: RiskLevel) -> ProposedAction {
        let mut action = proposed_action(action_type, risk_level);
        action.payload = json!({"read_only": true, "operation": "read_memory"});
        action
    }

    #[test]
    fn explicit_read_memory_informational_approved_without_granted_permissions() {
        // (a) read_memory explicite + informational => allowed
        let action = read_only_action(ActionType::ReadMemory, RiskLevel::Informational);

        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(
            decision.status,
            DecisionStatus::Approved,
            "explicit informational read_memory should be approved: {}",
            decision.reason
        );
        assert!(decision.reason.contains("read-only"));
    }

    #[test]
    fn implicit_read_memory_without_read_only_flag_requires_override() {
        // (b) read_memory implicite ou ambiguë => blocked
        // Without read_only: true in payload, the action goes through the
        // normal permission check and is requires_override (was Blocked before
        // the override mechanism was added).
        let action = proposed_action(ActionType::ReadMemory, RiskLevel::Informational);

        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(
            decision.status,
            DecisionStatus::RequiresOverride,
            "implicit read_memory without read_only flag should require override: {}",
            decision.reason
        );
        assert!(decision.reason.contains("Requires override"));
        assert!(decision.override_hint.is_some());
    }

    #[test]
    fn read_memory_low_risk_not_informational_requires_override() {
        // (c) read_memory hors périmètre => requires override
        // With Low risk (not Informational), the auto-grant doesn't apply.
        // The override mechanism makes this RequiresOverride instead of Blocked.
        let action = read_only_action(ActionType::ReadMemory, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(
            decision.status,
            DecisionStatus::RequiresOverride,
            "read_memory with Low risk should require override without permission: {}",
            decision.reason
        );
        assert!(decision.reason.contains("Requires override"));
        assert!(decision.override_hint.is_some());
    }

    #[test]
    fn non_read_action_with_read_only_flag_still_blocked() {
        // (c) bis: A non-read action type (SimulateEmail) with read_only: true
        // is not covered by the auto-grant rule — still blocked.
        let mut action = proposed_action(ActionType::SimulateEmail, RiskLevel::Informational);
        action.payload = json!({"read_only": true});

        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(
            decision.status,
            DecisionStatus::Blocked,
            "SimulateEmail with read_only flag should be blocked: {}",
            decision.reason
        );
    }

    #[test]
    fn approved_read_only_action_produces_audit_event() {
        // (d) toute lecture autorisée doit générer un audit event
        let action = read_only_action(ActionType::ReadMemory, RiskLevel::Informational);
        let decision = evaluate_proposed_action(&action, &[], &[]);

        assert_eq!(decision.status, DecisionStatus::Approved);

        let event = audit_event_for_decision(&action, &decision);

        assert_eq!(event.event_type, AuditEventType::DecisionCreated);
        assert_eq!(event.actor, ActorRef::System);
        assert!(event.proposed_action_id.is_some());
        assert!(event.decision_id.is_some());
        // Verify the causal trace records the auto-grant rule
        assert_eq!(
            event.payload["causal_trace"]["action_type"],
            json!("read_memory")
        );
        assert_eq!(
            event.payload["causal_trace"]["decision_status"],
            json!("approved")
        );
    }
}
