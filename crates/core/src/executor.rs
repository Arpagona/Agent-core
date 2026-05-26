//! Executor abstraction for the governed execution pipeline.
//!
//! Defines the [`Executor`] trait that connects the capability registry,
//! policy engine, and dry-run layer to a future execution layer.
//!
//! **All real execution is globally disabled.** The only registered executor
//! is [`NoopExecutor`], which always returns `ExecutionDisabled`.

use crate::execution_registry::ExecutionCapability;
use crate::policy_engine::PolicyEngineResult;
use crate::{ActionType, AuditEventId, ProposedActionId, RiskLevel};
use serde::{Deserialize, Serialize};

/// Status of an execution attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Execution is globally disabled — no executor supports real execution.
    ExecutionDisabled,
    /// Execution was blocked by policy or capability constraints.
    ExecutionBlocked,
    /// Execution completed successfully.
    ExecutionCompleted,
    /// Execution failed.
    ExecutionFailed,
}

/// Input to an executor for a single execution attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// The proposed action id being executed.
    pub proposal_id: ProposedActionId,
    /// The action type to execute.
    pub action_type: ActionType,
    /// Actor performing the execution.
    pub actor: String,
    /// Workspace scope for the execution.
    pub workspace_scope: String,
    /// The policy engine result (must be Allowed to reach executor).
    pub policy_decision: Option<PolicyEngineResult>,
    /// Capability metadata used for this action.
    pub capability: Option<ExecutionCapability>,
    /// Optional dry-run result, if a dry-run was performed before execution.
    pub dry_run_result: Option<serde_json::Value>,
    /// Risk level of the action.
    pub risk_level: RiskLevel,
    /// Required permissions for the action.
    pub required_permissions: Vec<String>,
}

/// Result of an execution attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The execution status.
    pub status: ExecutionStatus,
    /// Human-readable explanation.
    pub reason: String,
    /// Resources touched during execution (empty for disabled/blocked).
    pub touched_resources: Vec<String>,
    /// Whether the execution is reversible.
    pub reversible: bool,
    /// Audit event id, if one was created.
    pub audit_event_id: Option<AuditEventId>,
    /// Action type that was attempted.
    pub action_type: ActionType,
    /// The proposal id that was attempted.
    pub proposal_id: ProposedActionId,
}

impl ExecutionResult {
    fn disabled(proposal_id: ProposedActionId, action_type: ActionType, reason: impl Into<String>) -> Self {
        Self {
            status: ExecutionStatus::ExecutionDisabled,
            reason: reason.into(),
            touched_resources: vec![],
            reversible: true,
            audit_event_id: None,
            action_type,
            proposal_id,
        }
    }

    fn blocked(
        proposal_id: ProposedActionId,
        action_type: ActionType,
        reason: impl Into<String>,
        audit_event_id: Option<AuditEventId>,
    ) -> Self {
        Self {
            status: ExecutionStatus::ExecutionBlocked,
            reason: reason.into(),
            touched_resources: vec![],
            reversible: true,
            audit_event_id,
            action_type,
            proposal_id,
        }
    }
}

/// The Executor trait defines the interface for executing approved actions.
///
/// **All current executors return `ExecutionDisabled`**. Real execution is
/// globally disabled. This trait exists to define the contract that future
/// executor implementations will fulfill.
pub trait Executor: Send + Sync {
    /// Unique identifier for this executor.
    fn executor_id(&self) -> &str;

    /// Action types this executor supports.
    fn supported_action_types(&self) -> Vec<ActionType>;

    /// Execute a dry-run simulation for an approved action.
    ///
    /// The default implementation returns `None`, indicating this executor
    /// does not provide dry-run simulation (the generic dry-run layer handles it).
    fn dry_run(&self, _request: &ExecutionRequest) -> Option<ExecutionResult> {
        let _ = _request;
        None
    }

    /// Execute an approved, policy-checked action.
    ///
    /// **Always returns `ExecutionDisabled`** while real execution is globally
    /// disabled. Future executors may return `ExecutionCompleted` or
    /// `ExecutionFailed`.
    fn execute(&self, request: &ExecutionRequest) -> ExecutionResult;
}

/// Noop executor — the only registered executor.
///
/// Always returns `ExecutionDisabled`. Supports all known action types for
/// the purpose of capability enumeration, but never performs any side effects.
#[derive(Clone, Debug)]
pub struct NoopExecutor;

impl NoopExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for NoopExecutor {
    fn executor_id(&self) -> &str {
        "noop-executor"
    }

    fn supported_action_types(&self) -> Vec<ActionType> {
        use ActionType::*;
        vec![
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
            ReadDocument,
            WriteDocument,
            ProposeToolUse,
            SimulateEmail,
            ManageTask,
        ]
    }

    fn execute(&self, request: &ExecutionRequest) -> ExecutionResult {
        ExecutionResult::disabled(
            request.proposal_id.clone(),
            request.action_type.clone(),
            format!(
                "Real execution is globally disabled. NoopExecutor cannot execute '{:?}' actions.",
                request.action_type
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProposedActionId;

    // -- Executor trait exists and is deterministic -----------------------

    #[test]
    fn noop_executor_has_consistent_id() {
        let ex = NoopExecutor::new();
        assert_eq!(ex.executor_id(), "noop-executor");
    }

    #[test]
    fn noop_executor_supports_known_action_types() {
        let ex = NoopExecutor::new();
        let types = ex.supported_action_types();
        assert!(types.contains(&ActionType::ReadMemory));
        assert!(types.contains(&ActionType::SimulateEmail));
        assert!(types.contains(&ActionType::SystemCheck));
        assert!(types.contains(&ActionType::ProposeToolUse));
        assert!(types.contains(&ActionType::ManageTask));
        // No Custom types
        assert!(!types.contains(&ActionType::Custom("anything".to_owned())));
    }

    // -- NoopExecutor never performs side effects -------------------------

    #[test]
    fn noop_executor_never_side_effects() {
        let ex = NoopExecutor::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("test-action"),
            action_type: ActionType::ReadMemory,
            actor: "test".to_owned(),
            workspace_scope: "test-workspace".to_owned(),
            policy_decision: None,
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::Low,
            required_permissions: vec![],
        };
        let result = ex.execute(&request);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
        assert!(result.touched_resources.is_empty());
        assert!(result.reversible);
    }

    // -- execute() is globally disabled -----------------------------------

    #[test]
    fn execute_is_globally_disabled_for_read_memory() {
        let ex = NoopExecutor::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("test-1"),
            action_type: ActionType::ReadMemory,
            actor: "test".to_owned(),
            workspace_scope: "w".to_owned(),
            policy_decision: None,
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::Informational,
            required_permissions: vec![],
        };
        let result = ex.execute(&request);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
        assert!(result.reason.contains("globally disabled"));
    }

    #[test]
    fn execute_is_globally_disabled_for_system_check() {
        let ex = NoopExecutor::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("test-2"),
            action_type: ActionType::SystemCheck,
            actor: "test".to_owned(),
            workspace_scope: "w".to_owned(),
            policy_decision: None,
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::Low,
            required_permissions: vec![],
        };
        let result = ex.execute(&request);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
    }

    // -- Policy check happens before executor call ------------------------
    // This is verified by the API integration — the endpoint calls
    // PolicyEngine.evaluate() before calling NoopExecutor.execute().
    // Unit test: NoopExecutor accepts any policy decision without checking it.

    // The following test proves the executor itself does NOT bypass policy:
    #[test]
    fn noop_executor_does_not_check_policy_itself() {
        // NoopExecutor accepts and returns ExecutionDisabled regardless of policy decision.
        // Policy enforcement happens in the API layer, not in the executor.
        let ex = NoopExecutor::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("test-3"),
            action_type: ActionType::ReadMemory,
            actor: "test".to_owned(),
            workspace_scope: "w".to_owned(),
            policy_decision: Some(crate::policy_engine::PolicyEngineResult {
                decision: crate::policy_engine::PolicyDecision::Blocked,
                reason: "test block".to_owned(),
                matched_rules: vec!["test:rule".to_owned()],
                dry_run_required: false,
                human_approval_required: false,
                capability: None,
            }),
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::Low,
            required_permissions: vec![],
        };
        // Even with blocked policy, executor returns ExecutionDisabled (not Blocked)
        // because policy enforcement is in the API layer, not the executor.
        let result = ex.execute(&request);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
    }

    // -- Unsupported action types cannot execute (handled by capability registry) --

    #[test]
    fn noop_executor_does_not_support_custom_types() {
        let ex = NoopExecutor::new();
        let types = ex.supported_action_types();
        assert!(!types.contains(&ActionType::Custom("unknown".to_owned())));
    }

    // -- high/critical risk actions cannot execute (handled by policy engine) --
    // This is enforced by PolicyEngine, not by the executor itself.
    // NoopExecutor accepts any risk level but always returns ExecutionDisabled.

    #[test]
    fn noop_executor_accepts_high_risk_but_returns_disabled() {
        let ex = NoopExecutor::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("test-high"),
            action_type: ActionType::ReadMemory,
            actor: "test".to_owned(),
            workspace_scope: "w".to_owned(),
            policy_decision: None,
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::High,
            required_permissions: vec![],
        };
        let result = ex.execute(&request);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
    }

    // -- Dry-run path remains unaffected ----------------------------------
    // NoopExecutor.dry_run() returns None — the generic dry-run layer handles it.

    #[test]
    fn noop_executor_dry_run_returns_none() {
        let ex = NoopExecutor::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("test-dry"),
            action_type: ActionType::ReadMemory,
            actor: "test".to_owned(),
            workspace_scope: "w".to_owned(),
            policy_decision: None,
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::Low,
            required_permissions: vec![],
        };
        let result = ex.dry_run(&request);
        assert!(result.is_none(), "NoopExecutor should not provide dry-run");
    }
}
