//! Governed direct tool-calling bridge for the cognitive runtime.
//!
//! This module connects ToolCallIntent → Decision Gate → Tool Runtime → Observation,
//! completing the governed direct tool-calling chain (Track C Step C2).
//!
//! # Target chain
//!
//! ```text
//! LLM ToolCall Intent -> Decision Gate -> Tool Runtime -> Observation -> Audit -> Reflection
//! ```
//!
//! # Safety
//!
//! - Every tool call goes through the Decision Gate before execution.
//! - Blocked calls produce audit/readback, not silent failure.
//! - Approved calls execute only through bounded Tool Runtime capabilities.
//! - Tool results return as observations, not as final authority.

use arpagona_agent_core::{
    action::ToolCallIntent, Decision, DecisionStatus, Permission, ToolExecutionResult,
};
use arpagona_decision_gate::govern_tool_call;
use arpagona_tool_runtime::ToolRuntime;

/// Result of a governed tool-call evaluation and (if approved) execution.
///
/// Carries both the governance decision and the tool execution result.
#[derive(Clone, Debug)]
pub struct GovernedToolCallResult {
    /// The Decision Gate decision.
    pub decision: Decision,
    /// The tool execution result, if the call was approved and executed.
    /// None if the call was blocked or requires human approval.
    pub execution_result: Option<ToolExecutionResult>,
    /// Human-readable summary of what happened.
    pub summary: String,
}

/// Evaluate a tool-call intent through the Decision Gate and execute it
/// through the Tool Runtime if approved.
///
/// This is the core bridge function for governed direct tool-calling.
///
/// # Flow
///
/// 1. The ToolCallIntent is wrapped as a ProposedAction with ActionType::DirectToolCall
/// 2. The Decision Gate evaluates the action against granted permissions
/// 3. If Approved, the tool is executed through the bounded Tool Runtime
/// 4. If Blocked/NeedsHumanApproval, no execution occurs but audit info is returned
/// 5. The result is returned as a structured observation
///
/// # Safety invariants
///
/// - No shell, browser, secrets, email or unrestricted write tools
/// - Tool execution only through bounded Tool Runtime capabilities
/// - Results return as observations, not as final authority
/// - Blocked calls produce readable audit data, not silent failure
pub fn govern_and_execute_tool_call(
    intent: &ToolCallIntent,
    runtime: &ToolRuntime,
    granted_permissions: &[Permission],
) -> GovernedToolCallResult {
    // Step 1: Evaluate through Decision Gate
    let (decision, _proposed_action) = govern_tool_call(intent, granted_permissions);

    // Step 2: Check the Decision Gate result
    match decision.status {
        DecisionStatus::Approved | DecisionStatus::ApprovedByOverride => {
            // Step 3: Execute through Tool Runtime
            let tool_name = &intent.tool;
            let args = &intent.arguments;

            let result = runtime.execute(tool_name, args);

            GovernedToolCallResult {
                summary: format!(
                    "Tool call '{}' was approved by Decision Gate and executed. Result: {}",
                    tool_name, result.output_summary
                ),
                decision,
                execution_result: Some(result),
            }
        }
        DecisionStatus::NeedsHumanApproval => GovernedToolCallResult {
            summary: format!(
                "Tool call '{}' requires human approval. Decision Gate: {}",
                intent.tool, decision.reason
            ),
            decision,
            execution_result: None,
        },
        DecisionStatus::Blocked | DecisionStatus::RequiresOverride => GovernedToolCallResult {
            summary: format!(
                "Tool call '{}' was blocked by Decision Gate: {}",
                intent.tool, decision.reason
            ),
            decision,
            execution_result: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::{RiskLevel, ToolExecutionStatus};
    use arpagona_tool_runtime::ToolRuntimeConfig;
    use serde_json::json;
    use tempfile::TempDir;

    fn mock_runtime(workspace: &std::path::Path) -> ToolRuntime {
        ToolRuntime::new(ToolRuntimeConfig::new(workspace))
    }

    fn read_file_intent(path: &str) -> ToolCallIntent {
        ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({ "path": path }),
            rationale: "Need to read a file".to_owned(),
            risk_level: RiskLevel::Low,
        }
    }

    fn list_files_intent(path: &str) -> ToolCallIntent {
        ToolCallIntent {
            tool: "list_files".to_owned(),
            arguments: json!({ "path": path }),
            rationale: "Need to list directory contents".to_owned(),
            risk_level: RiskLevel::Low,
        }
    }

    fn search_text_intent(query: &str, path: &str) -> ToolCallIntent {
        ToolCallIntent {
            tool: "search_text".to_owned(),
            arguments: json!({ "query": query, "path": path }),
            rationale: "Need to search for text".to_owned(),
            risk_level: RiskLevel::Low,
        }
    }

    fn tool_call_intent_missing_args() -> ToolCallIntent {
        ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({}), // no "path" argument
            rationale: "Testing malformed call".to_owned(),
            risk_level: RiskLevel::Low,
        }
    }

    fn absolute_path_intent() -> ToolCallIntent {
        ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({ "path": "/etc/passwd" }),
            rationale: "Testing security boundary".to_owned(),
            risk_level: RiskLevel::Low,
        }
    }

    fn unknown_tool_intent() -> ToolCallIntent {
        ToolCallIntent {
            tool: "shell_exec".to_owned(),
            arguments: json!({ "command": "rm -rf /" }),
            rationale: "Testing unknown tool blocking".to_owned(),
            risk_level: RiskLevel::Medium,
        }
    }

    // ── Allowed path: tool call with correct permissions is approved and executes ──

    #[test]
    fn approved_tool_call_executes_via_tool_runtime() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, governed world!").unwrap();
        let relative = "test.txt";

        let runtime = mock_runtime(dir.path());
        let intent = read_file_intent(relative);
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        assert_eq!(
            result.decision.status,
            DecisionStatus::Approved,
            "tool call with ProposeToolUse should be approved"
        );
        assert!(
            result.execution_result.is_some(),
            "approved call should execute"
        );
        let exec_result = result.execution_result.unwrap();
        assert_eq!(exec_result.status, ToolExecutionStatus::Success);
        assert!(exec_result.observation.summary.contains("Read file"));
        assert!(result.summary.contains("approved"));
    }

    #[test]
    fn approved_list_files_executes_via_tool_runtime() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "content b").unwrap();

        let runtime = mock_runtime(dir.path());
        let intent = list_files_intent(".");
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        assert_eq!(result.decision.status, DecisionStatus::Approved);
        let exec = result.execution_result.unwrap();
        assert_eq!(exec.status, ToolExecutionStatus::Success);
        assert!(exec.observation.summary.contains("entries"));
    }

    #[test]
    fn approved_search_text_executes_via_tool_runtime() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("data.txt"), "governed content here").unwrap();

        let runtime = mock_runtime(dir.path());
        let intent = search_text_intent("governed", ".");
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        assert_eq!(result.decision.status, DecisionStatus::Approved);
        let exec = result.execution_result.unwrap();
        assert_eq!(exec.status, ToolExecutionStatus::Success);
    }

    // ── Blocked path: tool call without permission returns blocked ──

    #[test]
    fn blocked_tool_call_without_permission() {
        let dir = TempDir::new().unwrap();
        let runtime = mock_runtime(dir.path());
        let intent = read_file_intent("Cargo.toml");
        // No permissions granted
        let granted: Vec<Permission> = vec![];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        assert_eq!(
            result.decision.status,
            DecisionStatus::Blocked,
            "tool call without permission should be blocked"
        );
        assert!(
            result.execution_result.is_none(),
            "blocked call should not execute"
        );
        assert!(result.summary.contains("blocked"));
        assert!(result.decision.reason.contains("permission"));
    }

    #[test]
    fn blocked_high_risk_tool_call() {
        let dir = TempDir::new().unwrap();
        let runtime = mock_runtime(dir.path());
        // Critical risk tool call — Decision Gate requires human approval
        let intent = ToolCallIntent {
            tool: "read_file".to_owned(),
            arguments: json!({ "path": "Cargo.toml" }),
            rationale: "High risk test".to_owned(),
            risk_level: RiskLevel::Critical,
        };
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        // Decision Gate returns NeedsHumanApproval for Critical risk actions
        // (not Blocked — Blocked means permission denied or policy mismatch)
        assert_eq!(
            result.decision.status,
            DecisionStatus::NeedsHumanApproval,
            "high risk tool call should require human approval"
        );
        assert!(result.execution_result.is_none());
        assert!(result.summary.contains("human approval"));
    }

    // ── Malformed path: tool call with missing arguments → failed execution ──

    #[test]
    fn malformed_tool_call_missing_arguments() {
        let dir = TempDir::new().unwrap();
        let runtime = mock_runtime(dir.path());
        let intent = tool_call_intent_missing_args();
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        // The governance should pass (permissions are fine), but execution fails
        assert_eq!(
            result.decision.status,
            DecisionStatus::Approved,
            "permissions-check should pass"
        );
        let exec = result.execution_result.expect("should have executed");
        assert_eq!(
            exec.status,
            ToolExecutionStatus::Failed,
            "execution should fail due to missing arguments"
        );
    }

    // ── Safety path: absolute path in tool-call intent is blocked ──

    #[test]
    fn absolute_path_in_tool_call_is_blocked_by_runtime() {
        let dir = TempDir::new().unwrap();
        let runtime = mock_runtime(dir.path());
        let intent = absolute_path_intent();
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        // Permissions pass (ProposeToolUse), but tool runtime blocks the absolute path
        assert_eq!(result.decision.status, DecisionStatus::Approved);
        let exec = result.execution_result.expect("should have executed");
        assert_eq!(
            exec.status,
            ToolExecutionStatus::Blocked,
            "absolute path should be blocked by tool runtime"
        );
        assert!(exec.audit_hint.contains("security"));
    }

    // ── Unknown tool path: unknown tool name → NeedsHumanApproval ──

    #[test]
    fn unknown_tool_in_tool_call_requires_human_approval() {
        let dir = TempDir::new().unwrap();
        let runtime = mock_runtime(dir.path());
        let intent = unknown_tool_intent();
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        // Medium risk + ProposeToolUse → NeedsHumanApproval (policy gate)
        // No execution happens until human approves
        assert_eq!(
            result.decision.status,
            DecisionStatus::NeedsHumanApproval,
            "unknown tool with medium risk should require human approval"
        );
        assert!(result.execution_result.is_none());
        assert!(result.summary.contains("human approval"));
        assert!(result.summary.contains("shell_exec"));
    }

    // ── Non-authorizing invariant ──

    #[test]
    fn governed_tool_call_result_is_not_authorization() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "data").unwrap();

        let runtime = mock_runtime(dir.path());
        let intent = read_file_intent("test.txt");
        let granted = vec![Permission::ProposeToolUse];

        let result = govern_and_execute_tool_call(&intent, &runtime, &granted);

        // The result contains observations, but the decision can't be used
        // to bypass future governance
        assert_eq!(result.decision.status, DecisionStatus::Approved);
        let exec = result.execution_result.unwrap();

        // The observation is NOT an authorization token
        assert!(!exec.observation.summary.contains("authorized"));
        assert!(!exec.observation.summary.contains("approved"));

        // The execution result is an observation, not final authority
        assert_eq!(exec.status, ToolExecutionStatus::Success);
        assert!(exec.observation.payload.get("content_preview").is_some());
    }
}
