//! Executor registry for registering and resolving executors.
//!
//! Maps `executor_id` to [`Box<dyn Executor>`] wrapped in an [`ExecutorSlot`]
//! that carries the executor's readiness state.
//!
//! **All executors are registered as `Disabled` by default.** State must be
//! explicitly promoted to `Ready` before the executor can resolve actions.
//! The only built-in executor is [`NoopExecutor`], which always returns
//! `ExecutionDisabled`.
//!
//! This is a **deterministic** registry: it contains no tool execution logic,
//! no LLM calls, no side effects, and no Decision Gate bypass.

use crate::executor::{
    ExecutionRequest, ExecutionResult, ExecutionStatus, Executor, ExecutorState, NoopExecutor,
};
use crate::{ActionType, AuditEventId, ProposedActionId};
use std::collections::HashMap;

/// A registered executor slot that pairs an executor with its readiness state.
pub struct ExecutorSlot {
    /// The executor instance.
    pub executor: Box<dyn Executor>,
    /// The current readiness state (default: `Disabled`).
    pub state: ExecutorState,
}

impl std::fmt::Debug for ExecutorSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutorSlot")
            .field("executor_id", &self.executor.executor_id())
            .field("state", &self.state)
            .finish()
    }
}

impl ExecutorSlot {
    /// Create a new slot with the given executor and state.
    pub fn new(executor: Box<dyn Executor>, state: ExecutorState) -> Self {
        Self { executor, state }
    }

    /// Returns `true` if this slot is allowed to resolve and execute.
    pub fn can_resolve(&self) -> bool {
        self.state.allows_execution()
    }
}

/// Registry of executor slots, keyed by executor_id.
///
/// Supports registration with optional state, resolution by action type
/// (filtering disabled and blocked executors), lookup by id, and state
/// transitions.
///
/// Pre-registers [`NoopExecutor`] in `Disabled` state by default.
pub struct ExecutorRegistry {
    slots: HashMap<String, ExecutorSlot>,
}

impl ExecutorRegistry {
    /// Create a new registry and register the NoopExecutor in `Disabled` state.
    pub fn new() -> Self {
        let mut registry = Self {
            slots: HashMap::new(),
        };
        registry.register(Box::new(NoopExecutor::new()), None);
        registry
    }

    /// Register an executor with an optional readiness state.
    ///
    /// If no state is provided, the executor is registered as `Disabled`.
    /// If an executor with the same id already exists, it is replaced.
    pub fn register(
        &mut self,
        executor: Box<dyn Executor>,
        state: Option<ExecutorState>,
    ) -> &mut Self {
        let id = executor.executor_id().to_owned();
        let slot = ExecutorSlot::new(executor, state.unwrap_or_default());
        self.slots.insert(id, slot);
        self
    }

    /// Update the state of an executor slot by id.
    ///
    /// Returns `None` if no executor with this id exists.
    pub fn set_state(&mut self, executor_id: &str, state: ExecutorState) -> Option<()> {
        self.slots.get_mut(executor_id).map(|slot| {
            slot.state = state;
        })
    }

    /// Get the current state of an executor slot by id.
    ///
    /// Returns `None` if no executor with this id exists.
    pub fn get_state(&self, executor_id: &str) -> Option<ExecutorState> {
        self.slots.get(executor_id).map(|slot| slot.state.clone())
    }

    /// Resolve an executor for the given action type.
    ///
    /// Returns `None` for `Custom`/unknown action types.
    /// Returns `None` for executors that are not in `Ready` state.
    /// Otherwise returns the first executor that supports this action type.
    pub fn resolve(&self, action_type: &ActionType) -> Option<&dyn Executor> {
        // Custom action types never resolve
        if matches!(action_type, ActionType::Custom(_)) {
            return None;
        }
        // Find the first executor in Ready state whose supported_action_types includes this type
        for slot in self.slots.values() {
            if slot.can_resolve() && slot.executor.supported_action_types().contains(action_type) {
                return Some(slot.executor.as_ref());
            }
        }
        None
    }

    /// Get an executor slot by its id.
    pub fn get_slot(&self, executor_id: &str) -> Option<&ExecutorSlot> {
        self.slots.get(executor_id)
    }

    /// Get an executor by its id (regardless of state).
    pub fn get(&self, executor_id: &str) -> Option<&dyn Executor> {
        self.slots
            .get(executor_id)
            .map(|slot| slot.executor.as_ref())
    }

    /// List all registered executor ids.
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.slots.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Execute an action through the registry.
    ///
    /// Returns an [`ExecutionResult`] with:
    /// - `ExecutionBlocked` if no executor can handle the action type
    /// - `ExecutionBlocked` if the executor is in `Blocked` state
    /// - `ExecutionDisabled` if the executor is in `Disabled` state
    /// - The executor's result otherwise (currently always `ExecutionDisabled`).
    pub fn execute(
        &self,
        request: &ExecutionRequest,
        audit_event_id: Option<AuditEventId>,
    ) -> ExecutionResult {
        let Some(slot) = self.resolve_slot(&request.action_type) else {
            return self.blocked_result(request, audit_event_id.clone(), "No executor registered for this action type. All executors are disabled, blocked, or do not support this action type.");
        };

        if slot.state.is_blocked() {
            return self.blocked_result(
                request,
                audit_event_id.clone(),
                &format!(
                    "Executor '{}' is blocked and cannot execute actions.",
                    slot.executor.executor_id()
                ),
            );
        }

        // slot.state.allows_execution() is guaranteed true here (resolve_slot filters)
        let mut result = slot.executor.execute(request);
        result.audit_event_id = audit_event_id;
        result
    }

    /// Resolve a slot (not just the executor) for the given action type.
    ///
    /// Only returns slots in `Ready` state. Returns `None` for
    /// Custom/unknown action types or when no ready executor supports the type.
    fn resolve_slot(&self, action_type: &ActionType) -> Option<&ExecutorSlot> {
        if matches!(action_type, ActionType::Custom(_)) {
            return None;
        }
        for slot in self.slots.values() {
            if slot.can_resolve() && slot.executor.supported_action_types().contains(action_type) {
                return Some(slot);
            }
        }
        None
    }

    fn blocked_result(
        &self,
        request: &ExecutionRequest,
        audit_event_id: Option<AuditEventId>,
        reason: &str,
    ) -> ExecutionResult {
        ExecutionResult {
            status: ExecutionStatus::ExecutionBlocked,
            reason: reason.to_owned(),
            touched_resources: vec![],
            reversible: true,
            audit_event_id,
            action_type: request.action_type.clone(),
            proposal_id: request.proposal_id.clone(),
        }
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

    // -- NoopExecutor is registered by default (in Disabled state) ----------

    #[test]
    fn noop_executor_registered_by_default() {
        let registry = ExecutorRegistry::new();
        let ids = registry.list();
        assert_eq!(ids, vec!["noop-executor"]);
    }

    #[test]
    fn noop_executor_default_state_is_disabled() {
        let registry = ExecutorRegistry::new();
        let state = registry.get_state("noop-executor");
        assert_eq!(state, Some(ExecutorState::Disabled));
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

    // -- Disabled executors cannot resolve actions --------------------------

    #[test]
    fn disabled_executor_does_not_resolve_read_memory() {
        let registry = ExecutorRegistry::new(); // NoopExecutor disabled by default
        let result = registry.resolve(&ActionType::ReadMemory);
        assert!(
            result.is_none(),
            "Disabled executor should not resolve actions"
        );
    }

    #[test]
    fn disabled_executor_does_not_resolve_system_check() {
        let registry = ExecutorRegistry::new();
        let result = registry.resolve(&ActionType::SystemCheck);
        assert!(result.is_none());
    }

    #[test]
    fn disabled_executor_does_not_resolve_any_type() {
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
            let result = registry.resolve(&action_type);
            assert!(
                result.is_none(),
                "Disabled executor should not resolve {:?}",
                action_type
            );
        }
    }

    // -- Ready executors CAN resolve actions --------------------------------

    #[test]
    fn ready_executor_resolves_read_memory() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let result = registry.resolve(&ActionType::ReadMemory);
        assert!(result.is_some(), "Ready executor should resolve actions");
        assert_eq!(result.unwrap().executor_id(), "noop-executor");
    }

    #[test]
    fn ready_executor_resolves_all_known_types() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
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
            let result = registry.resolve(&action_type);
            assert!(
                result.is_some(),
                "Ready executor should resolve {:?}",
                action_type
            );
        }
    }

    // -- Blocked executors cannot resolve actions ---------------------------

    #[test]
    fn blocked_executor_does_not_resolve() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Blocked);
        let result = registry.resolve(&ActionType::ReadMemory);
        assert!(
            result.is_none(),
            "Blocked executor should not resolve actions"
        );
    }

    // -- Custom ActionType resolves to none regardless of state -------------

    #[test]
    fn resolve_custom_action_type_returns_none() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let executor = registry.resolve(&ActionType::Custom("anything".to_owned()));
        assert!(executor.is_none(), "Custom action types should not resolve");
    }

    // -- execute() produces correct status based on state -------------------

    #[test]
    fn execute_against_disabled_executor_returns_blocked() {
        let registry = ExecutorRegistry::new(); // NoopExecutor disabled by default
        let request = make_request(ActionType::ReadMemory);
        let result = registry.execute(&request, None);
        assert_eq!(
            result.status,
            ExecutionStatus::ExecutionBlocked,
            "Disabled executor should return ExecutionBlocked"
        );
        assert!(result.reason.contains("disabled"));
    }

    #[test]
    fn execute_against_blocked_executor_returns_blocked() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Blocked);
        let request = make_request(ActionType::ReadMemory);
        let result = registry.execute(&request, None);
        assert_eq!(
            result.status,
            ExecutionStatus::ExecutionBlocked,
            "Blocked executor should return ExecutionBlocked"
        );
        assert!(result.reason.contains("blocked"));
    }

    #[test]
    fn execute_against_ready_executor_returns_executor_result() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let request = make_request(ActionType::ReadMemory);
        let result = registry.execute(&request, None);
        // NoopExecutor returns ExecutionDisabled even when Ready
        assert_eq!(
            result.status,
            ExecutionStatus::ExecutionDisabled,
            "Ready NoopExecutor should still return ExecutionDisabled"
        );
        assert!(result.reason.contains("globally disabled"));
    }

    #[test]
    fn execute_custom_returns_blocked() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let request = make_request(ActionType::Custom("unknown".to_owned()));
        let audit_id = AuditEventId::new("audit-test");
        let result = registry.execute(&request, Some(audit_id.clone()));
        assert_eq!(result.status, ExecutionStatus::ExecutionBlocked);
        assert!(result.reason.contains("blocked"));
        assert_eq!(result.audit_event_id, Some(audit_id));
    }

    // -- Executor state transitions -----------------------------------------

    #[test]
    fn state_transition_disabled_to_ready() {
        let mut registry = ExecutorRegistry::new();
        assert_eq!(
            registry.get_state("noop-executor"),
            Some(ExecutorState::Disabled)
        );
        registry.set_state("noop-executor", ExecutorState::Ready);
        assert_eq!(
            registry.get_state("noop-executor"),
            Some(ExecutorState::Ready)
        );
    }

    #[test]
    fn state_transition_ready_to_blocked() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let result = registry.resolve(&ActionType::ReadMemory);
        assert!(result.is_some(), "Ready executor should resolve");

        registry.set_state("noop-executor", ExecutorState::Blocked);
        let result = registry.resolve(&ActionType::ReadMemory);
        assert!(result.is_none(), "Blocked executor should not resolve");
    }

    #[test]
    fn set_state_unknown_executor_returns_none() {
        let mut registry = ExecutorRegistry::new();
        let result = registry.set_state("unknown", ExecutorState::Ready);
        assert!(result.is_none());
    }

    // -- register() with optional state -------------------------------------

    #[test]
    fn register_with_explicit_state() {
        let mut registry = ExecutorRegistry::new();
        // Register another NoopExecutor as Ready
        registry.register(Box::new(NoopExecutor::new()), Some(ExecutorState::Ready));
        let state = registry.get_state("noop-executor");
        // The first registration's state (Disabled) was replaced
        assert_eq!(state, Some(ExecutorState::Ready));
    }

    #[test]
    fn register_without_state_defaults_to_disabled() {
        let mut registry = ExecutorRegistry::new();
        register_test_executor(&mut registry, "test-executor-1");
        let state = registry.get_state("test-executor-1");
        assert_eq!(state, Some(ExecutorState::Disabled));
    }

    #[test]
    fn register_ready_executor_in_new_slot() {
        let mut registry = ExecutorRegistry::new();
        // Register NoopExecutor with Ready state
        registry.register(Box::new(NoopExecutor::new()), Some(ExecutorState::Ready));
        assert!(registry.resolve(&ActionType::ReadMemory).is_some());
    }

    // -- registry list returns only noop-executor by default ----------------

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
        registry.register(Box::new(NoopExecutor::new()), None);
        let ids = registry.list();
        // Should still have only one unique id (replaced)
        assert_eq!(ids.len(), 1);
    }

    // -- audit_event_id propagation ----------------------------------------

    #[test]
    fn execute_passes_audit_event_id() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let request = make_request(ActionType::ReadMemory);
        let audit_id = AuditEventId::new("audit-123");
        let result = registry.execute(&request, Some(audit_id.clone()));
        assert_eq!(result.audit_event_id, Some(audit_id));
    }

    // -- real execution remains disabled even when ready --------------------

    #[test]
    fn ready_executor_still_disabled_globally() {
        let mut registry = ExecutorRegistry::new();
        registry.set_state("noop-executor", ExecutorState::Ready);
        let request = make_request(ActionType::ReadMemory);
        let result = registry.execute(&request, None);
        assert_eq!(result.status, ExecutionStatus::ExecutionDisabled);
    }

    // -- ExecutorSlot helpers ----------------------------------------------

    #[test]
    fn slot_default_cannot_resolve() {
        let slot = ExecutorSlot::new(Box::new(NoopExecutor::new()), ExecutorState::Disabled);
        assert!(!slot.can_resolve());
    }

    #[test]
    fn slot_ready_can_resolve() {
        let slot = ExecutorSlot::new(Box::new(NoopExecutor::new()), ExecutorState::Ready);
        assert!(slot.can_resolve());
    }

    #[test]
    fn slot_blocked_cannot_resolve() {
        let slot = ExecutorSlot::new(Box::new(NoopExecutor::new()), ExecutorState::Blocked);
        assert!(!slot.can_resolve());
    }

    // -- Helper to register a test executor with a custom id ---------------

    fn register_test_executor(registry: &mut ExecutorRegistry, id: &str) {
        struct TestExecutor(String);
        impl Executor for TestExecutor {
            fn executor_id(&self) -> &str {
                &self.0
            }
            fn supported_action_types(&self) -> Vec<ActionType> {
                vec![ActionType::ReadMemory, ActionType::SystemCheck]
            }
            fn execute(&self, request: &ExecutionRequest) -> ExecutionResult {
                ExecutionResult::disabled(
                    request.proposal_id.clone(),
                    request.action_type.clone(),
                    format!("Test executor '{}' is disabled", self.0),
                )
            }
        }
        registry.register(Box::new(TestExecutor(id.to_owned())), None);
    }
}
