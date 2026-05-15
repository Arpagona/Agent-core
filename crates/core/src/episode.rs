use crate::ids::{AgentId, EpisodeId, ObservationId, SourceId, TaskId, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub id: EpisodeId,
    pub workspace_id: WorkspaceId,
    pub task_id: Option<TaskId>,
    pub agent_id: Option<AgentId>,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: ObservationId,
    pub episode_id: EpisodeId,
    pub content: String,
    pub source_id: Option<SourceId>,
    pub created_at: DateTime<Utc>,
}
