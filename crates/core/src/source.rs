use crate::ids::SourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    UserInput,
    Document,
    Import,
    System,
    Api,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub source_type: SourceType,
    pub title: Option<String>,
    pub uri: Option<String>,
    pub content_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}
