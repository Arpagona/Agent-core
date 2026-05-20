use crate::ids::{
    AuditEventId, DecisionId, FailureInsightId, ProposedActionId, TaskId, WorkspaceId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Pure domain vocabulary for turning observed failures into durable learning.
///
/// A `FailureInsight` is descriptive and non-authorizing: it may guide future
/// code, tests, policy, memory, documentation or routing work, but it never
/// approves actions, executes tools, mutates persistence or changes governance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureInsight {
    pub id: FailureInsightId,
    pub failure_class: FailureClass,
    pub severity: InsightSeverity,
    pub status: InsightStatus,
    pub correction_target: CorrectionTarget,
    pub summary: String,
    pub root_cause: String,
    pub impact: String,
    pub corrective_action: String,
    pub owner_layer: String,
    pub detection_signal: DetectionSignal,
    pub confidence: f32,
    pub workspace_id: Option<WorkspaceId>,
    pub task_id: Option<TaskId>,
    pub proposed_action_id: Option<ProposedActionId>,
    pub decision_id: Option<DecisionId>,
    pub audit_event_id: Option<AuditEventId>,
    pub linked_pr: Option<String>,
    pub linked_test: Option<String>,
    pub linked_doc: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    MissingContext,
    StaleContext,
    BadActionType,
    PolicyGap,
    BlockedWithoutExplanation,
    WrongComputeChoice,
    ToolMismatch,
    UnsafeDrift,
    InsufficientObservability,
    TestGap,
    DocumentationGap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionTarget {
    Code,
    Test,
    Policy,
    Memory,
    Docs,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightStatus {
    Proposed,
    Accepted,
    Applied,
    Superseded,
    Rejected,
    NoChange,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightSeverity {
    Informational,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionSignalType {
    HumanCorrection,
    AuditEvent,
    TestFailure,
    ReviewFinding,
    RuntimeObservation,
    PolicyReview,
    DocumentationReview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionSignal {
    pub signal_type: DetectionSignalType,
    pub description: String,
}

impl DetectionSignal {
    pub fn new(signal_type: DetectionSignalType, description: impl Into<String>) -> Self {
        Self {
            signal_type,
            description: description.into(),
        }
    }
}

impl FailureInsight {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: FailureInsightId,
        failure_class: FailureClass,
        severity: InsightSeverity,
        correction_target: CorrectionTarget,
        summary: impl Into<String>,
        root_cause: impl Into<String>,
        impact: impl Into<String>,
        corrective_action: impl Into<String>,
        owner_layer: impl Into<String>,
        detection_signal: DetectionSignal,
        confidence: f32,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            failure_class,
            severity,
            status: InsightStatus::Proposed,
            correction_target,
            summary: summary.into(),
            root_cause: root_cause.into(),
            impact: impact.into(),
            corrective_action: corrective_action.into(),
            owner_layer: owner_layer.into(),
            detection_signal,
            confidence,
            workspace_id: None,
            task_id: None,
            proposed_action_id: None,
            decision_id: None,
            audit_event_id: None,
            linked_pr: None,
            linked_test: None,
            linked_doc: None,
            created_at,
        }
    }

    pub fn with_trace_links(
        mut self,
        workspace_id: Option<WorkspaceId>,
        task_id: Option<TaskId>,
        proposed_action_id: Option<ProposedActionId>,
        decision_id: Option<DecisionId>,
        audit_event_id: Option<AuditEventId>,
    ) -> Self {
        self.workspace_id = workspace_id;
        self.task_id = task_id;
        self.proposed_action_id = proposed_action_id;
        self.decision_id = decision_id;
        self.audit_event_id = audit_event_id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_insight_serializes_with_trace_links() {
        let created_at = "2026-05-20T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let insight = FailureInsight::new(
            FailureInsightId::new("insight-1"),
            FailureClass::MissingContext,
            InsightSeverity::Medium,
            CorrectionTarget::Memory,
            "Agent lacked the required context.",
            "Recall did not include the relevant policy note.",
            "The proposed action was too generic.",
            "Add a durable memory convention before the next loop.",
            "Graph Memory / Recall",
            DetectionSignal::new(
                DetectionSignalType::HumanCorrection,
                "Human reviewer identified missing context during PR review.",
            ),
            0.91,
            created_at,
        )
        .with_trace_links(
            Some(WorkspaceId::new("workspace-1")),
            Some(TaskId::new("task-1")),
            Some(ProposedActionId::new("action-1")),
            Some(DecisionId::new("decision-1")),
            Some(AuditEventId::new("audit-1")),
        );

        let encoded = serde_json::to_string(&insight).expect("insight should serialize");
        assert!(encoded.contains("missing_context"));
        assert!(encoded.contains("human_correction"));

        let decoded: FailureInsight =
            serde_json::from_str(&encoded).expect("insight should deserialize");

        assert_eq!(decoded.id, FailureInsightId::new("insight-1"));
        assert_eq!(decoded.failure_class, FailureClass::MissingContext);
        assert_eq!(decoded.status, InsightStatus::Proposed);
        assert_eq!(decoded.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(decoded.audit_event_id, Some(AuditEventId::new("audit-1")));
        assert_eq!(
            decoded.detection_signal.signal_type,
            DetectionSignalType::HumanCorrection
        );
    }

    #[test]
    fn failure_insight_can_record_no_change_without_authorizing_action() {
        let mut insight = FailureInsight::new(
            FailureInsightId::new("insight-no-change"),
            FailureClass::DocumentationGap,
            InsightSeverity::Low,
            CorrectionTarget::None,
            "No code correction required.",
            "The current behavior was correct but needed explicit reporting.",
            "Low; reporting clarity only.",
            "Record deliberate no_change so the loop is auditable.",
            "Docs / Doctrine",
            DetectionSignal::new(
                DetectionSignalType::ReviewFinding,
                "Reviewer asked why no durable artifact changed.",
            ),
            0.8,
            Utc::now(),
        );

        insight.status = InsightStatus::NoChange;

        assert_eq!(insight.status, InsightStatus::NoChange);
        assert_eq!(insight.correction_target, CorrectionTarget::None);
    }
}
