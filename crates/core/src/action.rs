use crate::graph::GraphRef;
use crate::ids::{
    AgentId, AuditEventId, DecisionId, FactId, FailureInsightId, ProposedActionId, SourceId,
    TaskId, WorkspaceId,
};
use crate::permission::Permission;
use crate::risk::RiskLevel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    ReadMemory,
    ReadTasks,
    ReadProposedActions,
    ReadPendingActions,
    ReadDecisions,
    ReadAudit,
    ReadStatus,
    SystemCheck,
    /// Legacy coarse memory-write action type. Prefer the more specific
    /// memory-write proposal variants below for new governed memory work.
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
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedActionStatus {
    PendingDecision,
    Approved,
    Blocked,
    NeedsHumanApproval,
    Cancelled,
    Rejected,
    Deferred,
    Superseded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryWriteKind {
    CreateMemoryFact,
    LinkMemoryFact,
    InvalidateMemoryFact,
    CreateFailureInsightMemory,
}

impl MemoryWriteKind {
    pub fn action_type(&self) -> ActionType {
        match self {
            Self::CreateMemoryFact => ActionType::CreateMemoryFact,
            Self::LinkMemoryFact => ActionType::LinkMemoryFact,
            Self::InvalidateMemoryFact => ActionType::InvalidateMemoryFact,
            Self::CreateFailureInsightMemory => ActionType::CreateFailureInsightMemory,
        }
    }
}

/// Structured intent for proposed memory writes.
///
/// This is proposal vocabulary only. Creating this value does not mutate Graph
/// Memory, approve the write, or bypass the Decision Gate. A memory write may
/// only become a controlled effect after the normal
/// `ProposedAction -> DecisionGate -> Decision -> Audit` path approves it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryWriteIntent {
    pub kind: MemoryWriteKind,
    pub target: MemoryWriteTarget,
    pub provenance: MemoryWriteProvenance,
    pub confidence: f32,
    pub actor: AgentId,
    pub reason_for_remembering: String,
    pub proposed_at: DateTime<Utc>,
    pub decision_id: Option<DecisionId>,
    pub audit_event_id: Option<AuditEventId>,
    pub invalidation_note: Option<String>,
}

impl MemoryWriteIntent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: MemoryWriteKind,
        target: MemoryWriteTarget,
        provenance: MemoryWriteProvenance,
        confidence: f32,
        actor: AgentId,
        reason_for_remembering: impl Into<String>,
        proposed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            kind,
            target,
            provenance,
            confidence,
            actor,
            reason_for_remembering: reason_for_remembering.into(),
            proposed_at,
            decision_id: None,
            audit_event_id: None,
            invalidation_note: None,
        }
    }

    pub fn with_audit_linkage(
        mut self,
        decision_id: Option<DecisionId>,
        audit_event_id: Option<AuditEventId>,
    ) -> Self {
        self.decision_id = decision_id;
        self.audit_event_id = audit_event_id;
        self
    }

    pub fn with_invalidation_note(mut self, invalidation_note: impl Into<String>) -> Self {
        self.invalidation_note = Some(invalidation_note.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryWriteTarget {
    pub entity_type: String,
    pub entity_id: String,
    pub attribute: Option<String>,
    /// Optional value proposed for fact creation.
    ///
    /// This makes the proposed memory write inspectable before persistence: a
    /// supervisor can see not only which entity/attribute would be touched, but
    /// the concrete value that would become a Graph Memory fact. Older proposal
    /// payloads may omit this while the alpha readback path remains compatible.
    #[serde(default)]
    pub value: Option<Value>,
    pub fact_id: Option<FactId>,
    pub related_fact_id: Option<FactId>,
    pub failure_insight_id: Option<FailureInsightId>,
}

impl MemoryWriteTarget {
    pub fn fact(
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        attribute: impl Into<String>,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            attribute: Some(attribute.into()),
            value: None,
            fact_id: None,
            related_fact_id: None,
            failure_insight_id: None,
        }
    }

    pub fn fact_with_value(
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            attribute: Some(attribute.into()),
            value: Some(value),
            fact_id: None,
            related_fact_id: None,
            failure_insight_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryWriteProvenance {
    pub source_id: Option<SourceId>,
    pub source_label: String,
    pub source_kind: String,
    pub evidence: String,
}

impl MemoryWriteProvenance {
    pub fn new(
        source_id: Option<SourceId>,
        source_label: impl Into<String>,
        source_kind: impl Into<String>,
        evidence: impl Into<String>,
    ) -> Self {
        Self {
            source_id,
            source_label: source_label.into(),
            source_kind: source_kind.into(),
            evidence: evidence.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposedAction {
    pub id: ProposedActionId,
    pub workspace_id: WorkspaceId,
    pub task_id: Option<TaskId>,
    pub proposed_by: AgentId,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub payload: Value,
    pub risk_level: RiskLevel,
    pub required_permissions: Vec<Permission>,
    pub rationale: String,
    pub context_refs: Vec<GraphRef>,
    pub status: ProposedActionStatus,
    pub created_at: DateTime<Utc>,
}

/// Status of a dry-run execution attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DryRunStatus {
    DryRunCompleted,
    DryRunBlocked,
}

/// Result of a dry-run execution simulation for an approved proposal.
///
/// Dry-run is non-destructive: it describes what *would* happen without
/// actually executing tools, modifying files, or calling external systems.
/// Every dry-run attempt creates an audit event.
///
/// The [`execution_capability`] field provides deterministic capability
/// metadata from the execution capability registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DryRunResult {
    pub proposal_id: ProposedActionId,
    pub action_type: ActionType,
    pub expected_effects: Vec<String>,
    pub required_permissions: Vec<Permission>,
    pub touched_resources: Vec<String>,
    pub risk_level: RiskLevel,
    pub reversibility: String,
    pub human_readable_summary: String,
    pub status: DryRunStatus,
    pub execution_capability: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl std::str::FromStr for ActionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read_memory" => Ok(ActionType::ReadMemory),
            "read_tasks" => Ok(ActionType::ReadTasks),
            "read_proposed_actions" => Ok(ActionType::ReadProposedActions),
            "read_pending_actions" => Ok(ActionType::ReadPendingActions),
            "read_decisions" => Ok(ActionType::ReadDecisions),
            "read_audit" => Ok(ActionType::ReadAudit),
            "read_status" => Ok(ActionType::ReadStatus),
            "system_check" => Ok(ActionType::SystemCheck),
            "write_memory" => Ok(ActionType::WriteMemory),
            "create_memory_fact" => Ok(ActionType::CreateMemoryFact),
            "link_memory_fact" => Ok(ActionType::LinkMemoryFact),
            "invalidate_memory_fact" => Ok(ActionType::InvalidateMemoryFact),
            "create_failure_insight_memory" => Ok(ActionType::CreateFailureInsightMemory),
            "read_document" => Ok(ActionType::ReadDocument),
            "write_document" => Ok(ActionType::WriteDocument),
            "propose_tool_use" => Ok(ActionType::ProposeToolUse),
            "simulate_email" => Ok(ActionType::SimulateEmail),
            "manage_task" => Ok(ActionType::ManageTask),
            other => Ok(ActionType::Custom(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn memory_write_kind_maps_to_specific_action_type() {
        assert_eq!(
            MemoryWriteKind::CreateMemoryFact.action_type(),
            ActionType::CreateMemoryFact
        );
        assert_eq!(
            MemoryWriteKind::CreateFailureInsightMemory.action_type(),
            ActionType::CreateFailureInsightMemory
        );
    }

    #[test]
    fn memory_write_intent_serializes_with_governance_metadata() {
        let proposed_at = "2026-05-21T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let intent = MemoryWriteIntent::new(
            MemoryWriteKind::CreateMemoryFact,
            MemoryWriteTarget::fact_with_value(
                "project",
                "arpagona-agent-core",
                "current_priority",
                json!("governed_memory_write_observability"),
            ),
            MemoryWriteProvenance::new(
                Some(SourceId::new("source-focus-loop")),
                "focus loop report",
                "system_observation",
                "The current priority is governed memory-write proposals.",
            ),
            0.87,
            AgentId::new("agent-alpha"),
            "Preserve the selected operational priority for future governed recall.",
            proposed_at,
        )
        .with_audit_linkage(
            Some(DecisionId::new("decision-memory-write")),
            Some(AuditEventId::new("audit-memory-write")),
        )
        .with_invalidation_note("Supersede when AGENT_FOCUS_LOOP changes priority.");

        let encoded = serde_json::to_value(&intent).expect("intent should serialize");

        assert_eq!(encoded["kind"], json!("create_memory_fact"));
        assert_eq!(encoded["target"]["entity_type"], json!("project"));
        assert_eq!(
            encoded["target"]["value"],
            json!("governed_memory_write_observability")
        );
        assert_eq!(
            encoded["provenance"]["source_id"],
            json!("source-focus-loop")
        );
        let encoded_confidence = encoded["confidence"].as_f64().unwrap();
        assert!((encoded_confidence - 0.87).abs() < 0.00001);
        assert_eq!(encoded["actor"], json!("agent-alpha"));
        assert_eq!(encoded["decision_id"], json!("decision-memory-write"));
        assert_eq!(encoded["audit_event_id"], json!("audit-memory-write"));
        assert!(encoded["invalidation_note"]
            .as_str()
            .unwrap()
            .contains("Supersede"));

        let decoded: MemoryWriteIntent =
            serde_json::from_value(encoded).expect("intent should deserialize");
        assert_eq!(decoded, intent);
    }
}
