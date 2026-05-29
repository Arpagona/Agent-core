//! Cognitive Observation Pipeline — pure domain types for turning tool
//! execution results into structured, assessable cognitive observations.
//!
//! This is the bridge between raw tool output and cognitive processing:
//!
//! ```text
//! ToolExecutionResult
//!     ↓ CognitiveObservation::from_tool_execution()
//! CognitiveObservation
//!     ↓ assess_observation()
//! ObservationAssessment
//!     ↓ (candidate detection)
//! Option<FailureInsightCandidate>
//! ```
//!
//! # Safety invariants
//!
//! - All types are pure domain: no I/O, no system access, no side effects.
//! - Conversion from ToolExecutionResult is explicit — not a trait that
//!   auto-executes or auto-authorizes anything.
//! - `assess_observation()` is a pure function that classifies observations;
//!   it does not create, persist, or route FailureInsight records.
//! - A security boundary block is classified as a positive observation
//!   ("the safety boundary functioned correctly"), not as a system failure.
//! - No type in this module authorizes actions, writes to memory, or
//!   bypasses the Decision Gate.

use crate::tool::{ToolExecutionResult, ToolExecutionStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Primary types
// ---------------------------------------------------------------------------

/// Where a cognitive observation originated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    /// The observation came from a read-only tool execution.
    ToolExecution,
    /// The observation came from direct human input (chat, review).
    HumanInput,
    /// The observation came from an audit event replay.
    AuditEvent,
    /// The observation came from a system signal (health, heartbeat).
    SystemSignal,
    /// The observation came from the agent's own reflection loop.
    Reflection,
}

/// What kind of content the observation carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    /// File content read from disk.
    FileContent,
    /// A directory listing.
    DirectoryListing,
    /// Text search results.
    SearchResult,
    /// An error or failure signal.
    Error,
    /// A security boundary was triggered (positive signal).
    SecurityBoundary,
    /// Informational or status observation.
    Informational,
}

/// The status of a cognitive observation.
///
/// This aligns with but extends [`ToolExecutionStatus`] to capture nuances
/// like partial results and truncation that matter for cognitive processing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    /// The tool completed successfully with a useful result.
    Success,
    /// The tool returned no results (empty output, zero matches).
    Empty,
    /// The tool returned partial results (not all data could be collected).
    Partial,
    /// The result was truncated due to size or count limits.
    Truncated,
    /// The tool was blocked by a security constraint.
    Blocked,
    /// The tool failed to produce a result.
    Failed,
    /// The tool produced a warning-level result that needs attention.
    Warning,
}

/// How useful this observation is for downstream cognitive processing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationUsefulness {
    /// The observation carries no actionable information.
    None,
    /// The observation has minimal signal but some metadata.
    Low,
    /// The observation is useful but may need synthesis with other data.
    Medium,
    /// The observation is directly valuable for current processing.
    High,
    /// The observation is ready to drive an immediate decision or action.
    DirectlyActionable,
}

/// Risk level associated with acting on this observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationRisk {
    /// No detectable risk.
    None,
    /// Low risk — routine inspection result.
    Low,
    /// Medium risk — observation contains unverified or ambiguous data.
    Medium,
    /// High risk — observation suggests a potentially unsafe condition.
    High,
    /// Security-tagged — observation was blocked or touched a sensitive boundary.
    Security,
}

/// Kinds of candidate that could trigger a FailureInsight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureInsightCandidateKind {
    /// A tool use was blocked by a security constraint.
    BlockedToolUse,
    /// The agent lacked necessary context to complete the operation.
    MissingContext,
    /// A search or list returned zero results.
    EmptySearchResult,
    /// A result was truncated before all data could be read.
    TruncatedResult,
    /// The result was ambiguous or could not be interpreted reliably.
    AmbiguousResult,
    /// A safety boundary was triggered (informational, not a failure).
    SafetyBoundaryTriggered,
    /// The tool runtime itself failed.
    ToolRuntimeFailure,
    /// The documentation or expected behaviour did not match reality.
    DocumentationMismatch,
    /// The same operation was tried repeatedly without progress.
    RepeatedOperatorFriction,
    /// The observation quality was too low to be useful.
    InsufficientObservationQuality,
    /// Context assembly in the orchestrator cycle returned weak or zero results.
    ContextAssemblyWeak,
}

// ---------------------------------------------------------------------------
// Core observation types
// ---------------------------------------------------------------------------

/// A structured cognitive observation produced by processing a tool execution
/// result through the cognitive pipeline.
///
/// Unlike [`ToolObservation`], which carries raw output, `CognitiveObservation`
/// adds interpretation: what kind of observation it is, how useful it is,
/// whether it should trigger a FailureInsight, and what the risk level is.
///
/// # Safety
///
/// - Creation does not execute anything.
/// - The observation is descriptive, not prescriptive.
/// - `failure_insight_candidate` is a flag, not a creation action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitiveObservation {
    /// The name of the tool that produced this observation.
    pub tool_name: String,
    /// Where this observation came from.
    pub source: ObservationSource,
    /// What kind of content this observation carries.
    pub kind: ObservationKind,
    /// The status of the observation.
    pub status: ObservationStatus,
    /// How useful this observation is.
    pub usefulness: ObservationUsefulness,
    /// Risk level of acting on this observation.
    pub risk: ObservationRisk,
    /// Human-readable summary of what was observed.
    pub summary: String,
    /// The raw payload from the tool (preserved for downstream use).
    pub payload: Value,
    /// Whether the result was truncated.
    pub truncated: bool,
    /// Number of items/records/lines in the result.
    pub count: usize,
    /// Free-form detail about what was observed (may include error messages).
    pub detail: String,
    /// Whether this observation should be considered a FailureInsight candidate.
    pub failure_insight_candidate: bool,
    /// If a candidate, what kind of candidate.
    pub candidate_kind: Option<FailureInsightCandidateKind>,
    /// Human-readable explanation of why this is (or is not) a candidate.
    pub candidate_reason: String,
}

// ---------------------------------------------------------------------------
// Assessment types
// ---------------------------------------------------------------------------

/// An evaluated cognitive observation with actionable metadata.
///
/// `ObservationAssessment` is the result of passing a `CognitiveObservation`
/// through the assessment function. It classifies the observation and
/// optionally produces a `FailureInsightCandidate` if warranted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationAssessment {
    /// The original observation.
    pub observation: CognitiveObservation,
    /// Whether this observation is useful for downstream processing.
    pub is_useful: bool,
    /// Whether this observation is complete enough to act on.
    pub is_complete: bool,
    /// Whether this observation should produce a FailureInsight.
    pub failure_insight_candidate: bool,
    /// The candidate, if one was produced.
    pub candidate: Option<FailureInsightCandidate>,
    /// Human-readable assessment summary.
    pub assessment_summary: String,
    /// Suggested next step (descriptive only, no execution).
    pub suggested_next_step: String,
}

/// A lightweight candidate for FailureInsight creation.
///
/// This is not a full [`FailureInsight`](crate::failure_insight::FailureInsight) — it is a
/// signal that the cognitive loop *might* want to create one. The actual
/// creation, governance, and persistence of FailureInsights remains the
/// responsibility of the governed learning loop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureInsightCandidate {
    /// The kind of candidate.
    pub kind: FailureInsightCandidateKind,
    /// Human-readable description of what was observed.
    pub summary: String,
    /// Why this is considered a candidate.
    pub reason: String,
    /// The tool name that triggered this candidate.
    pub tool_name: String,
    /// Whether this is a positive signal (safety boundary working) vs a failure.
    pub is_positive_signal: bool,
}

impl FailureInsightCandidate {
    /// Convert an `ImprovementCandidate` from the General Cognitive Work Loop
    /// into a `FailureInsightCandidate` that the cognitive observation pipeline
    /// can process.
    ///
    /// This is the bridge between P2 (Cognitive Work Loop) and P3 (Cognitive
    /// Observation to Governed Learning).
    ///
    /// # Mapping
    ///
    /// | ImprovementCandidateKind | FailureInsightCandidateKind |
    /// |---|---|
    /// | MissingContext | MissingContext |
    /// | WeakPlan | AmbiguousResult |
    /// | RepeatedFriction | RepeatedOperatorFriction |
    /// | MissingTool | DocumentationMismatch |
    /// | MissingMemory | DocumentationMismatch |
    /// | PolicyGap | DocumentationMismatch |
    /// | ProcessImprovement | AmbiguousResult |
    /// | PromptImprovement | AmbiguousResult |
    /// | TestImprovement | DocumentationMismatch |
    ///
    /// # Safety
    ///
    /// - Pure domain conversion — no I/O, no execution, no authorization.
    /// - The returned candidate does not create, persist, or route any
    ///   FailureInsight record.
    /// - The candidate must still pass through the Decision Gate before any
    ///   governed action.
    pub fn from_improvement_candidate(
        candidate: &crate::cognitive_work::ImprovementCandidate,
    ) -> Self {
        let (kind, is_positive_signal) = match &candidate.kind {
            crate::cognitive_work::ImprovementCandidateKind::MissingContext => {
                (FailureInsightCandidateKind::MissingContext, false)
            }
            crate::cognitive_work::ImprovementCandidateKind::WeakPlan => {
                (FailureInsightCandidateKind::AmbiguousResult, false)
            }
            crate::cognitive_work::ImprovementCandidateKind::RepeatedFriction => {
                (FailureInsightCandidateKind::RepeatedOperatorFriction, false)
            }
            crate::cognitive_work::ImprovementCandidateKind::MissingTool
            | crate::cognitive_work::ImprovementCandidateKind::MissingMemory
            | crate::cognitive_work::ImprovementCandidateKind::PolicyGap
            | crate::cognitive_work::ImprovementCandidateKind::TestImprovement => {
                (FailureInsightCandidateKind::DocumentationMismatch, false)
            }
            crate::cognitive_work::ImprovementCandidateKind::ProcessImprovement
            | crate::cognitive_work::ImprovementCandidateKind::PromptImprovement => {
                (FailureInsightCandidateKind::AmbiguousResult, true)
            }
        };

        Self {
            kind,
            summary: candidate.description.clone(),
            reason: candidate.rationale.clone(),
            tool_name: "cognitive_work_loop".to_owned(),
            is_positive_signal,
        }
    }

    /// Convert a slice of `ImprovementCandidate`s into `FailureInsightCandidate`s.
    ///
    /// This is a convenience wrapper for bulk conversion.
    pub fn from_improvement_candidates(
        candidates: &[crate::cognitive_work::ImprovementCandidate],
    ) -> Vec<Self> {
        candidates
            .iter()
            .map(Self::from_improvement_candidate)
            .collect()
    }

    /// Convert a slice of [`ObservationAssessment`]s into `FailureInsightCandidate`s.
    ///
    /// This extracts the `.candidate` field from each assessment where present.
    /// Assessments without candidates (e.g. successful observations with no
    /// failure-insight signal) are skipped.
    ///
    /// # Safety
    ///
    /// - Pure domain conversion — no I/O, no execution, no authorization.
    /// - The returned candidates do not create, persist, or route any
    ///   FailureInsight record.
    pub fn from_assessments(assessments: &[ObservationAssessment]) -> Vec<Self> {
        assessments
            .iter()
            .filter_map(|a| a.candidate.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Conversion: ToolExecutionResult → CognitiveObservation
// ---------------------------------------------------------------------------

impl CognitiveObservation {
    /// Convert a [`ToolExecutionResult`] into a [`CognitiveObservation`].
    ///
    /// This is the primary entry point for the pipeline. It interprets the
    /// raw tool result and produces a structured cognitive observation with
    /// status, usefulness, risk, and candidate classification.
    ///
    /// # Explicit conversion
    ///
    /// This is not a `From` or `Into` trait implementation. The conversion
    /// must be called explicitly, preserving the audit trail and ensuring
    /// no automatic conversion path exists.
    ///
    /// # Safety
    ///
    /// - Pure domain — no I/O, no execution, no authorization.
    /// - The returned observation does not autorise any action.
    /// - A security block produces an observation with `status: Blocked`
    ///   and `candidate_kind: Some(SafetyBoundaryTriggered)` — a positive signal.
    pub fn from_tool_execution(result: &ToolExecutionResult) -> Self {
        let (
            source,
            kind,
            status,
            usefulness,
            risk,
            truncated,
            count,
            candidate_kind,
            candidate_reason,
        ) = classify_tool_result(result);

        let detail = match &result.error {
            Some(err) => format!("[{}] {}", err.code, err.message),
            None => result.output_summary.clone(),
        };

        Self {
            tool_name: result.tool_name.clone(),
            source,
            kind,
            status,
            usefulness,
            risk,
            summary: result.observation.summary.clone(),
            payload: result.observation.payload.clone(),
            truncated,
            count,
            detail,
            failure_insight_candidate: candidate_kind.is_some(),
            candidate_kind,
            candidate_reason,
        }
    }
}

// ---------------------------------------------------------------------------
// Assessment function
// ---------------------------------------------------------------------------

/// Evaluate a [`CognitiveObservation`] and produce an [`ObservationAssessment`].
///
/// This is a pure function that classifies observations into actionable
/// categories. It does not execute anything, write anything, or authorise
/// anything.
pub fn assess_observation(observation: &CognitiveObservation) -> ObservationAssessment {
    let is_useful = matches!(
        observation.usefulness,
        ObservationUsefulness::Medium
            | ObservationUsefulness::High
            | ObservationUsefulness::DirectlyActionable
    );

    let is_complete = matches!(
        observation.status,
        ObservationStatus::Success | ObservationStatus::Warning
    ) && !observation.truncated;

    let (assessment_summary, suggested_next_step) =
        match (&observation.status, &observation.usefulness, &observation.candidate_kind) {
            (ObservationStatus::Success, ObservationUsefulness::DirectlyActionable, _) => (
                format!(
                    "{} returned a directly actionable result ({} items). Ready for downstream processing.",
                    observation.tool_name, observation.count
                ),
                "Use this observation to inform the current cognitive cycle.".to_owned(),
            ),
            (ObservationStatus::Success, _, _) => (
                format!(
                    "{} returned {} results. Observation is useful but may need synthesis.",
                    observation.tool_name, observation.count
                ),
                "Combine with other observations for a complete picture.".to_owned(),
            ),
            (ObservationStatus::Empty, _, Some(FailureInsightCandidateKind::EmptySearchResult)) => (
                format!(
                    "{} returned zero results. Empty search result detected as FailureInsight candidate.",
                    observation.tool_name
                ),
                "Consider adjusting the query parameters or checking if the target exists.".to_owned(),
            ),
            (ObservationStatus::Empty, _, _) => (
                format!("{} returned zero results.", observation.tool_name),
                "Consider retrying with different parameters.".to_owned(),
            ),
            (ObservationStatus::Partial, _, _) => (
                format!(
                    "{} returned partial results ({} items). Data may be incomplete.",
                    observation.tool_name, observation.count
                ),
                "Retry with a narrower scope or check bounds.".to_owned(),
            ),
            (ObservationStatus::Truncated, _, Some(FailureInsightCandidateKind::TruncatedResult)) => (
                format!(
                    "{} result was truncated at {} items. Truncation detected as FailureInsight candidate.",
                    observation.tool_name, observation.count
                ),
                "Consider narrowing the query or increasing the limit.".to_owned(),
            ),
            (ObservationStatus::Truncated, _, _) => (
                format!(
                    "{} result was truncated at {} items.",
                    observation.tool_name, observation.count
                ),
                "The complete dataset was not observed.".to_owned(),
            ),
            (ObservationStatus::Blocked, _, Some(FailureInsightCandidateKind::SafetyBoundaryTriggered)) => (
                format!(
                    "{} was blocked by a security constraint. This is a positive signal: the safety boundary is functioning correctly.",
                    observation.tool_name
                ),
                "No action needed — this confirms expected security behaviour.".to_owned(),
            ),
            (ObservationStatus::Blocked, _, _) => (
                format!(
                    "{} was blocked by a security constraint.",
                    observation.tool_name
                ),
                "Review the access policy if this block was unexpected.".to_owned(),
            ),
            (ObservationStatus::Failed, _, Some(FailureInsightCandidateKind::ToolRuntimeFailure)) => (
                format!(
                    "{} failed with an error. Runtime failure detected as FailureInsight candidate.",
                    observation.tool_name
                ),
                "Investigate the error and retry with corrected parameters.".to_owned(),
            ),
            (ObservationStatus::Failed, _, _) => (
                format!(
                    "{} failed. Detail: {}",
                    observation.tool_name, observation.detail
                ),
                "Check the error and retry.".to_owned(),
            ),
            (ObservationStatus::Warning, _, _) => (
                format!(
                    "{} returned a warning ({} items). Results may be incomplete.",
                    observation.tool_name, observation.count
                ),
                "Verify that the warning condition is acceptable.".to_owned(),
            ),
        };

    let candidate = observation
        .candidate_kind
        .as_ref()
        .map(|kind| FailureInsightCandidate {
            kind: kind.clone(),
            summary: observation.summary.clone(),
            reason: observation.candidate_reason.clone(),
            tool_name: observation.tool_name.clone(),
            is_positive_signal: matches!(
                kind,
                FailureInsightCandidateKind::SafetyBoundaryTriggered
            ),
        });

    ObservationAssessment {
        observation: observation.clone(),
        is_useful,
        is_complete,
        failure_insight_candidate: observation.failure_insight_candidate,
        candidate,
        assessment_summary,
        suggested_next_step,
    }
}

// ---------------------------------------------------------------------------
// Internal classification helper
// ---------------------------------------------------------------------------

/// Classify a [`ToolExecutionResult`] into cognitive categories.
#[allow(clippy::type_complexity)]
fn classify_tool_result(
    result: &ToolExecutionResult,
) -> (
    ObservationSource,
    ObservationKind,
    ObservationStatus,
    ObservationUsefulness,
    ObservationRisk,
    bool,  // truncated
    usize, // count
    Option<FailureInsightCandidateKind>,
    String, // candidate reason
) {
    let tool_name = &result.tool_name;
    let source = ObservationSource::ToolExecution;

    let (kind, _status, use_truncated, count) = match tool_name.as_str() {
        "read_file" => classify_read_file(result),
        "list_files" => classify_list_files(result),
        "search_text" => classify_search_text(result),
        _ => (
            ObservationKind::Informational,
            ObservationStatus::Success,
            false,
            0,
        ),
    };

    let truncated = use_truncated
        || result
            .observation
            .payload
            .get("truncated")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    // Derive status from execution status + truncated flag
    let status = if truncated
        && matches!(
            result.status,
            ToolExecutionStatus::Success | ToolExecutionStatus::Warning
        ) {
        // A structurally successful execution whose result was truncated
        ObservationStatus::Truncated
    } else if result.status == ToolExecutionStatus::Success
        && result
            .observation
            .payload
            .get("total")
            .and_then(Value::as_u64)
            == Some(0)
    {
        ObservationStatus::Empty
    } else {
        match result.status {
            ToolExecutionStatus::Success => ObservationStatus::Success,
            ToolExecutionStatus::Warning => ObservationStatus::Warning,
            ToolExecutionStatus::Failed => ObservationStatus::Failed,
            ToolExecutionStatus::Blocked => ObservationStatus::Blocked,
            ToolExecutionStatus::Skipped => ObservationStatus::Empty,
        }
    };

    // Determine usefulness
    let usefulness = match &status {
        ObservationStatus::Success if count > 0 && !truncated => {
            if matches!(tool_name.as_str(), "read_file") {
                // File content with at least some lines is directly actionable
                ObservationUsefulness::DirectlyActionable
            } else {
                ObservationUsefulness::High
            }
        }
        ObservationStatus::Success if count > 0 && truncated => ObservationUsefulness::Medium,
        ObservationStatus::Success => ObservationUsefulness::Low,
        ObservationStatus::Warning => ObservationUsefulness::Low,
        ObservationStatus::Blocked => ObservationUsefulness::Low,
        ObservationStatus::Failed => ObservationUsefulness::None,
        ObservationStatus::Empty | ObservationStatus::Partial => ObservationUsefulness::Low,
        ObservationStatus::Truncated => ObservationUsefulness::Medium,
    };

    // Determine risk
    let risk = match &status {
        ObservationStatus::Blocked => ObservationRisk::Security,
        ObservationStatus::Failed => {
            if let Some(err) = &result.error {
                if err.is_security {
                    ObservationRisk::Security
                } else {
                    ObservationRisk::Low
                }
            } else {
                ObservationRisk::Low
            }
        }
        ObservationStatus::Warning => ObservationRisk::Low,
        _ => ObservationRisk::None,
    };

    // Determine FailureInsightCandidate
    let (candidate_kind, candidate_reason) = match &status {
        ObservationStatus::Empty => {
            // Empty result is a candidate for learning
            (
                Some(FailureInsightCandidateKind::EmptySearchResult),
                format!(
                    "{} returned zero results. Consider whether the query, path, or expectation was incorrect.",
                    tool_name
                ),
            )
        }
        ObservationStatus::Truncated => {
            // Truncated result is a candidate for learning
            (
                Some(FailureInsightCandidateKind::TruncatedResult),
                format!(
                    "{} result was truncated at {} items. Full data was not observed.",
                    tool_name, count
                ),
            )
        }
        ObservationStatus::Blocked => {
            // Security block is a positive signal
            (
                Some(FailureInsightCandidateKind::SafetyBoundaryTriggered),
                format!(
                    "{} was blocked by a security constraint. The safety boundary triggered correctly.",
                    tool_name
                ),
            )
        }
        ObservationStatus::Failed => {
            // Runtime failure is a candidate
            (
                Some(FailureInsightCandidateKind::ToolRuntimeFailure),
                format!(
                    "{} failed. Error: {}",
                    tool_name,
                    result
                        .error
                        .as_ref()
                        .map(|e| e.message.as_str())
                        .unwrap_or("unknown")
                ),
            )
        }
        _ => (None, String::new()),
    };

    (
        source,
        kind,
        status,
        usefulness,
        risk,
        truncated,
        count,
        candidate_kind,
        candidate_reason,
    )
}

/// Classify a read_file result.
fn classify_read_file(
    result: &ToolExecutionResult,
) -> (ObservationKind, ObservationStatus, bool, usize) {
    let kind = ObservationKind::FileContent;
    let status = ObservationStatus::Success;
    let truncated = result
        .observation
        .payload
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let count = result
        .observation
        .payload
        .get("lines")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    (kind, status, truncated, count)
}

/// Classify a list_files result.
fn classify_list_files(
    result: &ToolExecutionResult,
) -> (ObservationKind, ObservationStatus, bool, usize) {
    let kind = ObservationKind::DirectoryListing;
    let status = match result.status {
        ToolExecutionStatus::Success | ToolExecutionStatus::Warning => ObservationStatus::Success,
        _ => ObservationStatus::Failed,
    };
    let truncated = result
        .observation
        .payload
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let count = result
        .observation
        .payload
        .get("total")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    (kind, status, truncated, count)
}

/// Classify a search_text result.
fn classify_search_text(
    result: &ToolExecutionResult,
) -> (ObservationKind, ObservationStatus, bool, usize) {
    let kind = ObservationKind::SearchResult;
    let status = ObservationStatus::Success;
    let truncated = result
        .observation
        .payload
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let count = result
        .observation
        .payload
        .get("total")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    (kind, status, truncated, count)
}

// ---------------------------------------------------------------------------
// Convenience helpers
// ---------------------------------------------------------------------------

impl CognitiveObservation {
    /// Returns `true` if this observation should be fed to the FailureInsight system.
    pub fn should_create_failure_insight(&self) -> bool {
        self.failure_insight_candidate
    }

    /// Returns `true` if this observation is a positive security signal rather than a failure.
    pub fn is_positive_signal(&self) -> bool {
        matches!(
            self.candidate_kind,
            Some(FailureInsightCandidateKind::SafetyBoundaryTriggered)
        )
    }
}

impl ObservationAssessment {
    /// Returns `true` if the assessment produced a FailureInsight candidate.
    pub fn has_candidate(&self) -> bool {
        self.candidate.is_some()
    }

    /// Returns the candidate summary, if any.
    pub fn candidate_summary(&self) -> Option<&str> {
        self.candidate.as_ref().map(|c| c.summary.as_str())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ToolExecutionId;
    use crate::tool::{ToolExecutionError, ToolExecutionResult, ToolObservation};
    use serde_json::json;

    // -----------------------------------------------------------------------
    // Serialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn cognitive_observation_serializes_and_deserializes() {
        let obs = CognitiveObservation {
            tool_name: "read_file".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::FileContent,
            status: ObservationStatus::Success,
            usefulness: ObservationUsefulness::DirectlyActionable,
            risk: ObservationRisk::None,
            summary: "Read file Cargo.toml (29 lines)".to_owned(),
            payload: json!({"lines": 29}),
            truncated: false,
            count: 29,
            detail: "29 lines read".to_owned(),
            failure_insight_candidate: false,
            candidate_kind: None,
            candidate_reason: String::new(),
        };

        let encoded = serde_json::to_value(&obs).expect("should serialize");
        assert_eq!(encoded["tool_name"], "read_file");
        assert_eq!(encoded["status"], "success");
        assert_eq!(encoded["usefulness"], "directly_actionable");

        let decoded: CognitiveObservation =
            serde_json::from_value(encoded).expect("should deserialize");
        assert_eq!(decoded.tool_name, "read_file");
        assert_eq!(decoded.status, ObservationStatus::Success);
        assert_eq!(decoded.count, 29);
    }

    #[test]
    fn observation_assessment_serializes() {
        let obs = CognitiveObservation {
            tool_name: "search_text".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::SearchResult,
            status: ObservationStatus::Truncated,
            usefulness: ObservationUsefulness::Medium,
            risk: ObservationRisk::None,
            summary: "Search returned 100 results (truncated)".to_owned(),
            payload: json!({"total": 100, "truncated": true}),
            truncated: true,
            count: 100,
            detail: "Search truncated at 100 results".to_owned(),
            failure_insight_candidate: true,
            candidate_kind: Some(FailureInsightCandidateKind::TruncatedResult),
            candidate_reason: "Result was truncated at 100 items".to_owned(),
        };

        let assessment = assess_observation(&obs);

        let encoded = serde_json::to_value(&assessment).expect("should serialize");
        assert!(encoded["is_useful"].as_bool().unwrap());
        assert!(encoded["failure_insight_candidate"].as_bool().unwrap());
        assert!(encoded["candidate"].is_object());

        let decoded: ObservationAssessment =
            serde_json::from_value(encoded).expect("should deserialize");
        assert!(decoded.is_useful);
        assert!(decoded.has_candidate());
        assert_eq!(
            decoded.candidate.unwrap().kind,
            FailureInsightCandidateKind::TruncatedResult
        );
    }

    #[test]
    fn failure_insight_candidate_kind_serializes() {
        let variants = vec![
            FailureInsightCandidateKind::BlockedToolUse,
            FailureInsightCandidateKind::MissingContext,
            FailureInsightCandidateKind::EmptySearchResult,
            FailureInsightCandidateKind::TruncatedResult,
            FailureInsightCandidateKind::AmbiguousResult,
            FailureInsightCandidateKind::SafetyBoundaryTriggered,
            FailureInsightCandidateKind::ToolRuntimeFailure,
            FailureInsightCandidateKind::DocumentationMismatch,
            FailureInsightCandidateKind::RepeatedOperatorFriction,
            FailureInsightCandidateKind::InsufficientObservationQuality,
            FailureInsightCandidateKind::ContextAssemblyWeak,
        ];

        for variant in &variants {
            let encoded = serde_json::to_value(variant).expect("should serialize");
            let decoded: FailureInsightCandidateKind =
                serde_json::from_value(encoded).expect("should deserialize");
            assert_eq!(&decoded, variant);
        }
    }

    // -----------------------------------------------------------------------
    // Conversion tests — ToolExecutionResult → CognitiveObservation
    // -----------------------------------------------------------------------

    #[test]
    fn from_tool_execution_success_maps_correctly() {
        let result = ToolExecutionResult::success(
            ToolExecutionId::new("exec-1"),
            "read_file",
            ToolObservation {
                summary: "Read file Cargo.toml (29 lines, 872 chars)".to_owned(),
                payload: json!({
                    "path": "Cargo.toml",
                    "lines": 29,
                    "characters": 872,
                    "content_preview": "[package]\nname = \"arpagona\"\n",
                    "truncated": false,
                }),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            "Read file Cargo.toml (29 lines)",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);

        assert_eq!(obs.tool_name, "read_file");
        assert_eq!(obs.source, ObservationSource::ToolExecution);
        assert_eq!(obs.kind, ObservationKind::FileContent);
        assert_eq!(obs.status, ObservationStatus::Success);
        assert_eq!(obs.usefulness, ObservationUsefulness::DirectlyActionable);
        assert_eq!(obs.risk, ObservationRisk::None);
        assert!(!obs.truncated);
        assert_eq!(obs.count, 29);
        assert!(!obs.failure_insight_candidate);
        assert!(obs.candidate_kind.is_none());
    }

    #[test]
    fn from_tool_execution_blocked_maps_to_positive_signal() {
        let result = ToolExecutionResult::blocked(
            ToolExecutionId::new("exec-2"),
            "read_file",
            "Path outside workspace",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);

        assert_eq!(obs.tool_name, "read_file");
        assert_eq!(obs.status, ObservationStatus::Blocked);
        assert_eq!(obs.usefulness, ObservationUsefulness::Low);
        assert_eq!(obs.risk, ObservationRisk::Security);
        assert!(obs.failure_insight_candidate);
        assert_eq!(
            obs.candidate_kind,
            Some(FailureInsightCandidateKind::SafetyBoundaryTriggered)
        );
        assert!(obs.is_positive_signal());
    }

    #[test]
    fn from_tool_execution_failed_maps_to_runtime_failure_candidate() {
        let result = ToolExecutionResult::failed(
            ToolExecutionId::new("exec-3"),
            "read_file",
            ToolExecutionError::new("io_error", "Cannot read file: permission denied"),
            "I/O error reading secrets.txt",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);

        assert_eq!(obs.status, ObservationStatus::Failed);
        assert_eq!(obs.usefulness, ObservationUsefulness::None);
        assert!(obs.failure_insight_candidate);
        assert_eq!(
            obs.candidate_kind,
            Some(FailureInsightCandidateKind::ToolRuntimeFailure)
        );
        assert!(!obs.is_positive_signal());
    }

    #[test]
    fn from_tool_execution_truncated_maps_to_truncated_candidate() {
        let result = ToolExecutionResult::warning(
            ToolExecutionId::new("exec-4"),
            "search_text",
            ToolObservation {
                summary: "Search returned 100 results (truncated at 100)".to_owned(),
                payload: json!({
                    "query": "fn main",
                    "total": 100,
                    "truncated": true,
                }),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            "Search for 'fn main': 100 results (truncated)",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);

        // Warning status with truncated payload
        assert_eq!(obs.status, ObservationStatus::Truncated);
        assert!(obs.truncated);
        assert_eq!(obs.count, 100);
        assert!(obs.failure_insight_candidate);
        assert_eq!(
            obs.candidate_kind,
            Some(FailureInsightCandidateKind::TruncatedResult)
        );
    }

    #[test]
    fn from_tool_execution_empty_search_maps_to_empty_candidate() {
        let result = ToolExecutionResult::success(
            ToolExecutionId::new("exec-5"),
            "search_text",
            ToolObservation {
                summary: "Search returned 0 results".to_owned(),
                payload: json!({
                    "query": "nonexistent_function_xyz",
                    "total": 0,
                    "truncated": false,
                }),
                actionable: false,
                failure_insight_candidate: true,
                failure_hint: Some("empty_result".to_string()),
            },
            "Search for 'nonexistent_function_xyz': 0 results",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);

        assert_eq!(obs.status, ObservationStatus::Empty);
        assert_eq!(obs.usefulness, ObservationUsefulness::Low);
        assert_eq!(obs.count, 0);
        assert!(obs.failure_insight_candidate);
        assert_eq!(
            obs.candidate_kind,
            Some(FailureInsightCandidateKind::EmptySearchResult)
        );
        assert!(obs.should_create_failure_insight());
    }

    // -----------------------------------------------------------------------
    // Assessment tests
    // -----------------------------------------------------------------------

    #[test]
    fn assess_successful_read_file_is_useful_and_complete() {
        let obs = CognitiveObservation {
            tool_name: "read_file".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::FileContent,
            status: ObservationStatus::Success,
            usefulness: ObservationUsefulness::DirectlyActionable,
            risk: ObservationRisk::None,
            summary: "Read file Cargo.toml".to_owned(),
            payload: json!({"lines": 29}),
            truncated: false,
            count: 29,
            detail: "29 lines".to_owned(),
            failure_insight_candidate: false,
            candidate_kind: None,
            candidate_reason: String::new(),
        };

        let assessment = assess_observation(&obs);

        assert!(assessment.is_useful);
        assert!(assessment.is_complete);
        assert!(!assessment.has_candidate());
        assert!(assessment
            .assessment_summary
            .contains("directly actionable"));
    }

    #[test]
    fn assess_truncated_search_is_useful_but_incomplete() {
        let obs = CognitiveObservation {
            tool_name: "search_text".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::SearchResult,
            status: ObservationStatus::Truncated,
            usefulness: ObservationUsefulness::Medium,
            risk: ObservationRisk::None,
            summary: "Search returned 100 results (truncated)".to_owned(),
            payload: json!({"total": 100, "truncated": true}),
            truncated: true,
            count: 100,
            detail: "Truncated at 100".to_owned(),
            failure_insight_candidate: true,
            candidate_kind: Some(FailureInsightCandidateKind::TruncatedResult),
            candidate_reason: "Result truncated at 100 items".to_owned(),
        };

        let assessment = assess_observation(&obs);

        assert!(assessment.is_useful);
        assert!(!assessment.is_complete);
        assert!(assessment.has_candidate());
        assert_eq!(
            assessment.candidate.as_ref().unwrap().kind,
            FailureInsightCandidateKind::TruncatedResult
        );
    }

    #[test]
    fn assess_safety_boundary_is_positive_signal() {
        let obs = CognitiveObservation {
            tool_name: "read_file".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::SecurityBoundary,
            status: ObservationStatus::Blocked,
            usefulness: ObservationUsefulness::Low,
            risk: ObservationRisk::Security,
            summary: "File access blocked: /etc/passwd".to_owned(),
            payload: json!({}),
            truncated: false,
            count: 0,
            detail: "Path outside workspace: /etc/passwd".to_owned(),
            failure_insight_candidate: true,
            candidate_kind: Some(FailureInsightCandidateKind::SafetyBoundaryTriggered),
            candidate_reason: "Blocked by security constraint".to_owned(),
        };

        let assessment = assess_observation(&obs);

        assert!(assessment.observation.is_positive_signal());
        assert!(assessment.has_candidate());
        assert!(assessment.candidate.as_ref().unwrap().is_positive_signal);
        assert!(assessment.assessment_summary.contains("positive signal"));
    }

    #[test]
    fn assess_failed_observation_is_not_useful() {
        let obs = CognitiveObservation {
            tool_name: "read_file".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::Error,
            status: ObservationStatus::Failed,
            usefulness: ObservationUsefulness::None,
            risk: ObservationRisk::Low,
            summary: "Cannot read file: permission denied".to_owned(),
            payload: json!({}),
            truncated: false,
            count: 0,
            detail: "[io_error] Cannot read file: permission denied".to_owned(),
            failure_insight_candidate: true,
            candidate_kind: Some(FailureInsightCandidateKind::ToolRuntimeFailure),
            candidate_reason: "Tool runtime failure".to_owned(),
        };

        let assessment = assess_observation(&obs);

        assert!(!assessment.is_useful);
        assert!(!assessment.is_complete);
        assert!(assessment.has_candidate());
        assert_eq!(
            assessment.candidate.unwrap().kind,
            FailureInsightCandidateKind::ToolRuntimeFailure
        );
    }

    // -----------------------------------------------------------------------
    // Total pipeline test
    // -----------------------------------------------------------------------

    #[test]
    fn full_pipeline_success_to_assessment() {
        let result = ToolExecutionResult::success(
            ToolExecutionId::new("exec-full"),
            "read_file",
            ToolObservation {
                summary: "Read file Cargo.toml (29 lines)".to_owned(),
                payload: json!({
                    "path": "Cargo.toml",
                    "lines": 29,
                    "characters": 872,
                    "content_preview": "[package]\nname = \"arpagona\"\n",
                    "truncated": false,
                }),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            "Read file Cargo.toml (29 lines)",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);
        let assessment = assess_observation(&obs);

        // Pipeline produces a useful, complete, non-candidate assessment
        assert_eq!(obs.tool_name, "read_file");
        assert_eq!(obs.status, ObservationStatus::Success);
        assert!(assessment.is_useful);
        assert!(assessment.is_complete);
        assert!(!assessment.has_candidate());
    }

    #[test]
    fn full_pipeline_truncated_to_candidate_assessment() {
        let result = ToolExecutionResult::warning(
            ToolExecutionId::new("exec-full-truncated"),
            "search_text",
            ToolObservation {
                summary: "Search returned 100 results (truncated at 100)".to_owned(),
                payload: json!({
                    "query": "fn",
                    "total": 100,
                    "truncated": true,
                }),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            "Search for 'fn': 100 results (truncated)",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);
        let assessment = assess_observation(&obs);

        assert_eq!(obs.status, ObservationStatus::Truncated);
        assert!(obs.failure_insight_candidate);
        assert!(assessment.is_useful);
        assert!(!assessment.is_complete);
        assert!(assessment.has_candidate());
        assert_eq!(
            assessment.candidate.unwrap().kind,
            FailureInsightCandidateKind::TruncatedResult
        );
    }

    #[test]
    fn full_pipeline_blocked_to_positive_signal() {
        let result = ToolExecutionResult::blocked(
            ToolExecutionId::new("exec-full-blocked"),
            "read_file",
            "Path outside workspace",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);
        let assessment = assess_observation(&obs);

        assert_eq!(obs.status, ObservationStatus::Blocked);
        assert!(obs.failure_insight_candidate);
        assert!(obs.is_positive_signal());
        assert!(assessment.has_candidate());
        assert!(assessment.candidate.as_ref().unwrap().is_positive_signal);
        assert!(assessment.assessment_summary.contains("positive signal"));
    }

    #[test]
    fn full_pipeline_empty_search_to_candidate() {
        let result = ToolExecutionResult::success(
            ToolExecutionId::new("exec-full-empty"),
            "search_text",
            ToolObservation {
                summary: "Search returned 0 results".to_owned(),
                payload: json!({
                    "query": "zzz_nonexistent",
                    "total": 0,
                    "truncated": false,
                }),
                actionable: false,
                failure_insight_candidate: true,
                failure_hint: Some("empty_result".to_string()),
            },
            "Search for 'zzz_nonexistent': 0 results",
        );

        let obs = CognitiveObservation::from_tool_execution(&result);
        let assessment = assess_observation(&obs);

        assert_eq!(obs.status, ObservationStatus::Empty);
        assert!(obs.failure_insight_candidate);
        assert!(!assessment.is_useful);
        assert!(!assessment.is_complete);
        assert!(assessment.has_candidate());
        assert_eq!(
            assessment.candidate.unwrap().kind,
            FailureInsightCandidateKind::EmptySearchResult
        );
    }

    // -----------------------------------------------------------------------
    // Domain purity tests
    // -----------------------------------------------------------------------

    #[test]
    fn types_do_not_contain_authorization_fields() {
        let obs = CognitiveObservation {
            tool_name: "test".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::Informational,
            status: ObservationStatus::Success,
            usefulness: ObservationUsefulness::Low,
            risk: ObservationRisk::None,
            summary: "test".to_owned(),
            payload: json!({}),
            truncated: false,
            count: 0,
            detail: "test".to_owned(),
            failure_insight_candidate: false,
            candidate_kind: None,
            candidate_reason: String::new(),
        };

        let encoded = serde_json::to_value(&obs).expect("should serialize");
        let text = encoded.to_string();
        assert!(!text.contains("approved"));
        assert!(!text.contains("authorized"));
        assert!(!text.contains("execute"));
        assert!(!text.contains("persist"));
        assert!(!text.contains("write"));

        let assessment = ObservationAssessment {
            observation: obs,
            is_useful: false,
            is_complete: false,
            failure_insight_candidate: false,
            candidate: None,
            assessment_summary: "test".to_owned(),
            suggested_next_step: "test".to_owned(),
        };

        let encoded = serde_json::to_value(&assessment).expect("should serialize");
        let text = encoded.to_string();
        assert!(!text.contains("approved"));
        assert!(!text.contains("authorized"));
    }

    #[test]
    fn observation_assessment_does_not_create_failure_insight() {
        // This test proves that the assessment function only *flags* candidates
        // — it does not create FailureInsight records.
        let obs = CognitiveObservation {
            tool_name: "search_text".to_owned(),
            source: ObservationSource::ToolExecution,
            kind: ObservationKind::SearchResult,
            status: ObservationStatus::Empty,
            usefulness: ObservationUsefulness::Low,
            risk: ObservationRisk::None,
            summary: "Search returned 0 results".to_owned(),
            payload: json!({"total": 0}),
            truncated: false,
            count: 0,
            detail: "Empty result".to_owned(),
            failure_insight_candidate: true,
            candidate_kind: Some(FailureInsightCandidateKind::EmptySearchResult),
            candidate_reason: "Empty result".to_owned(),
        };

        let assessment = assess_observation(&obs);

        assert!(assessment.has_candidate());
        // Verify the candidate is NOT a full FailureInsight
        // (it lacks id, severity, FailureClass, etc.)
        assert_eq!(
            assessment.candidate.as_ref().unwrap().kind,
            FailureInsightCandidateKind::EmptySearchResult
        );
        // The candidate is a lightweight marker, not a governed FailureInsight
    }
}
