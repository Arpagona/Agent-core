//! Deterministic execution capability registry.
//!
//! Maps known [`ActionType`] values to execution capabilities, permissions,
//! dry-run support, and execution eligibility.
//!
//! This is a **declarative** registry: it contains no tool execution logic,
//! no LLM calls, no side effects, and no Decision Gate bypass. Unknown or
//! unsupported action types return a blocked capability result.

use crate::{ActionType, Permission, RiskLevel};
use serde::{Deserialize, Serialize};

/// Capability declaration for a known action type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionCapability {
    /// The action type this capability entry describes.
    pub action_type: ActionType,
    /// Unique identifier of the executor that can handle this action type, or
    /// `None` if no real executor is registered.
    pub executor_id: Option<String>,
    /// Whether this action type supports dry-run simulation.
    pub supports_dry_run: bool,
    /// Whether real execution is enabled. **Always `false`** in this alpha;
    /// no tools, files, network, or external systems may be affected.
    pub supports_real_execution: bool,
    /// Permissions required to execute this action type.
    pub required_permissions: Vec<Permission>,
    /// Kinds of resources this action type may touch (e.g. `"memory"`,
    /// `"document"`, `"email"`, `"system"`).
    pub touched_resource_kinds: Vec<String>,
    /// Maximum allowed risk level for this action type. Actions with a higher
    /// risk level are blocked from dry-run and eventual execution.
    pub max_allowed_risk: RiskLevel,
    /// Human-readable reversibility statement.
    pub reversibility: String,
    /// Whether human approval is required before any execution (dry-run or real).
    pub human_approval_required: bool,
    /// Free-form operational notes.
    pub notes: Option<String>,
    /// Safety warning if the action type involves sensitive capabilities.
    pub safety_warning: Option<String>,
}

/// Return the deterministic execution capability for a known action type.
///
/// Unknown or custom action types return a blocked capability.
pub fn execution_capability(action_type: &ActionType) -> ExecutionCapability {
    match action_type {
        ActionType::ReadMemory => ExecutionCapability {
            action_type: ActionType::ReadMemory,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadMemory],
            touched_resource_kinds: vec!["memory".to_owned()],
            max_allowed_risk: RiskLevel::Low,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only memory inspection.".to_owned()),
            safety_warning: None,
        },
        ActionType::ReadTasks => ExecutionCapability {
            action_type: ActionType::ReadTasks,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadTasks],
            touched_resource_kinds: vec!["task_store".to_owned()],
            max_allowed_risk: RiskLevel::Informational,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only task listing.".to_owned()),
            safety_warning: None,
        },
        ActionType::ReadProposedActions | ActionType::ReadPendingActions => ExecutionCapability {
            action_type: action_type.clone(),
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadProposedActions],
            touched_resource_kinds: vec!["proposal_store".to_owned()],
            max_allowed_risk: RiskLevel::Informational,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only proposal inspection.".to_owned()),
            safety_warning: None,
        },
        ActionType::ReadDecisions => ExecutionCapability {
            action_type: ActionType::ReadDecisions,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadDecisions],
            touched_resource_kinds: vec!["decision_store".to_owned()],
            max_allowed_risk: RiskLevel::Informational,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only decision listing.".to_owned()),
            safety_warning: None,
        },
        ActionType::ReadAudit => ExecutionCapability {
            action_type: ActionType::ReadAudit,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadAudit],
            touched_resource_kinds: vec!["audit_store".to_owned()],
            max_allowed_risk: RiskLevel::Informational,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only audit event listing.".to_owned()),
            safety_warning: None,
        },
        ActionType::ReadStatus => ExecutionCapability {
            action_type: ActionType::ReadStatus,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadStatus],
            touched_resource_kinds: vec!["system_status".to_owned()],
            max_allowed_risk: RiskLevel::Informational,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only system status inspection.".to_owned()),
            safety_warning: None,
        },
        ActionType::SystemCheck => ExecutionCapability {
            action_type: ActionType::SystemCheck,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![],
            touched_resource_kinds: vec!["system_health".to_owned()],
            max_allowed_risk: RiskLevel::Low,
            reversibility: "Fully reversible — no state change.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only system health diagnostics.".to_owned()),
            safety_warning: None,
        },
        ActionType::WriteMemory
        | ActionType::CreateMemoryFact
        | ActionType::LinkMemoryFact
        | ActionType::InvalidateMemoryFact
        | ActionType::CreateFailureInsightMemory
        | ActionType::CreateHolographicTrace => {
            let human = matches!(
                action_type,
                ActionType::InvalidateMemoryFact
                    | ActionType::CreateFailureInsightMemory
                    | ActionType::CreateHolographicTrace
            );
            ExecutionCapability {
                action_type: action_type.clone(),
                executor_id: None,
                supports_dry_run: true,
                supports_real_execution: false,
                required_permissions: vec![Permission::WriteMemory],
                touched_resource_kinds: vec!["memory".to_owned()],
                max_allowed_risk: RiskLevel::Medium,
                reversibility: "Reversible via invalidation or supersession.".to_owned(),
                human_approval_required: human,
                notes: Some("Governed memory-write action type. Dry-run only simulates the write proposal.".to_string()),
                safety_warning: Some(
                    "Memory writes mutate agent knowledge. Invalidation and failure insight \
                     writes should be reviewed carefully."
                        .to_owned(),
                ),
            }
        }
        ActionType::ReadDocument => ExecutionCapability {
            action_type: ActionType::ReadDocument,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ReadDocument],
            touched_resource_kinds: vec!["document".to_owned()],
            max_allowed_risk: RiskLevel::Low,
            reversibility: "Fully reversible — no state mutation.".to_owned(),
            human_approval_required: false,
            notes: Some("Read-only document access.".to_owned()),
            safety_warning: None,
        },
        ActionType::WriteDocument => ExecutionCapability {
            action_type: ActionType::WriteDocument,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::WriteDocument],
            touched_resource_kinds: vec!["document".to_owned()],
            max_allowed_risk: RiskLevel::Medium,
            reversibility: "Reversible if version control is enabled.".to_owned(),
            human_approval_required: true,
            notes: Some("Document writes are simulated only — no files are modified.".to_owned()),
            safety_warning: Some(
                "Document writes could overwrite files if real execution is enabled.".to_owned(),
            ),
        },
        ActionType::ProposeToolUse => ExecutionCapability {
            action_type: ActionType::ProposeToolUse,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ProposeToolUse],
            touched_resource_kinds: vec!["tool".to_owned()],
            max_allowed_risk: RiskLevel::Medium,
            reversibility: "Fully reversible — proposal only, no execution.".to_owned(),
            human_approval_required: true,
            notes: Some(
                "Proposing tool use is a meta-action: it creates a new proposal.".to_owned(),
            ),
            safety_warning: Some(
                "Tool use may have side effects. Real execution is disabled.".to_owned(),
            ),
        },
        ActionType::DirectToolCall => ExecutionCapability {
            action_type: ActionType::DirectToolCall,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ProposeToolUse],
            touched_resource_kinds: vec!["llm_tool_call".to_owned()],
            max_allowed_risk: RiskLevel::Medium,
            reversibility: "Fully reversible — governance evaluation only, no execution.".to_owned(),
            human_approval_required: true,
            notes: Some(
                "DirectToolCall represents an LLM-initiated tool-call intent being evaluated by the Decision Gate.".to_owned(),
            ),
            safety_warning: Some(
                "LLM tool-call execution is disabled by default. May have side effects when enabled.".to_owned(),
            ),
        },
        ActionType::SimulateEmail => ExecutionCapability {
            action_type: ActionType::SimulateEmail,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::SimulateEmail],
            touched_resource_kinds: vec!["email".to_owned()],
            max_allowed_risk: RiskLevel::Medium,
            reversibility: "Fully reversible — no email is sent.".to_owned(),
            human_approval_required: true,
            notes: Some("Email simulation generates a draft without sending.".to_owned()),
            safety_warning: Some(
                "External communications could have reputation or compliance implications \
                 if real execution were enabled."
                    .to_owned(),
            ),
        },
        ActionType::ManageTask => ExecutionCapability {
            action_type: ActionType::ManageTask,
            executor_id: None,
            supports_dry_run: true,
            supports_real_execution: false,
            required_permissions: vec![Permission::ManageTask],
            touched_resource_kinds: vec!["task_store".to_owned()],
            max_allowed_risk: RiskLevel::Medium,
            reversibility: "Task lifecycle changes may not be trivially reversible.".to_owned(),
            human_approval_required: true,
            notes: Some("Task management affects tracking state.".to_owned()),
            safety_warning: None,
        },
        ActionType::Custom(name) => ExecutionCapability {
            action_type: ActionType::Custom(name.clone()),
            executor_id: None,
            supports_dry_run: false,
            supports_real_execution: false,
            required_permissions: vec![],
            touched_resource_kinds: vec!["unknown".to_owned()],
            max_allowed_risk: RiskLevel::Informational,
            reversibility: "Unknown — custom action type has no declared capability.".to_owned(),
            human_approval_required: true,
            notes: Some(format!(
                "Custom action type '{name}' has no registered capability entry."
            )),
            safety_warning: Some(
                "Unknown custom action types cannot be dry-run or executed. \
                 Register a capability entry before use."
                    .to_owned(),
            ),
        },
    }
}

/// List all known action types and their capabilities.
pub fn list_execution_capabilities() -> Vec<ExecutionCapability> {
    use ActionType::*;
    let known = vec![
        ReadMemory,
        ReadTasks,
        ReadProposedActions,
        ReadPendingActions,
        ReadDecisions,
        ReadAudit,
        ReadStatus,
        SystemCheck,
        WriteMemory,
        CreateMemoryFact,
        LinkMemoryFact,
        InvalidateMemoryFact,
        CreateFailureInsightMemory,
        CreateHolographicTrace,
        ReadDocument,
        WriteDocument,
        ProposeToolUse,
        SimulateEmail,
        DirectToolCall,
        ManageTask,
    ];
    known
        .into_iter()
        .map(|at| execution_capability(&at))
        .collect()
}

/// Check whether a risk level exceeds the maximum allowed for an action type.
///
/// Returns `true` if the action's risk level is higher than the capability's
/// `max_allowed_risk`, meaning the action is not eligible for dry-run or
/// execution.
pub fn risk_exceeds_max_allowed(action_risk: &RiskLevel, max_allowed: &RiskLevel) -> bool {
    risk_level_ordinal(action_risk) > risk_level_ordinal(max_allowed)
}

fn risk_level_ordinal(risk: &RiskLevel) -> u8 {
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

    // -- Known action types return deterministic capabilities ---------------

    #[test]
    fn read_memory_has_deterministic_capability() {
        let cap = execution_capability(&ActionType::ReadMemory);
        assert!(cap.supports_dry_run);
        assert!(!cap.supports_real_execution);
        assert_eq!(cap.max_allowed_risk, RiskLevel::Low);
        assert!(cap.required_permissions.contains(&Permission::ReadMemory));
    }

    #[test]
    fn system_check_has_no_required_permissions() {
        let cap = execution_capability(&ActionType::SystemCheck);
        assert!(cap.required_permissions.is_empty());
        assert!(cap.supports_dry_run);
    }

    #[test]
    fn write_document_requires_human_approval() {
        let cap = execution_capability(&ActionType::WriteDocument);
        assert!(cap.human_approval_required);
        assert!(cap.supports_dry_run);
        assert!(!cap.supports_real_execution);
    }

    #[test]
    fn simulate_email_is_dry_run_capable() {
        let cap = execution_capability(&ActionType::SimulateEmail);
        assert!(cap.supports_dry_run);
        assert!(!cap.supports_real_execution);
        assert_eq!(cap.max_allowed_risk, RiskLevel::Medium);
    }

    // -- Unknown action type is blocked ------------------------------------

    #[test]
    fn custom_action_type_is_blocked() {
        let cap = execution_capability(&ActionType::Custom("unknown".to_owned()));
        assert!(!cap.supports_dry_run);
        assert!(!cap.supports_real_execution);
        assert!(cap.human_approval_required);
        assert!(cap
            .notes
            .as_ref()
            .unwrap()
            .contains("no registered capability entry"));
    }

    #[test]
    fn custom_action_type_has_unknown_resources() {
        let cap = execution_capability(&ActionType::Custom("foo".to_owned()));
        assert_eq!(cap.touched_resource_kinds, vec!["unknown"]);
    }

    // -- Risk level comparison ---------------------------------------------

    #[test]
    fn risk_ordinal_monotonic() {
        assert!(
            risk_level_ordinal(&RiskLevel::Informational) < risk_level_ordinal(&RiskLevel::Low)
        );
        assert!(risk_level_ordinal(&RiskLevel::Low) < risk_level_ordinal(&RiskLevel::Medium));
        assert!(risk_level_ordinal(&RiskLevel::Medium) < risk_level_ordinal(&RiskLevel::High));
        assert!(risk_level_ordinal(&RiskLevel::High) < risk_level_ordinal(&RiskLevel::Critical));
    }

    #[test]
    fn risk_exceeds_low_threshold() {
        assert!(!risk_exceeds_max_allowed(
            &RiskLevel::Informational,
            &RiskLevel::Low
        ));
        assert!(!risk_exceeds_max_allowed(&RiskLevel::Low, &RiskLevel::Low));
        assert!(risk_exceeds_max_allowed(
            &RiskLevel::Medium,
            &RiskLevel::Low
        ));
        assert!(risk_exceeds_max_allowed(&RiskLevel::High, &RiskLevel::Low));
        assert!(risk_exceeds_max_allowed(
            &RiskLevel::Critical,
            &RiskLevel::Low
        ));
    }

    #[test]
    fn risk_within_medium_threshold() {
        assert!(!risk_exceeds_max_allowed(
            &RiskLevel::Informational,
            &RiskLevel::Medium
        ));
        assert!(!risk_exceeds_max_allowed(
            &RiskLevel::Low,
            &RiskLevel::Medium
        ));
        assert!(!risk_exceeds_max_allowed(
            &RiskLevel::Medium,
            &RiskLevel::Medium
        ));
        assert!(risk_exceeds_max_allowed(
            &RiskLevel::High,
            &RiskLevel::Medium
        ));
        assert!(risk_exceeds_max_allowed(
            &RiskLevel::Critical,
            &RiskLevel::Medium
        ));
    }

    #[test]
    fn high_critical_actions_not_execution_eligible_via_risk() {
        let high_actions = [
            ActionType::ReadMemory,          // max_allowed=Low
            ActionType::ReadTasks,           // max_allowed=Informational
            ActionType::ReadProposedActions, // max_allowed=Informational
        ];
        for at in &high_actions {
            let cap = execution_capability(at);
            // A High risk action of this type would exceed max_allowed
            assert!(
                risk_exceeds_max_allowed(&RiskLevel::High, &cap.max_allowed_risk),
                "High risk should exceed max_allowed for {:?}",
                at
            );
            assert!(
                risk_exceeds_max_allowed(&RiskLevel::Critical, &cap.max_allowed_risk),
                "Critical risk should exceed max_allowed for {:?}",
                at
            );
        }
    }

    // -- Real execution is disabled for all --------------------------------

    #[test]
    fn all_action_types_have_real_execution_disabled() {
        for cap in list_execution_capabilities() {
            assert!(
                !cap.supports_real_execution,
                "Real execution must remain disabled for {:?}",
                cap.action_type
            );
        }
    }
    #[test]
    fn list_capabilities_returns_every_known_type() {
        let caps = list_execution_capabilities();
        let types: Vec<String> = caps
            .iter()
            .map(|c| format!("{:?}", c.action_type))
            .collect();
        assert!(types.contains(&"ReadMemory".to_owned()));
        assert!(types.contains(&"WriteMemory".to_owned()));
        assert!(types.contains(&"SimulateEmail".to_owned()));
        assert!(types.contains(&"ProposeToolUse".to_owned()));
        assert!(types.contains(&"SystemCheck".to_owned()));
        assert!(types.contains(&"ManageTask".to_owned()));
    }
}
