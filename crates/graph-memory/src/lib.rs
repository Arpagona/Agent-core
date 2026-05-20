use arpagona_core::{
    AuditEvent, DecisionId, Episode, EpisodeId, Fact, FactId, FactStatus, GraphRef, GraphRelation,
    Observation, ObservationId, ProposedActionId, Source, SourceId, TaskId, WorkspaceId,
};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use surrealdb::engine::any::Any;
use surrealdb::sql::Thing;
use surrealdb::{Connection, Surreal};
use thiserror::Error;

pub const GRAPH_MEMORY_SCHEMA: &str = include_str!("../migrations/0001_graph_memory.surql");

#[derive(Debug, Error)]
pub enum GraphMemoryError {
    #[error("surrealdb error: {0}")]
    Surreal(Box<surrealdb::Error>),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<surrealdb::Error> for GraphMemoryError {
    fn from(error: surrealdb::Error) -> Self {
        Self::Surreal(Box::new(error))
    }
}

pub type Result<T> = std::result::Result<T, GraphMemoryError>;

/// Experimental async SurrealDB adapter port.
///
/// The canonical domain contract is `arpagona_core::GraphMemoryStore`.
/// This async port exists only because the SurrealDB client is async and the
/// V0 core contract intentionally remains synchronous and database-free.
/// Keep this adapter aligned with the core model, but do not treat it as a
/// second domain source of truth.
#[async_trait]
pub trait AsyncGraphMemoryStore {
    async fn init_schema(&self) -> Result<()>;

    async fn upsert_source(&self, source: Source) -> Result<()>;
    async fn get_source(&self, id: SourceId) -> Result<Option<Source>>;

    async fn upsert_fact(&self, fact: Fact) -> Result<()>;
    async fn get_fact(&self, id: FactId) -> Result<Option<Fact>>;
    async fn list_facts_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<Fact>>;
    async fn list_active_facts_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<Fact>>;
    async fn revoke_fact(&self, id: FactId) -> Result<()>;

    async fn upsert_episode(&self, episode: Episode) -> Result<()>;
    async fn get_episode(&self, id: EpisodeId) -> Result<Option<Episode>>;

    async fn upsert_observation(&self, observation: Observation) -> Result<()>;
    async fn get_observation(&self, id: ObservationId) -> Result<Option<Observation>>;
    async fn list_observations_for_episode(
        &self,
        episode_id: EpisodeId,
    ) -> Result<Vec<Observation>>;

    async fn record_audit_event(&self, event: AuditEvent) -> Result<()>;
    async fn list_audit_events_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<AuditEvent>>;
    async fn list_audit_events_for_task(&self, task_id: TaskId) -> Result<Vec<AuditEvent>>;
    async fn list_audit_events_for_proposed_action(
        &self,
        proposed_action_id: ProposedActionId,
    ) -> Result<Vec<AuditEvent>>;
    async fn list_audit_events_for_decision(
        &self,
        decision_id: DecisionId,
    ) -> Result<Vec<AuditEvent>>;

    async fn add_relation(&self, relation: GraphRelation) -> Result<()>;
    async fn list_relations(&self) -> Result<Vec<GraphRelation>>;
    async fn list_relations_from(&self, from: GraphRef) -> Result<Vec<GraphRelation>>;
}

#[derive(Clone, Debug)]
pub struct SurrealGraphMemoryStore<C = Any>
where
    C: Connection,
{
    db: Surreal<C>,
}

impl<C> SurrealGraphMemoryStore<C>
where
    C: Connection,
{
    pub fn new(db: Surreal<C>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &Surreal<C> {
        &self.db
    }

    async fn upsert_document<T>(&self, table: &str, id: &str, data: &T) -> Result<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        let data = serde_json::to_value(data)?;
        self.db
            .query("UPDATE type::thing($table, $id) SET data = $data")
            .bind(("table", table.to_owned()))
            .bind(("id", id.to_owned()))
            .bind(("data", data))
            .await?
            .check()?;
        Ok(())
    }
}

#[async_trait]
impl<C> AsyncGraphMemoryStore for SurrealGraphMemoryStore<C>
where
    C: Connection + Send + Sync,
{
    async fn init_schema(&self) -> Result<()> {
        self.db.query(GRAPH_MEMORY_SCHEMA).await?.check()?;
        Ok(())
    }

    async fn upsert_source(&self, source: Source) -> Result<()> {
        self.upsert_document("source", source.id.as_str(), &source)
            .await
    }

    async fn get_source(&self, id: SourceId) -> Result<Option<Source>> {
        select_data(&self.db, "source", id.as_str()).await
    }

    async fn upsert_fact(&self, fact: Fact) -> Result<()> {
        let status = serde_json::to_value(&fact.status)?;
        self.db
            .query(
                "UPDATE type::thing('fact', $id) \
                 SET data = $data, entity_type = $entity_type, entity_id = $entity_id, \
                     status = $status, created_at = $created_at",
            )
            .bind(("id", fact.id.to_string()))
            .bind(("data", serde_json::to_value(&fact)?))
            .bind(("entity_type", fact.entity_type.clone()))
            .bind(("entity_id", fact.entity_id.clone()))
            .bind(("status", status.as_str().unwrap_or("unknown").to_owned()))
            .bind(("created_at", fact.created_at.to_rfc3339()))
            .await?
            .check()?;
        Ok(())
    }

    async fn get_fact(&self, id: FactId) -> Result<Option<Fact>> {
        select_data(&self.db, "fact", id.as_str()).await
    }

    async fn list_facts_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<Fact>> {
        let rows: Vec<DataRow<Fact>> = self
            .db
            .query(
                "SELECT data, created_at FROM fact \
                 WHERE entity_type = $entity_type AND entity_id = $entity_id \
                 ORDER BY created_at ASC",
            )
            .bind(("entity_type", entity_type.to_owned()))
            .bind(("entity_id", entity_id.to_owned()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn list_active_facts_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<Fact>> {
        Ok(self
            .list_facts_for_entity(entity_type, entity_id)
            .await?
            .into_iter()
            .filter(|fact| fact.status == FactStatus::Active)
            .collect())
    }

    async fn revoke_fact(&self, id: FactId) -> Result<()> {
        let Some(mut fact) = self.get_fact(id.clone()).await? else {
            return Ok(());
        };

        fact.status = FactStatus::Revoked;
        fact.updated_at = chrono::Utc::now();
        self.upsert_fact(fact).await
    }

    async fn upsert_episode(&self, episode: Episode) -> Result<()> {
        self.db
            .query(
                "UPDATE type::thing('episode', $id) \
                 SET data = $data, workspace_id = $workspace_id, created_at = $created_at",
            )
            .bind(("id", episode.id.to_string()))
            .bind(("data", serde_json::to_value(&episode)?))
            .bind(("workspace_id", episode.workspace_id.to_string()))
            .bind(("created_at", episode.created_at.to_rfc3339()))
            .await?
            .check()?;
        Ok(())
    }

    async fn get_episode(&self, id: EpisodeId) -> Result<Option<Episode>> {
        select_data(&self.db, "episode", id.as_str()).await
    }

    async fn upsert_observation(&self, observation: Observation) -> Result<()> {
        self.db
            .query(
                "UPDATE type::thing('observation', $id) \
                 SET data = $data, episode_id = $episode_id, created_at = $created_at",
            )
            .bind(("id", observation.id.to_string()))
            .bind(("data", serde_json::to_value(&observation)?))
            .bind(("episode_id", observation.episode_id.to_string()))
            .bind(("created_at", observation.created_at.to_rfc3339()))
            .await?
            .check()?;
        Ok(())
    }

    async fn get_observation(&self, id: ObservationId) -> Result<Option<Observation>> {
        select_data(&self.db, "observation", id.as_str()).await
    }

    async fn list_observations_for_episode(
        &self,
        episode_id: EpisodeId,
    ) -> Result<Vec<Observation>> {
        let rows: Vec<DataRow<Observation>> = self
            .db
            .query(
                "SELECT data, created_at FROM observation \
                 WHERE episode_id = $episode_id \
                 ORDER BY created_at ASC",
            )
            .bind(("episode_id", episode_id.to_string()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn record_audit_event(&self, event: AuditEvent) -> Result<()> {
        let workspace_id = event.workspace_id.as_ref().map(ToString::to_string);
        let task_id = event.task_id.as_ref().map(ToString::to_string);
        let proposed_action_id = event.proposed_action_id.as_ref().map(ToString::to_string);
        let decision_id = event.decision_id.as_ref().map(ToString::to_string);
        self.db
            .query(
                "UPDATE type::thing('audit_event', $id) \
                 SET data = $data, workspace_id = $workspace_id, task_id = $task_id, \
                     proposed_action_id = $proposed_action_id, decision_id = $decision_id, \
                     created_at = $created_at",
            )
            .bind(("id", event.id.to_string()))
            .bind(("data", serde_json::to_value(&event)?))
            .bind(("workspace_id", workspace_id))
            .bind(("task_id", task_id))
            .bind(("proposed_action_id", proposed_action_id))
            .bind(("decision_id", decision_id))
            .bind(("created_at", event.created_at.to_rfc3339()))
            .await?
            .check()?;
        Ok(())
    }

    async fn list_audit_events_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<AuditEvent>> {
        let rows: Vec<DataRow<AuditEvent>> = self
            .db
            .query(
                "SELECT data, created_at FROM audit_event \
                 WHERE workspace_id = $workspace_id \
                 ORDER BY created_at ASC",
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn list_audit_events_for_task(&self, task_id: TaskId) -> Result<Vec<AuditEvent>> {
        let rows: Vec<DataRow<AuditEvent>> = self
            .db
            .query(
                "SELECT data, created_at FROM audit_event \
                 WHERE task_id = $task_id \
                 ORDER BY created_at ASC",
            )
            .bind(("task_id", task_id.to_string()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn list_audit_events_for_proposed_action(
        &self,
        proposed_action_id: ProposedActionId,
    ) -> Result<Vec<AuditEvent>> {
        let rows: Vec<DataRow<AuditEvent>> = self
            .db
            .query(
                "SELECT data, created_at FROM audit_event \
                 WHERE proposed_action_id = $proposed_action_id \
                 ORDER BY created_at ASC",
            )
            .bind(("proposed_action_id", proposed_action_id.to_string()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn list_audit_events_for_decision(
        &self,
        decision_id: DecisionId,
    ) -> Result<Vec<AuditEvent>> {
        let rows: Vec<DataRow<AuditEvent>> = self
            .db
            .query(
                "SELECT data, created_at FROM audit_event \
                 WHERE decision_id = $decision_id \
                 ORDER BY created_at ASC",
            )
            .bind(("decision_id", decision_id.to_string()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn add_relation(&self, relation: GraphRelation) -> Result<()> {
        let id = graph_relation_id(&relation)?;
        self.db
            .query(
                "UPDATE type::thing('graph_relation', $id) \
                 SET data = $data, from_node_type = $from_node_type, from_node_id = $from_node_id, \
                     relation_type = $relation_type",
            )
            .bind(("id", id))
            .bind(("data", serde_json::to_value(&relation)?))
            .bind(("from_node_type", graph_ref_node_type(&relation.from)?))
            .bind(("from_node_id", relation.from.node_id.clone()))
            .bind(("relation_type", relation_type_value(&relation)?))
            .await?
            .check()?;
        Ok(())
    }

    async fn list_relations(&self) -> Result<Vec<GraphRelation>> {
        let rows: Vec<DataRow<GraphRelation>> = self
            .db
            .query("SELECT data FROM graph_relation")
            .await?
            .take(0)?;
        Ok(rows.into_iter().map(|row| row.data).collect())
    }

    async fn list_relations_from(&self, from: GraphRef) -> Result<Vec<GraphRelation>> {
        let rows: Vec<DataRow<GraphRelation>> = self
            .db
            .query(
                "SELECT data FROM graph_relation \
                 WHERE from_node_type = $from_node_type AND from_node_id = $from_node_id",
            )
            .bind(("from_node_type", graph_ref_node_type(&from)?))
            .bind(("from_node_id", from.node_id.clone()))
            .await?
            .take(0)?;

        Ok(rows
            .into_iter()
            .map(|row| row.data)
            .filter(|relation| relation.from == from)
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct DataRow<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct MaybeDataRow {
    data: Option<Value>,
}

async fn select_data<C, T>(db: &Surreal<C>, table: &str, id: &str) -> Result<Option<T>>
where
    C: Connection,
    T: DeserializeOwned,
{
    let row: Option<MaybeDataRow> = db.select((table, id)).await?;
    match row.and_then(|row| row.data) {
        Some(data) => Ok(Some(serde_json::from_value(data)?)),
        None => Ok(None),
    }
}

fn graph_relation_id(relation: &GraphRelation) -> Result<String> {
    let encoded = serde_json::to_string(relation)?;
    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    Ok(format!("relation-{:x}", hasher.finish()))
}

fn graph_ref_node_type(graph_ref: &GraphRef) -> Result<String> {
    Ok(serde_json::to_value(&graph_ref.node_type)?
        .as_str()
        .unwrap_or("other")
        .to_owned())
}

fn relation_type_value(relation: &GraphRelation) -> Result<String> {
    Ok(serde_json::to_value(&relation.relation_type)?
        .as_str()
        .unwrap_or("other")
        .to_owned())
}

#[allow(dead_code)]
fn _record_id(table: &str, id: &str) -> Thing {
    Thing::from((table, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_core::{
        ActorRef, AgentId, AuditEventId, AuditEventType, DecisionId, GraphNodeType,
        ProposedActionId, RelationType, SourceType, TaskId,
    };
    use chrono::{Duration, Utc};
    use serde_json::json;
    use surrealdb::engine::local::{Db, Mem};

    async fn memory_store() -> SurrealGraphMemoryStore<Db> {
        let db = Surreal::new::<Mem>(()).await.expect("memory db");
        db.use_ns("test_ns")
            .use_db("test_db")
            .await
            .expect("namespace");
        let store = SurrealGraphMemoryStore::new(db);
        store.init_schema().await.expect("schema initializes");
        store
    }

    fn sample_fact(id: &str) -> Fact {
        let now = Utc::now();
        Fact {
            id: FactId::new(id),
            entity_type: "company".to_owned(),
            entity_id: "arpagona".to_owned(),
            attribute: "positioning".to_owned(),
            value: json!("local-first applied AI lab"),
            source_id: Some(SourceId::new("source-1")),
            confidence: 0.95,
            valid_from: None,
            valid_to: None,
            status: FactStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_source(id: &str) -> Source {
        Source {
            id: SourceId::new(id),
            source_type: SourceType::Document,
            title: Some("Positioning note".to_owned()),
            uri: Some("file://positioning.md".to_owned()),
            content_hash: Some("sha256:test".to_owned()),
            created_at: Utc::now(),
        }
    }

    fn sample_episode(id: &str) -> Episode {
        Episode {
            id: EpisodeId::new(id),
            workspace_id: WorkspaceId::new("workspace-1"),
            task_id: Some(TaskId::new("task-1")),
            agent_id: Some(AgentId::new("agent-1")),
            summary: "Graph memory adapter test episode.".to_owned(),
            created_at: Utc::now(),
        }
    }

    fn sample_observation(id: &str, episode_id: EpisodeId) -> Observation {
        Observation {
            id: ObservationId::new(id),
            episode_id,
            content: "SurrealDB adapter stores observations.".to_owned(),
            source_id: Some(SourceId::new("source-1")),
            created_at: Utc::now(),
        }
    }

    fn sample_audit_event(id: &str, workspace_id: &str) -> AuditEvent {
        AuditEvent {
            id: AuditEventId::new(id),
            event_type: AuditEventType::ActionProposed,
            actor: ActorRef::Agent(AgentId::new("agent-1")),
            workspace_id: Some(WorkspaceId::new(workspace_id)),
            task_id: None,
            proposed_action_id: None,
            decision_id: None,
            payload: json!({"note": "recorded by graph memory test"}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn initializes_schema_in_memory() {
        let _store = memory_store().await;
    }

    #[tokio::test]
    async fn inserts_and_reads_fact() {
        let store = memory_store().await;
        let fact = sample_fact("fact-1");

        store.upsert_fact(fact.clone()).await.expect("upsert fact");
        let stored = store
            .get_fact(FactId::new("fact-1"))
            .await
            .expect("get fact")
            .expect("fact exists");

        assert_eq!(stored, fact);

        let listed = store
            .list_facts_for_entity("company", "arpagona")
            .await
            .expect("list facts");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, FactId::new("fact-1"));

        let active = store
            .list_active_facts_for_entity("company", "arpagona")
            .await
            .expect("active facts");
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn update_type_thing_creates_and_updates_fact() {
        let store = memory_store().await;
        let mut fact = sample_fact("fact-upsert");

        store.upsert_fact(fact.clone()).await.expect("create fact");

        fact.attribute = "updated_positioning".to_owned();
        fact.value = json!("updated local-first positioning");
        store.upsert_fact(fact.clone()).await.expect("update fact");

        let stored = store
            .get_fact(FactId::new("fact-upsert"))
            .await
            .expect("get fact")
            .expect("fact exists");

        assert_eq!(stored, fact);
    }

    #[tokio::test]
    async fn revokes_fact() {
        let store = memory_store().await;
        store
            .upsert_fact(sample_fact("fact-to-revoke"))
            .await
            .expect("upsert fact");

        store
            .revoke_fact(FactId::new("fact-to-revoke"))
            .await
            .expect("revoke fact");

        let stored = store
            .get_fact(FactId::new("fact-to-revoke"))
            .await
            .expect("get fact")
            .expect("fact exists");
        assert_eq!(stored.status, FactStatus::Revoked);

        let active = store
            .list_active_facts_for_entity("company", "arpagona")
            .await
            .expect("active facts");
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn inserts_and_reads_source() {
        let store = memory_store().await;
        let source = sample_source("source-1");

        store
            .upsert_source(source.clone())
            .await
            .expect("upsert source");
        let stored = store
            .get_source(SourceId::new("source-1"))
            .await
            .expect("get source")
            .expect("source exists");

        assert_eq!(stored, source);
    }

    #[tokio::test]
    async fn inserts_and_reads_episode_and_observation() {
        let store = memory_store().await;
        let episode = sample_episode("episode-1");
        let observation = sample_observation("observation-1", episode.id.clone());

        store
            .upsert_episode(episode.clone())
            .await
            .expect("upsert episode");
        store
            .upsert_observation(observation.clone())
            .await
            .expect("upsert observation");

        let stored_episode = store
            .get_episode(EpisodeId::new("episode-1"))
            .await
            .expect("get episode")
            .expect("episode exists");
        let observations = store
            .list_observations_for_episode(EpisodeId::new("episode-1"))
            .await
            .expect("list observations");

        assert_eq!(stored_episode, episode);
        assert_eq!(observations, vec![observation.clone()]);
        assert_eq!(
            store
                .get_observation(ObservationId::new("observation-1"))
                .await
                .expect("get observation"),
            Some(observation)
        );
    }

    #[tokio::test]
    async fn inserts_and_reads_graph_relation() {
        let store = memory_store().await;
        let relation = GraphRelation::new(
            GraphRef::new(GraphNodeType::Observation, "observation-1"),
            GraphRef::with_relation(GraphNodeType::Fact, "fact-1", RelationType::Supports),
            RelationType::Supports,
        );

        store
            .add_relation(relation.clone())
            .await
            .expect("add relation");

        assert_eq!(
            store.list_relations().await.unwrap(),
            vec![relation.clone()]
        );
        assert_eq!(
            store
                .list_relations_from(GraphRef::new(GraphNodeType::Observation, "observation-1"))
                .await
                .unwrap(),
            vec![relation]
        );
    }

    #[tokio::test]
    async fn records_audit_event_for_workspace() {
        let store = memory_store().await;
        let event = sample_audit_event("audit-1", "workspace-1");

        store
            .record_audit_event(event.clone())
            .await
            .expect("record audit event");

        let events = store
            .list_audit_events_for_workspace(WorkspaceId::new("workspace-1"))
            .await
            .expect("list audit events");

        assert_eq!(events, vec![event]);
    }

    #[tokio::test]
    async fn queries_audit_events_by_trace_links() {
        let store = memory_store().await;
        let task_id = TaskId::new("task-1");
        let proposed_action_id = ProposedActionId::new("action-query-1");
        let decision_id = DecisionId::new("decision-query-1");
        let event = AuditEvent {
            id: AuditEventId::new("audit-query-1"),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(task_id.clone()),
            proposed_action_id: Some(proposed_action_id.clone()),
            decision_id: Some(decision_id.clone()),
            payload: json!({"causal_trace": {"flow": "proposed_action_to_decision"}}),
            created_at: Utc::now(),
        };

        store
            .record_audit_event(event.clone())
            .await
            .expect("record decision audit event");

        assert_eq!(
            store
                .list_audit_events_for_task(task_id)
                .await
                .expect("list audit events by task"),
            vec![event.clone()]
        );
        assert_eq!(
            store
                .list_audit_events_for_proposed_action(proposed_action_id)
                .await
                .expect("list audit events by proposed action"),
            vec![event.clone()]
        );
        assert_eq!(
            store
                .list_audit_events_for_decision(decision_id)
                .await
                .expect("list audit events by decision"),
            vec![event]
        );
    }

    #[tokio::test]
    async fn returns_audit_trace_queries_in_chronological_order() {
        let store = memory_store().await;
        let workspace_id = WorkspaceId::new("workspace-1");
        let task_id = TaskId::new("task-ordered");
        let proposed_action_id = ProposedActionId::new("action-ordered");
        let decision_id = DecisionId::new("decision-ordered");
        let base_time = Utc::now();

        let older_event = AuditEvent {
            id: AuditEventId::new("audit-older"),
            event_type: AuditEventType::ActionProposed,
            actor: ActorRef::Agent(AgentId::new("agent-1")),
            workspace_id: Some(workspace_id.clone()),
            task_id: Some(task_id.clone()),
            proposed_action_id: Some(proposed_action_id.clone()),
            decision_id: Some(decision_id.clone()),
            payload: json!({"note": "older trace event"}),
            created_at: base_time,
        };
        let newer_event = AuditEvent {
            id: AuditEventId::new("audit-newer"),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(workspace_id.clone()),
            task_id: Some(task_id.clone()),
            proposed_action_id: Some(proposed_action_id.clone()),
            decision_id: Some(decision_id.clone()),
            payload: json!({"note": "newer trace event"}),
            created_at: base_time + Duration::seconds(5),
        };

        store
            .record_audit_event(newer_event.clone())
            .await
            .expect("record newer audit event first");
        store
            .record_audit_event(older_event.clone())
            .await
            .expect("record older audit event second");

        assert_eq!(
            store
                .list_audit_events_for_workspace(workspace_id)
                .await
                .expect("list ordered workspace events"),
            vec![older_event.clone(), newer_event.clone()]
        );
        assert_eq!(
            store
                .list_audit_events_for_task(task_id)
                .await
                .expect("list ordered task events"),
            vec![older_event.clone(), newer_event.clone()]
        );
        assert_eq!(
            store
                .list_audit_events_for_proposed_action(proposed_action_id)
                .await
                .expect("list ordered proposed action events"),
            vec![older_event.clone(), newer_event.clone()]
        );
        assert_eq!(
            store
                .list_audit_events_for_decision(decision_id)
                .await
                .expect("list ordered decision events"),
            vec![older_event, newer_event]
        );
    }

    #[tokio::test]
    async fn persists_decision_created_audit_trace_links() {
        let store = memory_store().await;
        let proposed_action_id = ProposedActionId::new("action-1");
        let decision_id = DecisionId::new("decision-1");
        let event = AuditEvent {
            id: AuditEventId::new("audit-decision-1"),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(TaskId::new("task-1")),
            proposed_action_id: Some(proposed_action_id.clone()),
            decision_id: Some(decision_id.clone()),
            payload: json!({
                "causal_trace": {
                    "flow": "proposed_action_to_decision",
                    "decision_status": "needs_human_approval",
                    "reason": "medium risk action requires human validation"
                }
            }),
            created_at: Utc::now(),
        };

        store
            .record_audit_event(event.clone())
            .await
            .expect("record decision audit event");

        let events = store
            .list_audit_events_for_workspace(WorkspaceId::new("workspace-1"))
            .await
            .expect("list audit events");

        assert_eq!(events, vec![event.clone()]);
        assert_eq!(events[0].event_type, AuditEventType::DecisionCreated);
        assert_eq!(events[0].proposed_action_id, Some(proposed_action_id));
        assert_eq!(events[0].decision_id, Some(decision_id));
        assert_eq!(
            events[0].payload["causal_trace"]["flow"],
            json!("proposed_action_to_decision")
        );
    }

    #[test]
    fn serializes_and_deserializes_core_types() {
        let fact = sample_fact("fact-json");
        let source = sample_source("source-json");
        let event = sample_audit_event("audit-json", "workspace-json");

        let fact_json = serde_json::to_string(&fact).expect("serialize fact");
        let source_json = serde_json::to_string(&source).expect("serialize source");
        let event_json = serde_json::to_string(&event).expect("serialize audit event");

        assert_eq!(serde_json::from_str::<Fact>(&fact_json).unwrap(), fact);
        assert_eq!(
            serde_json::from_str::<Source>(&source_json).unwrap(),
            source
        );
        assert_eq!(
            serde_json::from_str::<AuditEvent>(&event_json).unwrap(),
            event
        );
    }
}
