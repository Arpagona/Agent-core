use crate::ids::{ToolExecutionId, ToolId};
use crate::permission::Permission;
use crate::risk::RiskLevel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Available,
    Disabled,
    Deprecated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub required_permissions: Vec<Permission>,
    pub default_risk_level: RiskLevel,
    pub status: ToolStatus,
}

// ---------------------------------------------------------------------------
// Cognitive Tool Vocabulary — pure domain types for tool intent,
// execution request/result, observation, and rationale.
// ---------------------------------------------------------------------------

/// Why a tool is being requested in cognitive terms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitivePurpose {
    Perception,
    Recall,
    Inspection,
    Transformation,
    Validation,
    Execution,
    Communication,
    Reflection,
}

/// The execution mode requested or authorised for a tool use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionMode {
    /// Dry-run / audit-only — the tool will report what it would do without
    /// performing side effects. For read-only tools this is equivalent to
    /// normal execution; for write tools this avoids mutation.
    Simulate,
    /// Full execution under governance.
    Execute,
    /// Execution is deferred until a human explicitly confirms.
    RequireHumanConfirmation,
}

/// Level of risk associated with a specific tool use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Strategy to fall back to if the tool cannot be used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackStrategy {
    /// Retry with the same tool.
    Retry,
    /// Use an alternative tool.
    UseAlternative(String),
    /// Report the failure without retrying.
    ReportOnly,
    /// Escalate to a human operator.
    EscalateToHuman,
}

/// Full rationale for why a specific tool is the right choice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolUseRationale {
    /// Short explanation of why this tool is relevant.
    pub justification: String,
    /// What observation we expect the tool to produce.
    pub expected_observation: String,
    /// How the observation will be used downstream.
    pub downstream_use: String,
    /// What could go wrong.
    pub risk_assessment: String,
    /// Alternative tool(s) if this one fails or is unavailable.
    pub fallback_strategy: FallbackStrategy,
}

/// A fully-formed intention to use a tool for a cognitive purpose.
///
/// `ToolIntent` is a pure-domain declaration. It does not request execution,
/// does not authorise anything, and does not bypass the Decision Gate.
/// It is the structured answer to "what tool should I use, why, and what do
/// I expect to learn?"
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolIntent {
    /// Name of the tool to use (e.g. "read_file", "search_text").
    pub tool_name: String,
    /// The cognitive role this tool plays.
    pub cognitive_purpose: CognitivePurpose,
    /// Why this tool is the right choice.
    pub rationale: ToolUseRationale,
    /// What observation we expect to receive.
    pub expected_observation: String,
    /// Arguments to pass to the tool.
    pub arguments: Value,
    /// Risk level assessed for this specific use.
    pub risk_level: ToolRiskLevel,
    /// What to do if the tool is unavailable or fails.
    pub fallback_strategy: FallbackStrategy,
}

/// A concrete execution request for a tool.
///
/// This is produced from a `ToolIntent` after a decision to proceed has been
/// made (or in demo/alpha mode where governance is bypassed explicitly).
/// It carries the execution mode and governance linkage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub id: ToolExecutionId,
    pub tool_name: String,
    pub mode: ToolExecutionMode,
    pub arguments: Value,
    pub created_at: DateTime<Utc>,
}

/// Status of a tool execution attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    /// The tool ran and produced an observation.
    Success,
    /// The tool ran but produced a warning-level result.
    Warning,
    /// The tool failed to produce a useful observation.
    Failed,
    /// The tool was blocked by a security constraint before execution.
    Blocked,
    /// The tool was skipped (e.g. fallback strategy chose not to run).
    Skipped,
}

/// Structured error from a tool execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionError {
    pub code: String,
    pub message: String,
    pub is_security: bool,
}

impl ToolExecutionError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            is_security: false,
        }
    }

    pub fn security(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            is_security: true,
        }
    }
}

/// An observation produced by a tool execution.
///
/// Observations are the bridge between tool output and cognitive processing.
/// They carry structured data plus metadata about whether the observation
/// is usable as-is or should trigger a FailureInsight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolObservation {
    /// Free-form description of what was observed.
    pub summary: String,
    /// Structured payload from the tool.
    pub payload: Value,
    /// Whether the observation can be used directly.
    pub actionable: bool,
    /// If not actionable, whether a FailureInsight should be produced.
    pub failure_insight_candidate: bool,
    /// Hint about what kind of FailureInsight would be appropriate.
    pub failure_hint: Option<String>,
}

/// Complete structured result of a tool execution.
///
/// This is the single output type for all tool executions in the runtime.
/// It carries enough context for audit, reflection, and FailureInsight
/// generation without exposing raw shell output or internal state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub execution_id: ToolExecutionId,
    pub tool_name: String,
    pub status: ToolExecutionStatus,
    pub observation: ToolObservation,
    pub output_summary: String,
    pub error: Option<ToolExecutionError>,
    /// Hint for audit about what to record.
    pub audit_hint: String,
    /// If true, the runtime should consider creating a FailureInsight.
    pub failure_insight_candidate: bool,
    pub executed_at: DateTime<Utc>,
}

impl ToolExecutionResult {
    pub fn success(
        execution_id: ToolExecutionId,
        tool_name: impl Into<String>,
        observation: ToolObservation,
        audit_hint: impl Into<String>,
    ) -> Self {
        Self {
            execution_id,
            tool_name: tool_name.into(),
            status: ToolExecutionStatus::Success,
            output_summary: observation.summary.clone(),
            observation,
            error: None,
            audit_hint: audit_hint.into(),
            failure_insight_candidate: false,
            executed_at: Utc::now(),
        }
    }

    pub fn warning(
        execution_id: ToolExecutionId,
        tool_name: impl Into<String>,
        observation: ToolObservation,
        audit_hint: impl Into<String>,
    ) -> Self {
        Self {
            execution_id,
            tool_name: tool_name.into(),
            status: ToolExecutionStatus::Warning,
            output_summary: observation.summary.clone(),
            observation,
            error: None,
            audit_hint: audit_hint.into(),
            failure_insight_candidate: false,
            executed_at: Utc::now(),
        }
    }

    pub fn failed(
        execution_id: ToolExecutionId,
        tool_name: impl Into<String>,
        error: ToolExecutionError,
        audit_hint: impl Into<String>,
    ) -> Self {
        let is_failure_candidate = !error.is_security;
        Self {
            execution_id,
            tool_name: tool_name.into(),
            status: ToolExecutionStatus::Failed,
            observation: ToolObservation {
                summary: error.message.clone(),
                payload: Value::Null,
                actionable: false,
                failure_insight_candidate: is_failure_candidate,
                failure_hint: Some(error.code.clone()),
            },
            output_summary: format!("Error: {}", error.message),
            error: Some(error),
            audit_hint: audit_hint.into(),
            failure_insight_candidate: is_failure_candidate,
            executed_at: Utc::now(),
        }
    }

    pub fn blocked(
        execution_id: ToolExecutionId,
        tool_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let reason = reason.into();
        Self {
            execution_id,
            tool_name: tool_name.into(),
            status: ToolExecutionStatus::Blocked,
            observation: ToolObservation {
                summary: reason.clone(),
                payload: Value::Null,
                actionable: false,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            output_summary: format!("Blocked: {reason}"),
            error: Some(ToolExecutionError::security("blocked", reason.clone())),
            audit_hint: format!("Blocked by security: {reason}"),
            failure_insight_candidate: false,
            executed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_intent_serializes_and_deserializes() {
        let intent = ToolIntent {
            tool_name: "read_file".to_owned(),
            cognitive_purpose: CognitivePurpose::Inspection,
            rationale: ToolUseRationale {
                justification: "Need to inspect config file".to_owned(),
                expected_observation: "File content".to_owned(),
                downstream_use: "Validate configuration".to_owned(),
                risk_assessment: "Read-only, low risk".to_owned(),
                fallback_strategy: FallbackStrategy::ReportOnly,
            },
            expected_observation: "lines of text".to_owned(),
            arguments: json!({"path": "Cargo.toml"}),
            risk_level: ToolRiskLevel::None,
            fallback_strategy: FallbackStrategy::ReportOnly,
        };

        let encoded = serde_json::to_value(&intent).expect("intent should serialize");
        assert_eq!(encoded["tool_name"], "read_file");
        assert_eq!(encoded["cognitive_purpose"], "inspection");
        assert_eq!(encoded["risk_level"], "none");

        let decoded: ToolIntent =
            serde_json::from_value(encoded).expect("intent should deserialize");
        assert_eq!(decoded.tool_name, "read_file");
        assert_eq!(decoded.cognitive_purpose, CognitivePurpose::Inspection);
    }

    #[test]
    fn tool_execution_result_success_is_not_a_failure_candidate() {
        let result = ToolExecutionResult::success(
            ToolExecutionId::new("exec-1"),
            "read_file",
            ToolObservation {
                summary: "Found 42 lines".to_owned(),
                payload: json!({"lines": 42}),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            "Read file successfully",
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert!(!result.failure_insight_candidate);
        assert!(result.error.is_none());
    }

    #[test]
    fn tool_execution_result_blocked_is_security_error() {
        let result = ToolExecutionResult::blocked(
            ToolExecutionId::new("exec-2"),
            "read_file",
            "Path outside workspace",
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(!result.failure_insight_candidate);
    }

    #[test]
    fn tool_observation_can_mark_failure_insight_candidate() {
        let observation = ToolObservation {
            summary: "Tool returned no results".to_owned(),
            payload: json!({"matches": 0}),
            actionable: false,
            failure_insight_candidate: true,
            failure_hint: Some("empty_result".to_string()),
        };

        assert!(!observation.actionable);
        assert!(observation.failure_insight_candidate);
        assert_eq!(observation.failure_hint, Some("empty_result".to_owned()));
    }

    #[test]
    fn tool_execution_result_is_not_an_authorization() {
        let result = ToolExecutionResult::success(
            ToolExecutionId::new("exec-3"),
            "read_file",
            ToolObservation {
                summary: "ok".to_owned(),
                payload: json!({"lines": 1}),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            "Read file",
        );

        let encoded = serde_json::to_value(&result).expect("result should serialize");
        assert!(!encoded.to_string().contains("authorized"));
        assert!(!encoded.to_string().contains("approved"));
    }
}
