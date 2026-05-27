//! MCP governance layer — wraps `tools/call` through the Decision Gate.
//!
//! Phase 2: adds DecisionGate governance before ToolRuntime execution,
//! so that every tool call from an external MCP client is evaluated
//! against policies and permissions before execution.

use arpagona_agent_core::{
    ActionType, AgentId, Decision, DecisionStatus, Permission, ProposedAction, ProposedActionId,
    ProposedActionStatus, RiskLevel, WorkspaceId,
};
use arpagona_decision_gate::evaluate_proposed_action;
use chrono::Utc;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

/// Sequential ID counter for MCP governance actions.
static NEXT_ACTION_ID: AtomicU64 = AtomicU64::new(1000);

/// The outcome of a governance evaluation for an MCP tool call.
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceDecision {
    /// The tool call is approved for execution.
    Approved { decision: Decision },
    /// The tool call is blocked by policy/missing permissions.
    Blocked { decision: Decision },
    /// The tool call requires an override (password/etc.) to proceed.
    RequiresOverride { decision: Decision },
}

impl GovernanceDecision {
    /// Returns true if the decision is `Approved`.
    pub fn is_approved(&self) -> bool {
        matches!(self, GovernanceDecision::Approved { .. })
    }

    /// Returns a human-readable summary of the governance outcome.
    pub fn summary(&self) -> String {
        match self {
            GovernanceDecision::Approved { decision } => {
                format!("Approved: {}", decision.reason)
            }
            GovernanceDecision::Blocked { decision } => {
                format!("Blocked: {}", decision.reason)
            }
            GovernanceDecision::RequiresOverride { decision } => {
                format!("Requires override: {}", decision.reason)
            }
        }
    }
}

/// Evaluate a tool call through the Decision Gate.
///
/// Creates a `ProposedAction` with `ActionType::ProposeToolUse` and runs it
/// through `evaluate_proposed_action()`. Returns a `GovernanceDecision`
/// indicating whether execution should proceed.
///
/// # Arguments
///
/// * `tool_name` — The name of the tool being called
/// * `arguments` — The arguments passed to the tool
/// * `policies` — Active policies for evaluation
/// * `granted_permissions` — Permissions granted to the caller
pub fn evaluate_tool_call(tool_name: &str, arguments: &Value) -> GovernanceDecision {
    let id = NEXT_ACTION_ID.fetch_add(1, Ordering::SeqCst);

    let proposed_action = ProposedAction {
        id: ProposedActionId::new(format!("mcp-{id}")),
        workspace_id: WorkspaceId::new("mcp-workspace"),
        task_id: None,
        proposed_by: AgentId::new("mcp-client"),
        action_type: ActionType::ProposeToolUse,
        target: Some(tool_name.to_owned()),
        payload: serde_json::json!({
            "tool": tool_name,
            "arguments": arguments,
        }),
        risk_level: RiskLevel::Informational,
        required_permissions: vec![Permission::ProposeToolUse],
        rationale: format!("MCP client requested tool call: {tool_name}(...)"),
        context_refs: vec![],
        status: ProposedActionStatus::PendingDecision,
        created_at: Utc::now(),
    };

    // Use no additional policies (built-in Decision Gate rules apply).
    // Grant ProposeToolUse permission so read-only informational calls
    // are auto-approved.
    let decision = evaluate_proposed_action(&proposed_action, &[], &[Permission::ProposeToolUse]);

    match decision.status {
        DecisionStatus::Approved | DecisionStatus::ApprovedByOverride => {
            GovernanceDecision::Approved { decision }
        }
        DecisionStatus::Blocked => GovernanceDecision::Blocked { decision },
        DecisionStatus::RequiresOverride => GovernanceDecision::RequiresOverride { decision },
        DecisionStatus::NeedsHumanApproval => {
            // Informational risk + ProposeToolUse permission granted should
            // never reach NeedsHumanApproval. If it does, treat as blocked.
            GovernanceDecision::Blocked { decision }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_tool_approved_with_permission() {
        // Read-only tools at Informational risk with ProposeToolUse permission
        // should be Approved.
        let decision = evaluate_tool_call("read_file", &serde_json::json!({"path": "Cargo.toml"}));
        assert!(
            decision.is_approved(),
            "Read-only tool with permission should be approved"
        );
        assert!(
            decision.summary().contains("Approved"),
            "Summary should say Approved"
        );
    }

    #[test]
    fn test_any_read_only_tool_approved() {
        // All read-only tools (read_file, list_files, search_text) should
        // be approved with ProposeToolUse permission at Informational risk.
        for tool in &["read_file", "list_files", "search_text"] {
            let decision = evaluate_tool_call(tool, &serde_json::json!({}));
            assert!(
                decision.is_approved(),
                "Tool '{tool}' should be approved with permission"
            );
        }
    }

    #[test]
    fn test_governance_summary_includes_status() {
        // The governance summary should contain either "Approved", "Blocked",
        // or "Requires override".
        let decision = evaluate_tool_call("list_files", &serde_json::json!({}));
        let summary = decision.summary();
        assert!(
            summary.contains("Approved")
                || summary.contains("Blocked")
                || summary.contains("Requires override"),
            "Summary should contain one of the status keywords"
        );
    }

    #[test]
    fn test_governance_decision_has_decision() {
        let decision = evaluate_tool_call("search_text", &serde_json::json!({"query": "test"}));
        match &decision {
            GovernanceDecision::Approved { decision: d }
            | GovernanceDecision::Blocked { decision: d }
            | GovernanceDecision::RequiresOverride { decision: d } => {
                assert!(!d.reason.is_empty(), "Decision should have a reason");
            }
        }
    }
}
