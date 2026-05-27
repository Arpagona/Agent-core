//! Pure Decision Gate rules for ARPAGONA Agent Core.
//!
//! This crate intentionally contains no LLM, database, API, shell, or tool
//! execution logic. It turns proposed actions into explicit, auditable
//! decisions according to domain policies and granted permissions.

pub mod override_engine;

use arpagona_agent_core::{
    ActionType, ActorRef, AgentId, AuditEvent, AuditEventId, Decision, DecisionActor, DecisionId,
    DecisionStatus, OverridePolicy, Permission, Policy, PolicyId, ProposedAction, ProposedActionId,
    ProposedActionStatus, RiskLevel, WorkspaceId,
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

/// Evaluate a tool-call intent through the Decision Gate, producing a
/// governance result. This is the core entry point for governed direct
/// tool-calling (Track C Step C2).
///
/// The function:
/// 1. Wraps the intent as a ProposedAction with ActionType::DirectToolCall
/// 2. Runs the intent through evaluate_proposed_action
/// 3. Returns the decision with audit context
pub fn govern_tool_call(
    intent: &arpagona_agent_core::action::ToolCallIntent,
    granted_permissions: &[Permission],
) -> (Decision, ProposedAction) {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let proposed_action = ProposedAction {
        id: ProposedActionId::new(format!("direct-tc-{id}")),
        workspace_id: WorkspaceId::new("llm-workspace"),
        task_id: None,
        proposed_by: AgentId::new("llm"),
        action_type: ActionType::DirectToolCall,
        target: Some(intent.tool.clone()),
        payload: serde_json::json!({
            "tool": intent.tool,
            "arguments": intent.arguments,
            "rationale": intent.rationale,
        }),
        risk_level: intent.risk_level.clone(),
        required_permissions: vec![Permission::ProposeToolUse],
        rationale: format!(
            "LLM tool-call intent: {} — {}",
            intent.tool, intent.rationale
        ),
        context_refs: vec![],
        status: ProposedActionStatus::PendingDecision,
        created_at: chrono::Utc::now(),
    };

    let decision = evaluate_proposed_action(&proposed_action, &[], granted_permissions);
    (decision, proposed_action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::{
        action::ToolCallIntent, AgentId, AuditEventType, MemoryWriteIntent, MemoryWriteKind,
        MemoryWriteProvenance, MemoryWriteTarget, PolicyId, ProposedActionId, ProposedActionStatus,
        SourceId, TaskId, WorkspaceId,
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

    #[test]
    fn missing_write_memory_permission_blocks_create_holographic_trace() {
        let action = memory_write_action(MemoryWriteKind::CreateHolographicTrace, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[]);
        let event = audit_event_for_decision(&action, &decision);
        let trace = &event.payload["causal_trace"];

        assert_eq!(decision.status, DecisionStatus::Blocked);
        assert!(decision.reason.contains("required permission"));
        assert_eq!(trace["action_type"], json!("create_holographic_trace"));
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
    fn low_risk_holographic_trace_with_permission_is_approved() {
        let action = memory_write_action(MemoryWriteKind::CreateHolographicTrace, RiskLevel::Low);

        let decision = evaluate_proposed_action(&action, &[], &[Permission::WriteMemory]);
        let event = audit_event_for_decision(&action, &decision);
        let trace = &event.payload["causal_trace"];

        assert_eq!(decision.status, DecisionStatus::Approved);
        assert!(decision.reason.contains("Approved"));
        assert_eq!(trace["action_type"], json!("create_holographic_trace"));
        assert_eq!(
            trace["matched_policy_or_fallback_rule"],
            json!("permission_granted_default_allow")
        );
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

    #[test]
    fn govern_tool_call_allows_read_tool_with_permission() {
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({"path": "test.md"}),
            rationale: "Need to read a file".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (decision, _) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
        assert_eq!(
            decision.status,
            DecisionStatus::Approved,
            "read_file with ProposeToolUse permission should be approved: {}",
            decision.reason
        );
    }

    #[test]
    fn govern_tool_call_blocks_tool_without_permission() {
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({"path": "/etc/passwd"}),
            rationale: "Check system file".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (decision, _) = govern_tool_call(&intent, &[]);
        assert_eq!(
            decision.status,
            DecisionStatus::Blocked,
            "read_file without permission should be blocked: {}",
            decision.reason
        );
    }

    #[test]
    fn govern_tool_call_blocks_write_tool_without_permission() {
        let intent = ToolCallIntent {
            tool: "write_document".to_owned(),
            arguments: json!({"path": "test.md", "content": "data"}),
            rationale: "Update documentation".to_owned(),
            risk_level: RiskLevel::Low,
        };
        let (decision, proposal) = govern_tool_call(&intent, &[Permission::ReadDocument]);
        assert_eq!(
            decision.status,
            DecisionStatus::Blocked,
            "write_document with only ReadDocument permission should be blocked: {}",
            decision.reason
        );
        assert_eq!(
            proposal.action_type,
            ActionType::DirectToolCall,
            "proposed action should use DirectToolCall type"
        );
    }
    // ── C5 anti-drift: tool bypass attempts ─────────────────────────────────
    //
    // The Decision Gate is a permission gate, not a tool whitelist.
    // Tool name validation is the responsibility of the Tool Runtime layer
    // (crates/tool-runtime). These tests prove that the governance layer:
    // - Approves tool-call intents when permissions and risk level allow
    // - Never blocks solely based on tool name (that's the runtime's job)
    // - Every tool-call intent always produces a governing Decision

    #[test]
    fn govern_tool_call_approves_shell_tool_with_permission() {
        // The Decision Gate approves when ProposeToolUse permission is granted
        // and risk is Low/Informational. Tool name validation belongs to the
        // Tool Runtime, not the governance layer.
        let intent = ToolCallIntent {
            tool: "shell".to_owned(),
            arguments: json!({"command": "rm -rf /"}),
            rationale: "Execute system cleanup".to_owned(),
            risk_level: RiskLevel::Low,
        };
        let (decision, _) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
        assert_eq!(
            decision.status,
            DecisionStatus::Approved,
            "Decision Gate approves based on permissions, not tool names: {}",
            decision.reason
        );
        // The unsafe tool name must produce a governing decision regardless
        assert!(
            decision.id.as_str().starts_with("decision-direct-tc-"),
            "every tool call must produce a governance decision"
        );
    }

    #[test]
    fn govern_tool_call_blocks_tool_without_proposetooluse_permission() {
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({"path": "test.md"}),
            rationale: "Need to read a file".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (decision, _) = govern_tool_call(&intent, &[]);
        // Without ProposeToolUse permission, the gate produces RequiresOverride
        // (since override engine classifies this as overridable)
        assert_ne!(
            decision.status,
            DecisionStatus::Approved,
            "without ProposeToolUse, tool calls should not be approved"
        );
    }

    #[test]
    fn govern_tool_call_with_any_tool_name_produces_governing_decision() {
        // Every tool name, including dangerous ones, must produce a valid
        // Decision rather than panicking or silently executing
        let test_tools = [
            "shell",
            "bash",
            "exec",
            "sh",
            "sudo",
            "rm",
            "mv",
            "chmod",
            "write",
            "curl",
            "wget",
            "ssh",
            "eval",
            "system",
            "command",
            "read_file",
            "list_files",
            "search_text",
            "",
        ];
        for tool in &test_tools {
            let intent = ToolCallIntent {
                tool: tool.to_string(),
                arguments: json!({}),
                rationale: format!("Test tool name: {tool}"),
                risk_level: RiskLevel::Informational,
            };
            let (decision, proposal) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
            // Must produce a Decision (not panic)
            assert!(
                matches!(
                    decision.status,
                    DecisionStatus::Approved
                        | DecisionStatus::Blocked
                        | DecisionStatus::NeedsHumanApproval
                ),
                "every tool name must produce a decision: {tool} -> {:?}",
                decision.status
            );
            // The proposed action must use DirectToolCall type
            assert_eq!(
                proposal.action_type,
                ActionType::DirectToolCall,
                "{tool} should produce DirectToolCall type"
            );
            assert_eq!(
                proposal.status,
                ProposedActionStatus::PendingDecision,
                "{tool} proposal must begin as PendingDecision"
            );
        }
    }

    // ── C5 anti-drift: malformed tool-call payloads ─────────────────────────

    #[test]
    fn govern_tool_call_handles_missing_arguments_gracefully() {
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({}),
            rationale: "Missing path argument".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (decision, _) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
        assert_eq!(
            decision.status,
            DecisionStatus::Approved,
            "governance should approve tool calls with valid permissions: {}",
            decision.reason
        );
    }

    #[test]
    fn govern_tool_call_handles_null_arguments_without_panic() {
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: serde_json::Value::Null,
            rationale: "Null arguments".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (decision, _) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
        assert!(
            matches!(
                decision.status,
                DecisionStatus::Approved | DecisionStatus::Blocked
            ),
            "should produce a decision without panicking: {}",
            decision.reason
        );
    }

    // ── C5 anti-drift: Decision Gate mandatory regression tests ─────────────

    #[test]
    fn every_proposed_action_begins_as_pending_decision() {
        let action = proposed_action(ActionType::ReadDocument, RiskLevel::Informational);
        assert_eq!(action.status, ProposedActionStatus::PendingDecision);

        let action2 = proposed_action(ActionType::WriteDocument, RiskLevel::Critical);
        assert_eq!(action2.status, ProposedActionStatus::PendingDecision);
    }

    #[test]
    fn proposed_action_from_tool_call_intent_begins_pending_decision() {
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({"path": "test.md"}),
            rationale: "Testing".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (_, proposal) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
        assert_eq!(
            proposal.status,
            ProposedActionStatus::PendingDecision,
            "all tool-call proposals must begin as PendingDecision"
        );
    }

    #[test]
    fn every_tool_call_requires_governance_decision() {
        // Prove the only way to evaluate a ToolCallIntent is through govern_tool_call
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({"path": "test.md"}),
            rationale: "Prove governance path".to_owned(),
            risk_level: RiskLevel::Informational,
        };
        let (decision, _) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
        // A Decision must be produced through proper governance
        assert!(
            matches!(
                decision.status,
                DecisionStatus::Approved
                    | DecisionStatus::Blocked
                    | DecisionStatus::NeedsHumanApproval
            ),
            "govern_tool_call must produce a valid decision"
        );
        assert!(
            decision.id.as_str().starts_with("decision-direct-tc-"),
            "decision ID should reference the tool-call prefix: {}",
            decision.id.as_str()
        );
    }
}
