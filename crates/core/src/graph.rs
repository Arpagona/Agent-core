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
