use arpagona_core::{
    AuditEvent, AuditTraceSummary, Decision, DecisionId, DecisionStatus, Episode, EpisodeId, Fact,
    FactId, FactStatus, FailureInsight, FailureInsightId, GraphNodeType, GraphRef, GraphRelation,
    MemoryWriteIntent, MemoryWriteKind, Observation, ObservationId, ProposedActionId, RelationType,
    Source, SourceId, SourceType, TaskId, WorkspaceId,
};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use surrealdb::engine::any::Any;
use surrealdb::engine::local::{Db, Mem};
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
    #[error("invalid governed memory write: {0}")]
    InvalidGovernedMemoryWrite(String),
}

impl From<surrealdb::Error> for GraphMemoryError {
    fn from(error: surrealdb::Error) -> Self {
        Self::Surreal(Box::new(error))
    }
}

pub type Result<T> = std::result::Result<T, GraphMemoryError>;

/// Create an initialized in-memory SurrealDB-backed Graph Memory store.
///
/// This is intended for repeatable local demos and tests. It does not connect
/// to a durable user database and should not be treated as production memory.
pub async fn in_memory_graph_memory_store(
    namespace: &str,
    database: &str,
) -> Result<SurrealGraphMemoryStore<Db>> {
    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns(namespace).use_db(database).await?;
    let store = SurrealGraphMemoryStore::new(db);
    store.init_schema().await?;
    Ok(store)
}

const FAILURE_INSIGHT_MEMORY_READBACK_WARNING: &str =
    "Readback only: persisted FailureInsight memory is evidence for supervision, not approval, authorization, policy, or execution state.";

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct FailureInsightMemoryReadback {
    pub insight: Option<FailureInsight>,
    pub decision_audit_events: Vec<AuditEvent>,
    pub insight_relations: Vec<GraphRelation>,
    pub warning: &'static str,
}

impl FailureInsightMemoryReadback {
    pub fn missing() -> Self {
        Self {
            insight: None,
            decision_audit_events: Vec::new(),
            insight_relations: Vec::new(),
            warning: FAILURE_INSIGHT_MEMORY_READBACK_WARNING,
        }
    }
}

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

    async fn upsert_failure_insight(&self, insight: FailureInsight) -> Result<()>;
    async fn get_failure_insight(&self, id: FailureInsightId) -> Result<Option<FailureInsight>>;
    async fn list_failure_insights_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<FailureInsight>>;
    async fn failure_insight_memory_readback(
        &self,
        id: FailureInsightId,
    ) -> Result<FailureInsightMemoryReadback> {
        let Some(insight) = self.get_failure_insight(id.clone()).await? else {
            return Ok(FailureInsightMemoryReadback::missing());
        };
        let decision_audit_events = match insight.decision_id.clone() {
            Some(decision_id) => self.list_audit_events_for_decision(decision_id).await?,
            None => Vec::new(),
        };
        let insight_relations = self
            .list_relations_from(GraphRef::new(
                GraphNodeType::Other("failure_insight".to_owned()),
                id.to_string(),
            ))
            .await?;

        Ok(FailureInsightMemoryReadback {
            insight: Some(insight),
            decision_audit_events,
            insight_relations,
            warning: FAILURE_INSIGHT_MEMORY_READBACK_WARNING,
        })
    }

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
    async fn audit_trace_summary_for_decision(
        &self,
        decision_id: DecisionId,
    ) -> Result<AuditTraceSummary> {
        let events = self
            .list_audit_events_for_decision(decision_id.clone())
            .await?;
        let mut summary = AuditTraceSummary::from_events(&events);
        summary.decision_id = Some(decision_id);
        Ok(summary)
    }

    /// Persist a governed, approved `create_memory_fact` intent as an alpha local fact.
    ///
    /// This is a controlled persistence helper, not an authorization path. It refuses
    /// to write unless the caller supplies an approved Decision Gate result and the
    /// matching decision audit event, then records the audit event alongside the fact
    /// for later readback.
    async fn persist_approved_create_memory_fact(
        &self,
        intent: MemoryWriteIntent,
        decision: Decision,
        audit_event: AuditEvent,
    ) -> Result<Fact> {
        let fact = fact_from_approved_memory_intent(&intent, &decision, &audit_event)?;

        self.record_audit_event(audit_event.clone()).await?;
        if let Some(source) = source_from_memory_intent(&intent) {
            self.upsert_source(source).await?;
        }
        self.upsert_fact(fact.clone()).await?;
        self.add_relation(GraphRelation::new(
            GraphRef::new(GraphNodeType::Fact, fact.id.to_string()),
            GraphRef::with_relation(
                GraphNodeType::Decision,
                decision.id.to_string(),
                RelationType::DerivedFrom,
            ),
            RelationType::DerivedFrom,
        ))
        .await?;
        self.add_relation(GraphRelation::new(
            GraphRef::new(GraphNodeType::Fact, fact.id.to_string()),
            GraphRef::with_relation(
                GraphNodeType::AuditEvent,
                audit_event.id.to_string(),
                RelationType::DerivedFrom,
            ),
            RelationType::DerivedFrom,
        ))
        .await?;

        Ok(fact)
    }

    /// Persist a governed, approved `create_failure_insight_memory` intent as an alpha local insight.
    ///
    /// This writes only after an approved Decision Gate result and matching audit
    /// event. The persisted `FailureInsight` remains descriptive and
    /// non-authorizing; readback never becomes approval or execution state.
    async fn persist_approved_failure_insight_memory(
        &self,
        intent: MemoryWriteIntent,
        decision: Decision,
        audit_event: AuditEvent,
    ) -> Result<FailureInsight> {
        let insight =
            failure_insight_from_approved_memory_intent(&intent, &decision, &audit_event)?;

        self.record_audit_event(audit_event.clone()).await?;
        if let Some(source) = source_from_memory_intent(&intent) {
            self.upsert_source(source).await?;
        }
        self.upsert_failure_insight(insight.clone()).await?;
        let insight_ref = GraphRef::new(
            GraphNodeType::Other("failure_insight".to_owned()),
            insight.id.to_string(),
        );
        self.add_relation(GraphRelation::new(
            insight_ref.clone(),
            GraphRef::with_relation(
                GraphNodeType::Decision,
                decision.id.to_string(),
                RelationType::DerivedFrom,
            ),
            RelationType::DerivedFrom,
        ))
        .await?;
        self.add_relation(GraphRelation::new(
            insight_ref,
            GraphRef::with_relation(
                GraphNodeType::AuditEvent,
                audit_event.id.to_string(),
                RelationType::DerivedFrom,
            ),
            RelationType::DerivedFrom,
        ))
        .await?;

        Ok(insight)
    }

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

    async fn upsert_failure_insight(&self, insight: FailureInsight) -> Result<()> {
        let workspace_id = insight.workspace_id.as_ref().map(ToString::to_string);
        let task_id = insight.task_id.as_ref().map(ToString::to_string);
        let proposed_action_id = insight.proposed_action_id.as_ref().map(ToString::to_string);
        let decision_id = insight.decision_id.as_ref().map(ToString::to_string);
        let audit_event_id = insight.audit_event_id.as_ref().map(ToString::to_string);
        self.db
            .query(
                "UPDATE type::thing('failure_insight', $id) \
                 SET data = $data, workspace_id = $workspace_id, task_id = $task_id, \
                     proposed_action_id = $proposed_action_id, decision_id = $decision_id, \
                     audit_event_id = $audit_event_id, created_at = $created_at",
            )
            .bind(("id", insight.id.to_string()))
            .bind(("data", serde_json::to_value(&insight)?))
            .bind(("workspace_id", workspace_id))
            .bind(("task_id", task_id))
            .bind(("proposed_action_id", proposed_action_id))
            .bind(("decision_id", decision_id))
            .bind(("audit_event_id", audit_event_id))
            .bind(("created_at", insight.created_at.to_rfc3339()))
            .await?
            .check()?;
        Ok(())
    }

    async fn get_failure_insight(&self, id: FailureInsightId) -> Result<Option<FailureInsight>> {
        select_data(&self.db, "failure_insight", id.as_str()).await
    }

    async fn list_failure_insights_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<FailureInsight>> {
        let rows: Vec<DataRow<FailureInsight>> = self
            .db
            .query(
                "SELECT data, created_at FROM failure_insight \
                 WHERE workspace_id = $workspace_id \
                 ORDER BY created_at ASC",
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .await?
            .take(0)?;

        Ok(rows.into_iter().map(|row| row.data).collect())
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

fn validate_approved_memory_write_links(
    intent: &MemoryWriteIntent,
    decision: &Decision,
    audit_event: &AuditEvent,
) -> Result<()> {
    if decision.status != DecisionStatus::Approved {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "decision {} is {:?}, not approved",
            decision.id, decision.status
        )));
    }
    if let Some(linked_decision_id) = &intent.decision_id {
        if linked_decision_id != &decision.id {
            return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
                "intent decision link {} does not match supplied decision {}",
                linked_decision_id, decision.id
            )));
        }
    }
    if let Some(linked_audit_event_id) = &intent.audit_event_id {
        if linked_audit_event_id != &audit_event.id {
            return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
                "intent audit link {} does not match supplied audit event {}",
                linked_audit_event_id, audit_event.id
            )));
        }
    }
    if audit_event.decision_id.as_ref() != Some(&decision.id) {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "audit event {} is not linked to decision {}",
            audit_event.id, decision.id
        )));
    }
    if audit_event.proposed_action_id.as_ref() != Some(&decision.proposed_action_id) {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "audit event {} is not linked to proposed action {}",
            audit_event.id, decision.proposed_action_id
        )));
    }
    Ok(())
}

fn fact_from_approved_memory_intent(
    intent: &MemoryWriteIntent,
    decision: &Decision,
    audit_event: &AuditEvent,
) -> Result<Fact> {
    validate_approved_memory_write_links(intent, decision, audit_event)?;
    if intent.kind != MemoryWriteKind::CreateMemoryFact {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "intent kind {:?} cannot be persisted as a memory fact",
            intent.kind
        )));
    }

    let attribute = intent.target.attribute.clone().ok_or_else(|| {
        GraphMemoryError::InvalidGovernedMemoryWrite(
            "create_memory_fact intent requires target.attribute".to_owned(),
        )
    })?;
    let value = intent.target.value.clone().ok_or_else(|| {
        GraphMemoryError::InvalidGovernedMemoryWrite(
            "create_memory_fact intent requires target.value".to_owned(),
        )
    })?;
    let now = chrono::Utc::now();

    Ok(Fact {
        id: intent
            .target
            .fact_id
            .clone()
            .unwrap_or_else(|| FactId::new(format!("fact-{}", decision.id.as_str()))),
        entity_type: intent.target.entity_type.clone(),
        entity_id: intent.target.entity_id.clone(),
        attribute,
        value,
        source_id: intent.provenance.source_id.clone(),
        confidence: intent.confidence,
        valid_from: Some(intent.proposed_at),
        valid_to: None,
        status: FactStatus::Active,
        created_at: now,
        updated_at: now,
    })
}

fn failure_insight_from_approved_memory_intent(
    intent: &MemoryWriteIntent,
    decision: &Decision,
    audit_event: &AuditEvent,
) -> Result<FailureInsight> {
    validate_approved_memory_write_links(intent, decision, audit_event)?;
    if intent.kind != MemoryWriteKind::CreateFailureInsightMemory {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "intent kind {:?} cannot be persisted as failure insight memory",
            intent.kind
        )));
    }
    if intent.target.entity_type != "failure_insight" {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "failure insight memory requires entity_type failure_insight, got {}",
            intent.target.entity_type
        )));
    }

    let failure_insight_id = intent.target.failure_insight_id.clone().ok_or_else(|| {
        GraphMemoryError::InvalidGovernedMemoryWrite(
            "create_failure_insight_memory intent requires target.failure_insight_id".to_owned(),
        )
    })?;
    if intent.target.entity_id != failure_insight_id.to_string() {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "intent entity_id {} does not match failure insight id {}",
            intent.target.entity_id, failure_insight_id
        )));
    }

    let value = intent.target.value.clone().ok_or_else(|| {
        GraphMemoryError::InvalidGovernedMemoryWrite(
            "create_failure_insight_memory intent requires target.value".to_owned(),
        )
    })?;
    let insight: FailureInsight = serde_json::from_value(value)?;
    if insight.id != failure_insight_id {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "serialized insight id {} does not match target failure insight id {}",
            insight.id, failure_insight_id
        )));
    }
    if insight.decision_id.as_ref() != Some(&decision.id) {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "failure insight {} is not linked to decision {}",
            insight.id, decision.id
        )));
    }
    if insight.audit_event_id.as_ref() != Some(&audit_event.id) {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "failure insight {} is not linked to audit event {}",
            insight.id, audit_event.id
        )));
    }
    if insight.proposed_action_id.as_ref() != Some(&decision.proposed_action_id) {
        return Err(GraphMemoryError::InvalidGovernedMemoryWrite(format!(
            "failure insight {} is not linked to proposed action {}",
            insight.id, decision.proposed_action_id
        )));
    }

    Ok(insight)
}

fn source_from_memory_intent(intent: &MemoryWriteIntent) -> Option<Source> {
    intent
        .provenance
        .source_id
        .as_ref()
        .map(|source_id| Source {
            id: source_id.clone(),
            source_type: source_type_from_memory_source_kind(&intent.provenance.source_kind),
            title: Some(intent.provenance.source_label.clone()),
            uri: None,
            content_hash: None,
            created_at: intent.proposed_at,
        })
}

fn source_type_from_memory_source_kind(source_kind: &str) -> SourceType {
    match source_kind {
        "user_input" => SourceType::UserInput,
        "document" => SourceType::Document,
        "import" => SourceType::Import,
        "system" | "system_observation" => SourceType::System,
        "api" => SourceType::Api,
        other => SourceType::Other(other.to_owned()),
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
        ActorRef, AgentId, AuditEventId, AuditEventType, CorrectionTarget, Decision, DecisionId,
        DecisionStatus, DetectionSignal, DetectionSignalType, FailureClass, FailureInsight,
        FailureInsightId, GraphNodeType, InsightSeverity, MemoryWriteIntent, MemoryWriteKind,
        MemoryWriteProvenance, MemoryWriteTarget, PolicyId, ProposedActionId, RelationType,
        RiskLevel, SourceType, TaskId,
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

    fn approved_memory_decision(status: DecisionStatus) -> Decision {
        Decision {
            id: DecisionId::new("decision-approved-memory-fact"),
            proposed_action_id: ProposedActionId::new("action-approved-memory-fact"),
            status,
            reason: "Approved local project memory fact after Decision Gate evaluation.".to_owned(),
            risk_level: RiskLevel::Low,
            policies_applied: vec![PolicyId::new("policy-local-project-memory")],
            decided_by: None,
            created_at: Utc::now(),
        }
    }

    fn approved_memory_audit_event(decision: &Decision) -> AuditEvent {
        AuditEvent {
            id: AuditEventId::new("audit-approved-memory-fact"),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(TaskId::new("task-1")),
            proposed_action_id: Some(decision.proposed_action_id.clone()),
            decision_id: Some(decision.id.clone()),
            payload: json!({
                "causal_trace": {
                    "decision_status": decision.status,
                    "action_type": "create_memory_fact",
                    "fact_id": "fact-approved-memory-fact"
                }
            }),
            created_at: Utc::now(),
        }
    }

    fn approved_memory_intent(decision: &Decision, audit_event: &AuditEvent) -> MemoryWriteIntent {
        MemoryWriteIntent::new(
            MemoryWriteKind::CreateMemoryFact,
            MemoryWriteTarget::fact_with_value(
                "project",
                "arpagona-agent-core",
                "current_priority",
                json!("controlled local Graph Memory persistence"),
            ),
            MemoryWriteProvenance::new(
                Some(SourceId::new("source-approved-memory-fact")),
                "focus loop approved memory proposal",
                "system_observation",
                "Decision Gate approved a safe local project memory fact.",
            ),
            0.91,
            AgentId::new("agent-1"),
            "Remember approved operational project memory for later inspection.",
            Utc::now(),
        )
        .with_audit_linkage(Some(decision.id.clone()), Some(audit_event.id.clone()))
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
    async fn persists_approved_create_memory_fact_with_audit_readback() {
        let store = memory_store().await;
        let decision = approved_memory_decision(DecisionStatus::Approved);
        let audit_event = approved_memory_audit_event(&decision);
        let intent = approved_memory_intent(&decision, &audit_event);

        let fact = store
            .persist_approved_create_memory_fact(
                intent.clone(),
                decision.clone(),
                audit_event.clone(),
            )
            .await
            .expect("approved create_memory_fact intent should persist");

        assert_eq!(fact.entity_type, "project");
        assert_eq!(fact.entity_id, "arpagona-agent-core");
        assert_eq!(fact.attribute, "current_priority");
        assert_eq!(
            fact.value,
            json!("controlled local Graph Memory persistence")
        );
        assert_eq!(fact.source_id, intent.provenance.source_id);
        assert_eq!(fact.status, FactStatus::Active);

        let stored_fact = store
            .get_fact(fact.id.clone())
            .await
            .expect("fact readback succeeds")
            .expect("fact was persisted");
        assert_eq!(stored_fact, fact.clone());

        let decision_events = store
            .list_audit_events_for_decision(decision.id.clone())
            .await
            .expect("decision audit readback succeeds");
        assert_eq!(decision_events, vec![audit_event.clone()]);
        assert_eq!(
            store
                .get_source(SourceId::new("source-approved-memory-fact"))
                .await
                .expect("source readback succeeds")
                .expect("source exists")
                .source_type,
            SourceType::System
        );
        assert!(store
            .list_relations_from(GraphRef::new(GraphNodeType::Fact, fact.id.to_string()))
            .await
            .expect("fact relation readback succeeds")
            .iter()
            .any(|relation| relation.to.node_type == GraphNodeType::AuditEvent));
    }

    #[tokio::test]
    async fn rejects_non_approved_create_memory_fact_without_persisting() {
        let store = memory_store().await;
        let decision = approved_memory_decision(DecisionStatus::NeedsHumanApproval);
        let audit_event = approved_memory_audit_event(&decision);
        let intent = approved_memory_intent(&decision, &audit_event);

        let error = store
            .persist_approved_create_memory_fact(intent, decision.clone(), audit_event)
            .await
            .expect_err("non-approved decision must not persist memory facts");

        assert!(matches!(
            error,
            GraphMemoryError::InvalidGovernedMemoryWrite(_)
        ));
        assert!(store
            .list_active_facts_for_entity("project", "arpagona-agent-core")
            .await
            .expect("active fact readback succeeds")
            .is_empty());
        assert!(store
            .list_audit_events_for_decision(decision.id)
            .await
            .expect("audit readback succeeds")
            .is_empty());
    }

    fn sample_failure_insight(decision: &Decision, audit_event: &AuditEvent) -> FailureInsight {
        FailureInsight::new(
            FailureInsightId::new("insight-approved-memory"),
            FailureClass::InsufficientObservability,
            InsightSeverity::Low,
            CorrectionTarget::Memory,
            "Approved failure insight memory should be inspectable.",
            "The focus loop produced a durable learning candidate.",
            "Future loops can inspect the correction without treating it as authorization.",
            "Persist only after Decision Gate approval and audit linkage.",
            "Graph Memory / Failure-to-Insight",
            DetectionSignal::new(
                DetectionSignalType::RuntimeObservation,
                "Controlled local persistence test observed an approved insight.",
            ),
            0.88,
            Utc::now(),
        )
        .with_trace_links(
            Some(WorkspaceId::new("workspace-1")),
            Some(TaskId::new("task-1")),
            Some(decision.proposed_action_id.clone()),
            Some(decision.id.clone()),
            Some(audit_event.id.clone()),
        )
    }

    fn approved_failure_insight_intent(
        decision: &Decision,
        audit_event: &AuditEvent,
        insight: &FailureInsight,
    ) -> MemoryWriteIntent {
        MemoryWriteIntent::new(
            MemoryWriteKind::CreateFailureInsightMemory,
            MemoryWriteTarget {
                entity_type: "failure_insight".to_owned(),
                entity_id: insight.id.to_string(),
                attribute: Some("insight".to_owned()),
                value: Some(serde_json::to_value(insight).expect("failure insight serializes")),
                fact_id: None,
                related_fact_id: None,
                failure_insight_id: Some(insight.id.clone()),
            },
            MemoryWriteProvenance::new(
                Some(SourceId::new("source-approved-failure-insight")),
                "focus loop approved failure insight proposal",
                "system_observation",
                "Decision Gate approved a safe local FailureInsight memory artifact.",
            ),
            insight.confidence,
            AgentId::new("agent-1"),
            "Remember approved Failure-to-Insight learning for later inspection.",
            insight.created_at,
        )
        .with_audit_linkage(Some(decision.id.clone()), Some(audit_event.id.clone()))
    }

    #[tokio::test]
    async fn persists_approved_failure_insight_memory_with_audit_readback() {
        let store = memory_store().await;
        let decision = Decision {
            id: DecisionId::new("decision-approved-failure-insight"),
            proposed_action_id: ProposedActionId::new("action-approved-failure-insight"),
            status: DecisionStatus::Approved,
            reason: "Approved local operational FailureInsight after Decision Gate evaluation."
                .to_owned(),
            risk_level: RiskLevel::Low,
            policies_applied: vec![PolicyId::new("policy-local-project-memory")],
            decided_by: None,
            created_at: Utc::now(),
        };
        let audit_event = approved_memory_audit_event(&decision);
        let insight = sample_failure_insight(&decision, &audit_event);
        let intent = approved_failure_insight_intent(&decision, &audit_event, &insight);

        let persisted = store
            .persist_approved_failure_insight_memory(
                intent.clone(),
                decision.clone(),
                audit_event.clone(),
            )
            .await
            .expect("approved create_failure_insight_memory intent should persist");

        assert_eq!(persisted, insight);
        assert_eq!(
            store
                .get_failure_insight(FailureInsightId::new("insight-approved-memory"))
                .await
                .expect("failure insight readback succeeds"),
            Some(insight.clone())
        );
        assert_eq!(
            store
                .list_failure_insights_for_workspace(WorkspaceId::new("workspace-1"))
                .await
                .expect("workspace insight readback succeeds"),
            vec![insight.clone()]
        );
        assert_eq!(
            store
                .list_audit_events_for_decision(decision.id.clone())
                .await
                .expect("decision audit readback succeeds"),
            vec![audit_event.clone()]
        );
        assert!(store
            .list_relations_from(GraphRef::new(
                GraphNodeType::Other("failure_insight".to_owned()),
                insight.id.to_string(),
            ))
            .await
            .expect("failure insight relation readback succeeds")
            .iter()
            .any(|relation| relation.to.node_type == GraphNodeType::AuditEvent));
        assert_eq!(
            store
                .get_source(SourceId::new("source-approved-failure-insight"))
                .await
                .expect("source readback succeeds")
                .expect("source exists")
                .source_type,
            SourceType::System
        );
    }

    #[tokio::test]
    async fn reads_back_persisted_failure_insight_memory_with_trace_proof() {
        let store = memory_store().await;
        let decision = Decision {
            id: DecisionId::new("decision-readback-failure-insight"),
            proposed_action_id: ProposedActionId::new("action-readback-failure-insight"),
            status: DecisionStatus::Approved,
            reason: "Approved local FailureInsight memory for readback proof.".to_owned(),
            risk_level: RiskLevel::Low,
            policies_applied: vec![PolicyId::new("policy-local-project-memory")],
            decided_by: None,
            created_at: Utc::now(),
        };
        let audit_event = approved_memory_audit_event(&decision);
        let insight = sample_failure_insight(&decision, &audit_event);
        let intent = approved_failure_insight_intent(&decision, &audit_event, &insight);

        store
            .persist_approved_failure_insight_memory(intent, decision.clone(), audit_event.clone())
            .await
            .expect("approved failure insight memory persists");

        let readback = store
            .failure_insight_memory_readback(insight.id.clone())
            .await
            .expect("failure insight memory readback succeeds");

        assert_eq!(readback.insight, Some(insight.clone()));
        assert_eq!(readback.decision_audit_events, vec![audit_event.clone()]);
        assert_eq!(readback.warning, FAILURE_INSIGHT_MEMORY_READBACK_WARNING);
        assert!(readback
            .insight_relations
            .iter()
            .any(|relation| relation.to.node_type == GraphNodeType::Decision));
        assert!(readback
            .insight_relations
            .iter()
            .any(|relation| relation.to.node_type == GraphNodeType::AuditEvent));
    }

    #[tokio::test]
    async fn missing_failure_insight_memory_readback_is_non_authorizing_empty_proof() {
        let store = memory_store().await;

        let readback = store
            .failure_insight_memory_readback(FailureInsightId::new("missing-insight"))
            .await
            .expect("missing failure insight readback succeeds");

        assert_eq!(readback, FailureInsightMemoryReadback::missing());
    }

    #[tokio::test]
    async fn rejects_non_approved_failure_insight_memory_without_persisting() {
        let store = memory_store().await;
        let decision = Decision {
            id: DecisionId::new("decision-rejected-failure-insight"),
            proposed_action_id: ProposedActionId::new("action-rejected-failure-insight"),
            status: DecisionStatus::NeedsHumanApproval,
            reason: "Human confirmation is required before persisting FailureInsight memory."
                .to_owned(),
            risk_level: RiskLevel::Medium,
            policies_applied: vec![PolicyId::new("policy-human-confirmation")],
            decided_by: None,
            created_at: Utc::now(),
        };
        let audit_event = approved_memory_audit_event(&decision);
        let insight = sample_failure_insight(&decision, &audit_event);
        let intent = approved_failure_insight_intent(&decision, &audit_event, &insight);

        let error = store
            .persist_approved_failure_insight_memory(intent, decision.clone(), audit_event)
            .await
            .expect_err("non-approved decision must not persist failure insights");

        assert!(matches!(
            error,
            GraphMemoryError::InvalidGovernedMemoryWrite(_)
        ));
        assert!(store
            .list_failure_insights_for_workspace(WorkspaceId::new("workspace-1"))
            .await
            .expect("workspace insight readback succeeds")
            .is_empty());
        assert!(store
            .list_audit_events_for_decision(decision.id)
            .await
            .expect("audit readback succeeds")
            .is_empty());
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
    async fn persists_canonical_decision_created_audit_trace_links() {
        let store = memory_store().await;
        let decision = Decision {
            id: DecisionId::new("decision-1"),
            proposed_action_id: ProposedActionId::new("action-1"),
            status: DecisionStatus::NeedsHumanApproval,
            reason: "medium risk action requires human validation".to_owned(),
            risk_level: RiskLevel::Medium,
            policies_applied: vec![PolicyId::new("policy-human-approval")],
            decided_by: None,
            created_at: Utc::now(),
        };
        let event_at = Utc::now();
        let event = AuditEvent::decision_created(
            AuditEventId::new("audit-decision-1"),
            ActorRef::System,
            WorkspaceId::new("workspace-1"),
            Some(TaskId::new("task-1")),
            &decision,
            event_at,
        );

        store
            .record_audit_event(event.clone())
            .await
            .expect("record canonical decision audit event");

        let workspace_events = store
            .list_audit_events_for_workspace(WorkspaceId::new("workspace-1"))
            .await
            .expect("list workspace audit events");
        let action_events = store
            .list_audit_events_for_proposed_action(ProposedActionId::new("action-1"))
            .await
            .expect("list proposed action audit events");
        let decision_events = store
            .list_audit_events_for_decision(DecisionId::new("decision-1"))
            .await
            .expect("list decision audit events");

        assert_eq!(workspace_events, vec![event.clone()]);
        assert_eq!(action_events, vec![event.clone()]);
        assert_eq!(decision_events, vec![event.clone()]);
        assert_eq!(
            workspace_events[0].event_type,
            AuditEventType::DecisionCreated
        );
        assert_eq!(
            workspace_events[0].proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(
            workspace_events[0].decision_id,
            Some(DecisionId::new("decision-1"))
        );
        assert_eq!(
            workspace_events[0].payload["causal_trace"]["decision_status"],
            json!("needs_human_approval")
        );
        assert_eq!(
            workspace_events[0].payload["causal_trace"]["policies_applied"][0],
            json!("policy-human-approval")
        );

        let summary = store
            .audit_trace_summary_for_decision(DecisionId::new("decision-1"))
            .await
            .expect("decision audit trace summary");
        assert_eq!(summary.event_count, 1);
        assert_eq!(
            summary.first_event_id,
            Some(AuditEventId::new("audit-decision-1"))
        );
        assert_eq!(
            summary.last_event_id,
            Some(AuditEventId::new("audit-decision-1"))
        );
        assert_eq!(summary.first_event_at, Some(event_at));
        assert_eq!(summary.last_event_at, Some(event_at));
        assert_eq!(summary.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(summary.task_id, Some(TaskId::new("task-1")));
        assert_eq!(
            summary.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-1")));
        assert!(!summary.has_action_proposed);
        assert!(summary.has_decision_created);
        assert!(!summary.has_execution_event);
    }

    #[tokio::test]
    async fn decision_trace_summary_keeps_human_request_readback_non_authorizing() {
        let store = memory_store().await;
        let workspace_id = WorkspaceId::new("workspace-human-request");
        let task_id = TaskId::new("task-human-request");
        let proposed_action_id = ProposedActionId::new("action-human-request");
        let decision_id = DecisionId::new("decision-human-request");
        let base_time = Utc::now();

        let decision_event = AuditEvent {
            id: AuditEventId::new("audit-human-request-decision"),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: Some(workspace_id.clone()),
            task_id: Some(task_id.clone()),
            proposed_action_id: Some(proposed_action_id.clone()),
            decision_id: Some(decision_id.clone()),
            payload: json!({"causal_trace": {"decision_status": "needs_human_approval"}}),
            created_at: base_time,
        };
        let human_request_event = AuditEvent {
            id: AuditEventId::new("audit-human-request-sent"),
            event_type: AuditEventType::HumanApprovalRequested,
            actor: ActorRef::System,
            workspace_id: Some(workspace_id.clone()),
            task_id: Some(task_id.clone()),
            proposed_action_id: Some(proposed_action_id.clone()),
            decision_id: Some(decision_id.clone()),
            payload: json!({"readback_only": true}),
            created_at: base_time + Duration::seconds(5),
        };

        store
            .record_audit_event(human_request_event.clone())
            .await
            .expect("record human approval request first");
        store
            .record_audit_event(decision_event.clone())
            .await
            .expect("record decision audit event second");

        let summary = store
            .audit_trace_summary_for_decision(decision_id.clone())
            .await
            .expect("decision audit trace summary");

        assert_eq!(summary.event_count, 2);
        assert_eq!(
            summary.first_event_id,
            Some(AuditEventId::new("audit-human-request-decision"))
        );
        assert_eq!(
            summary.last_event_id,
            Some(AuditEventId::new("audit-human-request-sent"))
        );
        assert_eq!(summary.first_event_at, Some(base_time));
        assert_eq!(
            summary.last_event_at,
            Some(base_time + Duration::seconds(5))
        );
        assert_eq!(summary.workspace_id, Some(workspace_id));
        assert_eq!(summary.task_id, Some(task_id));
        assert_eq!(summary.proposed_action_id, Some(proposed_action_id));
        assert_eq!(summary.decision_id, Some(decision_id));
        assert!(!summary.has_action_proposed);
        assert!(summary.has_decision_created);
        assert!(summary.has_human_approval_request);
        assert!(!summary.has_human_outcome);
        assert!(!summary.has_execution_event);
    }

    #[tokio::test]
    async fn decision_trace_summary_preserves_decision_scope_when_empty() {
        let store = memory_store().await;
        let decision_id = DecisionId::new("decision-empty");

        let summary = store
            .audit_trace_summary_for_decision(decision_id.clone())
            .await
            .expect("empty decision audit trace summary");

        assert_eq!(summary.event_count, 0);
        assert_eq!(summary.first_event_id, None);
        assert_eq!(summary.last_event_id, None);
        assert_eq!(summary.workspace_id, None);
        assert_eq!(summary.task_id, None);
        assert_eq!(summary.proposed_action_id, None);
        assert_eq!(summary.decision_id, Some(decision_id));
        assert!(!summary.has_action_proposed);
        assert!(!summary.has_decision_created);
        assert!(!summary.has_human_approval_request);
        assert!(!summary.has_human_outcome);
        assert!(!summary.has_execution_event);
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
