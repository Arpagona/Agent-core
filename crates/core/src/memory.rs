use crate::ids::{FactId, SourceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Active,
    Superseded,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub id: FactId,
    pub entity_type: String,
    pub entity_id: String,
    pub attribute: String,
    pub value: Value,
    pub source_id: Option<SourceId>,
    pub confidence: f32,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub status: FactStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
