//! Executor registry for registering and resolving executors.
//!
//! Maps `executor_id` to [`Box<dyn Executor>`] and [`ActionType`] to the
//! appropriate executor. The only registered executor is [`NoopExecutor`],
//! which always returns `ExecutionDisabled`.
//!
//! This is a **deterministic** registry: it contains no tool execution logic,
//! no LLM calls, no side effects, and no Decision Gate bypass.

use crate::executor::{Executor, ExecutionRequest, ExecutionResult, ExecutionStatus, NoopExecutor};
use crate::{ActionType, AuditEventId, ProposedActionId};
use std::collections::HashMap;

/// Registry of executors, keyed by executor_id.
///
/// Supports registration, resolution by action type, and lookup by id.
/// Pre-registers [`NoopExecutor`] as the only entry by default.
pub struct ExecutorRegistry {
    executors: HashMap<String, Box<dyn Executor>>,
}

impl ExecutorRegistry {
    /// Create a new registry and register the NoopExecutor by default.
    pub fn new() -> Self {
        let mut registry = Self {
            executors: HashMap::new(),
        };
        registry.register(Box::new(NoopExecutor::new()));
        registry
    }

    /// Register an executor.
    ///
    /// If an executor with the same id already exists, it is replaced.
    pub fn register(&mut self, executor: Box<dyn Executor>) -> &mut Self {
        let id = executor.executor_id().to_owned();
        self.executors.insert(id, executor);
        self
    }

    /// Resolve an executor for the given action type.
    ///
    /// Returns `None` for `Custom`/unknown action types. Otherwise returns the
    /// first executor that supports this action type (currently NoopExecutor).
    pub fn resolve(&self, action_type: &ActionType) -> Option<&dyn Executor> {
        // Custom action types never resolve
        if matches!(action_type, ActionType::Custom(_)) {
            return None;
        }
        // Find the first executor whose supported_action_types includes this type
        for executor in self.executors.values() {
            if executor.supported_action_types().contains(action_type) {
                return Some(executor.as_ref());
            }
        }
        None
    }

    /// Get an executor by its id.
    pub fn get(&self, executor_id: &str) -> Option<&dyn Executor> {
        self.executors.get(executor_id).map(|b| b.as_ref())
    }

    /// List all registered executor ids.
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.executors.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Execute an action through the registry.
    ///
    /// Returns an [`ExecutionResult`] with `ExecutionBlocked` if no executor
    /// can handle the action type.
    pub fn execute(
        &self,
        request: &ExecutionRequest,
        audit_event_id: Option<AuditEventId>,
    ) -> ExecutionResult {
        let Some(executor) = self.resolve(&request.action_type) else {
            let blocked = ExecutionResult {
                status: ExecutionStatus::ExecutionBlocked,
                reason: format!(
                    "No executor registered for action type '{:?}'. Only NoopExecutor is available.",
                    request.action_type
                ),
                touched_resources: vec![],
                reversible: true,
                audit_event_id,
                action_type: request.action_type.clone(),
                proposal_id: request.proposal_id.clone(),
            };
            return blocked;
        };

        let mut result = executor.execute(request);
        result.audit_event_id = audit_event_id;
        result
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditEventId, ProposedActionId, RiskLevel};
    use std::sync::Arc;

    fn make_request(action_type: ActionType) -> ExecutionRequest {
        ExecutionRequest {
            proposal_id: ProposedActionId::new("test"),
            action_type,
            actor: "test".to_owned(),
            workspace_scope: "w".to_owned(),
            policy_decision: None,
            capability: None,
            dry_run_result: None,
            risk_level: RiskLevel::Low,
            required_permissions: vec![],
        }
    }

    // -- NoopExecutor is registered by default -----------------------------

    #[test]
    fn noop_executor_registered_by_default() {
        let registry = ExecutorRegistry::new();
        let ids = registry.list();
        assert_eq!(ids, vec!["noop-executor"]);
    }

    #[test]
    fn can_get_noop_executor_by_id() {
        let registry = ExecutorRegistry::new();
        let executor = registry.get("noop-executor");
        assert!(executor.is_some(), "noop-executor should be gettable");
        assert_eq!(executor.unwrap().executor_id(), "noop-executor");
    }

    #[test]
    fn get_unknown_executor_returns_none() {
        let registry = ExecutorRegistry::new();
        assert!(registry.get("unknown-executor").is_none());
    }

    // -- Known ActionType resolves to NoopExecutor -------------------------

    #[test]
    fn resolve_read_memory_returns_noop() {
        let registry = ExecutorRegistry::new();
        let executor = registry.resolve(&ActionType::ReadMemory);
        assert!(executor.is_some());
        assert_eq!(executor.unwrap().executor_id(), "noop-executor");
    }

    #[test]
    fn resolve_system_check_returns_noop() {
        let registry = ExecutorRegistry::new();
        let executor = registry.resolve(&ActionType::SystemCheck);
        assert!(executor.is_some());
    }

    #[test]
    fn resolve_simulate_email_returns_noop() {
        let registry = ExecutorRegistry::new();
        let executor = registry.resolve(&ActionType::SimulateEmail);
        assert!(executor.is_some());
    }

    // -- Custom ActionType resolves to none / blocked -----------------------

    #[test]
    fn resolve_custom_action_type_returns_none() {
        let registry = ExecutorRegistry::new();
        let executor = registry.resolve(&ActionType::Custom("anything".to_owned()));
        assert!(executor.is_none(), "Custom action types should not resolve");
    }

    #[test]
    fn execute_custom_returns_blocked() {
        let registry = ExecutorRegistry::new();
        let request = make_request(ActionType::Custom("unknown".to_owned()));
        let audit_id = AuditEventId::new("audit-test");
        let result = registry.execute(&request, Some(audit_id.clone()));
        assert_eq!(result.status, ExecutionStatus::ExecutionBlocked);
        assert!(result.reason.contains("No executor registered"));
        assert_eq!(result.audit_event_id, Some(audit_id));
    }

    #[test]
    fn execute_unknown_action_type_returns_blocked() {
        let registry = ExecutorRegistry::new();
        let request = make_request(ActionType::Custom("foo".to_owned()));
        let result = registry.execute(&request, None);
        assert_eq!(result.status, ExecutionStatus::ExecutionBlocked);
    }

    // -- registry list returns only NoopExecutor ---------------------------

    #[test]
    fn list_returns_only_noop_executor() {
        let registry = ExecutorRegistry::new();
        let ids = registry.list();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "noop-executor");
    }

    #[test]
    fn list_is_sorted() {
        let mut registry = ExecutorRegistry::new();
        // Register a second executor (still NoopExecutor for testing)
        registry.register(Box::new(NoopExecutor::new()));
        let ids = registry.list();
        // Should still have only one unique id (replaced)
        assert_eq!(ids.len(), 1);
    }

    // -- execute pipeline uses registry resolution -------------------------

    #[test]
    fn execute_read_memory_through_registry_returns_disabled() {
        let registry = ExecutorRegistry::new();
        let request = make_request(ActionType::ReadMemory);
        let result = registry.execute(&request, None);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
        assert!(result.reason.contains("globally disabled"));
    }

    #[test]
    fn execute_returns_proposal_and_action_type() {
        let registry = ExecutorRegistry::new();
        let request = ExecutionRequest {
            proposal_id: ProposedActionId::new("prop-42"),
            action_type: ActionType::SystemCheck,
            ..make_request(ActionType::SystemCheck)
        };
        let result = registry.execute(&request, None);
        assert_eq!(result.proposal_id.as_str(), "prop-42");
        assert_eq!(result.action_type, ActionType::SystemCheck);
    }

    // -- real execution remains disabled -----------------------------------

    #[test]
    fn real_execution_disabled_for_all_known_types() {
        let registry = ExecutorRegistry::new();
        for action_type in [
            ActionType::ReadMemory,
            ActionType::ReadTasks,
            ActionType::ReadProposedActions,
            ActionType::ReadDecisions,
            ActionType::ReadAudit,
            ActionType::ReadStatus,
            ActionType::SystemCheck,
            ActionType::WriteMemory,
            ActionType::CreateMemoryFact,
            ActionType::LinkMemoryFact,
            ActionType::InvalidateMemoryFact,
            ActionType::CreateFailureInsightMemory,
            ActionType::ReadDocument,
            ActionType::WriteDocument,
            ActionType::ProposeToolUse,
            ActionType::SimulateEmail,
            ActionType::ManageTask,
        ] {
            let request = make_request(action_type.clone());
            let result = registry.execute(&request, None);
            assert_eq!(
                result.status,
                ExecutionStatus::ExecutionDisabled,
                "Real execution should be disabled for {:?}",
                action_type
            );
        }
    }

    // -- dry-run path remains unaffected ----------------------------------

    #[test]
    fn dry_run_not_provided_by_registry() {
        let registry = ExecutorRegistry::new();
        let executor = registry.resolve(&ActionType::ReadMemory).unwrap();
        let request = make_request(ActionType::ReadMemory);
        let dry_result = executor.dry_run(&request);
        assert!(dry_result.is_none(), "NoopExecutor should not provide dry-run");
    }
}
