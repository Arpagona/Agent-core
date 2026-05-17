use arpagona_core::{AuditEvent, Fact, FactId, FactStatus, Source, SourceId, WorkspaceId};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use surrealdb::engine::any::Any;
use surrealdb::sql::Thing;
use surrealdb::{Connection, Surreal};
use thiserror::Error;

pub const GRAPH_MEMORY_SCHEMA: &str = include_str!("../migrations/0001_graph_memory.surql");

#[derive(Debug, Error)]
pub enum GraphMemoryError {
    #[error("surrealdb error: {0}")]
    Surreal(#[from] surrealdb::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GraphMemoryError>;

#[async_trait]
pub trait GraphMemoryStore {
    async fn init_schema(&self) -> Result<()>;
    async fn upsert_fact(&self, fact: Fact) -> Result<()>;
    async fn get_fact(&self, id: FactId) -> Result<Option<Fact>>;
    async fn list_facts_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<Fact>>;
    async fn revoke_fact(&self, id: FactId) -> Result<()>;
    async fn upsert_source(&self, source: Source) -> Result<()>;
    async fn get_source(&self, id: SourceId) -> Result<Option<Source>>;
    async fn record_audit_event(&self, event: AuditEvent) -> Result<()>;
    async fn list_audit_events_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<AuditEvent>>;
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
impl<C> GraphMemoryStore for SurrealGraphMemoryStore<C>
where
    C: Connection + Send + Sync,
{
    async fn init_schema(&self) -> Result<()> {
        self.db.query(GRAPH_MEMORY_SCHEMA).await?.check()?;
        Ok(())
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

    async fn revoke_fact(&self, id: FactId) -> Result<()> {
        let Some(mut fact) = self.get_fact(id.clone()).await? else {
            return Ok(());
        };

        fact.status = FactStatus::Revoked;
        fact.updated_at = chrono::Utc::now();
        self.upsert_fact(fact).await
    }

    async fn upsert_source(&self, source: Source) -> Result<()> {
        self.upsert_document("source", source.id.as_str(), &source)
            .await
    }

    async fn get_source(&self, id: SourceId) -> Result<Option<Source>> {
        select_data(&self.db, "source", id.as_str()).await
    }

    async fn record_audit_event(&self, event: AuditEvent) -> Result<()> {
        let workspace_id = event.workspace_id.as_ref().map(ToString::to_string);
        self.db
            .query(
                "UPDATE type::thing('audit_event', $id) \
                 SET data = $data, workspace_id = $workspace_id, created_at = $created_at",
            )
            .bind(("id", event.id.to_string()))
            .bind(("data", serde_json::to_value(&event)?))
            .bind(("workspace_id", workspace_id))
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

#[allow(dead_code)]
fn _record_id(table: &str, id: &str) -> Thing {
    Thing::from((table, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_core::{ActorRef, AgentId, AuditEventId, AuditEventType, SourceType};
    use chrono::Utc;
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
