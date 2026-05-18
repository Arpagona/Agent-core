use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeType {
    Workspace,
    Agent,
    Task,
    Goal,
    ProposedAction,
    Decision,
    Policy,
    Fact,
    Source,
    Episode,
    Observation,
    AuditEvent,
    Tool,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    References,
    Supports,
    Contradicts,
    DerivedFrom,
    RelatedTo,
    Supersedes,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphRef {
    pub node_type: GraphNodeType,
    pub node_id: String,
    pub relation_type: Option<RelationType>,
}

impl GraphRef {
    pub fn new(node_type: GraphNodeType, node_id: impl Into<String>) -> Self {
        Self {
            node_type,
            node_id: node_id.into(),
            relation_type: None,
        }
    }

    pub fn with_relation(
        node_type: GraphNodeType,
        node_id: impl Into<String>,
        relation_type: RelationType,
    ) -> Self {
        Self {
            node_type,
            node_id: node_id.into(),
            relation_type: Some(relation_type),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphRelation {
    pub from: GraphRef,
    pub to: GraphRef,
    pub relation_type: RelationType,
}

impl GraphRelation {
    pub fn new(from: GraphRef, to: GraphRef, relation_type: RelationType) -> Self {
        Self {
            from,
            to,
            relation_type,
        }
    }
}
