use crate::audit::AuditEvent;
use crate::episode::{Episode, Observation};
use crate::errors::CoreError;
use crate::graph::{GraphNodeType, GraphRef, GraphRelation, RelationType};
use crate::ids::{
    AuditEventId, DecisionId, EpisodeId, FactId, ObservationId, ProposedActionId, SourceId, TaskId,
    WorkspaceId,
};
use crate::source::Source;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

pub type GraphMemoryResult<T> = Result<T, CoreError>;

/// Minimal synchronous graph-memory port for the pure core domain.
///
/// Implementations are storage adapters only: they persist and query graph-domain
/// objects. They must not execute tools, call LLMs, open APIs, or bypass the
/// `Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit` flow.
pub trait GraphMemoryStore {
    fn upsert_source(&mut self, source: Source) -> GraphMemoryResult<()>;
    fn get_source(&self, id: &SourceId) -> GraphMemoryResult<Option<Source>>;

    fn upsert_fact(&mut self, fact: Fact) -> GraphMemoryResult<()>;
    fn get_fact(&self, id: &FactId) -> GraphMemoryResult<Option<Fact>>;
    fn list_facts_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> GraphMemoryResult<Vec<Fact>>;
    fn list_active_facts_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> GraphMemoryResult<Vec<Fact>>;

    fn upsert_episode(&mut self, episode: Episode) -> GraphMemoryResult<()>;
    fn get_episode(&self, id: &EpisodeId) -> GraphMemoryResult<Option<Episode>>;

    fn upsert_observation(&mut self, observation: Observation) -> GraphMemoryResult<()>;
    fn get_observation(&self, id: &ObservationId) -> GraphMemoryResult<Option<Observation>>;
    fn list_observations_for_episode(
        &self,
        episode_id: &EpisodeId,
    ) -> GraphMemoryResult<Vec<Observation>>;

    fn record_audit_event(&mut self, event: AuditEvent) -> GraphMemoryResult<()>;
    fn get_audit_event(&self, id: &AuditEventId) -> GraphMemoryResult<Option<AuditEvent>>;
    fn list_audit_events_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> GraphMemoryResult<Vec<AuditEvent>>;
    fn list_audit_events_for_task(&self, task_id: &TaskId) -> GraphMemoryResult<Vec<AuditEvent>>;
    fn list_audit_events_for_proposed_action(
        &self,
        proposed_action_id: &ProposedActionId,
    ) -> GraphMemoryResult<Vec<AuditEvent>>;
    fn list_audit_events_for_decision(
        &self,
        decision_id: &DecisionId,
    ) -> GraphMemoryResult<Vec<AuditEvent>>;

    fn add_relation(&mut self, relation: GraphRelation) -> GraphMemoryResult<()>;
    fn list_relations(&self) -> GraphMemoryResult<Vec<GraphRelation>>;
    fn list_relations_from(&self, from: &GraphRef) -> GraphMemoryResult<Vec<GraphRelation>>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryGraphMemoryStore {
    sources: HashMap<SourceId, Source>,
    facts: HashMap<FactId, Fact>,
    episodes: HashMap<EpisodeId, Episode>,
    observations: HashMap<ObservationId, Observation>,
    audit_events: HashMap<AuditEventId, AuditEvent>,
    relations: Vec<GraphRelation>,
}

impl InMemoryGraphMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
            && self.facts.is_empty()
            && self.episodes.is_empty()
            && self.observations.is_empty()
            && self.audit_events.is_empty()
            && self.relations.is_empty()
    }

    fn source_ref(id: &SourceId, relation_type: RelationType) -> GraphRef {
        GraphRef::with_relation(GraphNodeType::Source, id.to_string(), relation_type)
    }

    fn episode_ref(id: &EpisodeId, relation_type: RelationType) -> GraphRef {
        GraphRef::with_relation(GraphNodeType::Episode, id.to_string(), relation_type)
    }

    fn fact_ref(id: &FactId) -> GraphRef {
        GraphRef::new(GraphNodeType::Fact, id.to_string())
    }

    fn observation_ref(id: &ObservationId) -> GraphRef {
        GraphRef::new(GraphNodeType::Observation, id.to_string())
    }
}

impl GraphMemoryStore for InMemoryGraphMemoryStore {
    fn upsert_source(&mut self, source: Source) -> GraphMemoryResult<()> {
        self.sources.insert(source.id.clone(), source);
        Ok(())
    }

    fn get_source(&self, id: &SourceId) -> GraphMemoryResult<Option<Source>> {
        Ok(self.sources.get(id).cloned())
    }

    fn upsert_fact(&mut self, fact: Fact) -> GraphMemoryResult<()> {
        if let Some(source_id) = &fact.source_id {
            if !self.sources.contains_key(source_id) {
                return Err(CoreError::InvalidState(format!(
                    "fact {} references missing source {}",
                    fact.id, source_id
                )));
            }
        }

        if let Some(source_id) = &fact.source_id {
            let relation = GraphRelation::new(
                Self::fact_ref(&fact.id),
                Self::source_ref(source_id, RelationType::DerivedFrom),
                RelationType::DerivedFrom,
            );
            if !self.relations.contains(&relation) {
                self.relations.push(relation);
            }
        }

        self.facts.insert(fact.id.clone(), fact);
        Ok(())
    }

    fn get_fact(&self, id: &FactId) -> GraphMemoryResult<Option<Fact>> {
        Ok(self.facts.get(id).cloned())
    }

    fn list_facts_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> GraphMemoryResult<Vec<Fact>> {
        let mut facts = self
            .facts
            .values()
            .filter(|fact| fact.entity_type == entity_type && fact.entity_id == entity_id)
            .cloned()
            .collect::<Vec<_>>();
        facts.sort_by_key(|fact| fact.created_at);
        Ok(facts)
    }

    fn list_active_facts_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> GraphMemoryResult<Vec<Fact>> {
        Ok(self
            .list_facts_for_entity(entity_type, entity_id)?
            .into_iter()
            .filter(|fact| fact.status == FactStatus::Active)
            .collect())
    }

    fn upsert_episode(&mut self, episode: Episode) -> GraphMemoryResult<()> {
        self.episodes.insert(episode.id.clone(), episode);
        Ok(())
    }

    fn get_episode(&self, id: &EpisodeId) -> GraphMemoryResult<Option<Episode>> {
        Ok(self.episodes.get(id).cloned())
    }

    fn upsert_observation(&mut self, observation: Observation) -> GraphMemoryResult<()> {
        if !self.episodes.contains_key(&observation.episode_id) {
            return Err(CoreError::InvalidState(format!(
                "observation {} references missing episode {}",
                observation.id, observation.episode_id
            )));
        }
        if let Some(source_id) = &observation.source_id {
            if !self.sources.contains_key(source_id) {
                return Err(CoreError::InvalidState(format!(
                    "observation {} references missing source {}",
                    observation.id, source_id
                )));
            }
        }

        let episode_relation = GraphRelation::new(
            Self::observation_ref(&observation.id),
            Self::episode_ref(&observation.episode_id, RelationType::DerivedFrom),
            RelationType::DerivedFrom,
        );
        if !self.relations.contains(&episode_relation) {
            self.relations.push(episode_relation);
        }

        if let Some(source_id) = &observation.source_id {
            let source_relation = GraphRelation::new(
                Self::observation_ref(&observation.id),
                Self::source_ref(source_id, RelationType::DerivedFrom),
                RelationType::DerivedFrom,
            );
            if !self.relations.contains(&source_relation) {
                self.relations.push(source_relation);
            }
        }

        self.observations
            .insert(observation.id.clone(), observation);
        Ok(())
    }

    fn get_observation(&self, id: &ObservationId) -> GraphMemoryResult<Option<Observation>> {
        Ok(self.observations.get(id).cloned())
    }

    fn list_observations_for_episode(
        &self,
        episode_id: &EpisodeId,
    ) -> GraphMemoryResult<Vec<Observation>> {
        let mut observations = self
            .observations
            .values()
            .filter(|observation| &observation.episode_id == episode_id)
            .cloned()
            .collect::<Vec<_>>();
        observations.sort_by_key(|observation| observation.created_at);
        Ok(observations)
    }

    fn record_audit_event(&mut self, event: AuditEvent) -> GraphMemoryResult<()> {
        self.audit_events.insert(event.id.clone(), event);
        Ok(())
    }

    fn get_audit_event(&self, id: &AuditEventId) -> GraphMemoryResult<Option<AuditEvent>> {
        Ok(self.audit_events.get(id).cloned())
    }

    fn list_audit_events_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> GraphMemoryResult<Vec<AuditEvent>> {
        let mut events = self
            .audit_events
            .values()
            .filter(|event| event.workspace_id.as_ref() == Some(workspace_id))
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by_key(|event| event.created_at);
        Ok(events)
    }

    fn list_audit_events_for_task(&self, task_id: &TaskId) -> GraphMemoryResult<Vec<AuditEvent>> {
        let mut events = self
            .audit_events
            .values()
            .filter(|event| event.task_id.as_ref() == Some(task_id))
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by_key(|event| event.created_at);
        Ok(events)
    }

    fn list_audit_events_for_proposed_action(
        &self,
        proposed_action_id: &ProposedActionId,
    ) -> GraphMemoryResult<Vec<AuditEvent>> {
        let mut events = self
            .audit_events
            .values()
            .filter(|event| event.proposed_action_id.as_ref() == Some(proposed_action_id))
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by_key(|event| event.created_at);
        Ok(events)
    }

    fn list_audit_events_for_decision(
        &self,
        decision_id: &DecisionId,
    ) -> GraphMemoryResult<Vec<AuditEvent>> {
        let mut events = self
            .audit_events
            .values()
            .filter(|event| event.decision_id.as_ref() == Some(decision_id))
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by_key(|event| event.created_at);
        Ok(events)
    }

    fn add_relation(&mut self, relation: GraphRelation) -> GraphMemoryResult<()> {
        if !self.relations.contains(&relation) {
            self.relations.push(relation);
        }
        Ok(())
    }

    fn list_relations(&self) -> GraphMemoryResult<Vec<GraphRelation>> {
        Ok(self.relations.clone())
    }

    fn list_relations_from(&self, from: &GraphRef) -> GraphMemoryResult<Vec<GraphRelation>> {
        Ok(self
            .relations
            .iter()
            .filter(|relation| &relation.from == from)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{ActorRef, AuditEventType};
    use crate::ids::{AgentId, DecisionId, ProposedActionId, TaskId};
    use crate::source::SourceType;
    use serde_json::json;

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

    fn sample_fact(id: &str, source_id: SourceId, status: FactStatus) -> Fact {
        let now = Utc::now();
        Fact {
            id: FactId::new(id),
            entity_type: "company".to_owned(),
            entity_id: "arpagona".to_owned(),
            attribute: "positioning".to_owned(),
            value: json!("local-first agentic runtime"),
            source_id: Some(source_id),
            confidence: 0.95,
            valid_from: None,
            valid_to: None,
            status,
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_episode(id: &str) -> Episode {
        Episode {
            id: EpisodeId::new(id),
            workspace_id: WorkspaceId::new("workspace-1"),
            task_id: Some(TaskId::new("task-1")),
            agent_id: Some(AgentId::new("agent-1")),
            summary: "Graph memory V0 design session.".to_owned(),
            created_at: Utc::now(),
        }
    }

    fn sample_observation(id: &str, episode_id: EpisodeId, source_id: SourceId) -> Observation {
        Observation {
            id: ObservationId::new(id),
            episode_id,
            content: "Facts should remain traceable to sources.".to_owned(),
            source_id: Some(source_id),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn creates_source_and_fact_linked_to_it() {
        let mut store = InMemoryGraphMemoryStore::new();
        let source = sample_source("source-1");
        let fact = sample_fact("fact-1", source.id.clone(), FactStatus::Active);

        store.upsert_source(source.clone()).expect("source stored");
        store.upsert_fact(fact.clone()).expect("fact stored");

        assert_eq!(store.get_source(&source.id).unwrap(), Some(source));
        assert_eq!(store.get_fact(&fact.id).unwrap(), Some(fact.clone()));
        assert!(store
            .list_relations()
            .unwrap()
            .contains(&GraphRelation::new(
                GraphRef::new(GraphNodeType::Fact, "fact-1"),
                GraphRef::with_relation(
                    GraphNodeType::Source,
                    "source-1",
                    RelationType::DerivedFrom
                ),
                RelationType::DerivedFrom,
            )));
    }

    #[test]
    fn creates_episode_and_observation_linked_to_it() {
        let mut store = InMemoryGraphMemoryStore::new();
        let source = sample_source("source-1");
        let episode = sample_episode("episode-1");
        let observation =
            sample_observation("observation-1", episode.id.clone(), source.id.clone());

        store.upsert_source(source).expect("source stored");
        store
            .upsert_episode(episode.clone())
            .expect("episode stored");
        store
            .upsert_observation(observation.clone())
            .expect("observation stored");

        assert_eq!(store.get_episode(&episode.id).unwrap(), Some(episode));
        assert_eq!(
            store
                .list_observations_for_episode(&EpisodeId::new("episode-1"))
                .unwrap(),
            vec![observation]
        );
    }

    #[test]
    fn retrieves_only_active_facts_by_entity() {
        let mut store = InMemoryGraphMemoryStore::new();
        let source = sample_source("source-1");
        store.upsert_source(source.clone()).expect("source stored");
        store
            .upsert_fact(sample_fact(
                "fact-active",
                source.id.clone(),
                FactStatus::Active,
            ))
            .expect("active fact stored");
        store
            .upsert_fact(sample_fact("fact-revoked", source.id, FactStatus::Revoked))
            .expect("revoked fact stored");

        let facts = store
            .list_active_facts_for_entity("company", "arpagona")
            .expect("active facts");

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].id, FactId::new("fact-active"));
    }

    #[test]
    fn represents_supports_relation() {
        let mut store = InMemoryGraphMemoryStore::new();
        let relation = GraphRelation::new(
            GraphRef::new(GraphNodeType::Observation, "observation-1"),
            GraphRef::with_relation(GraphNodeType::Fact, "fact-1", RelationType::Supports),
            RelationType::Supports,
        );

        store
            .add_relation(relation.clone())
            .expect("relation stored");

        assert_eq!(
            store
                .list_relations_from(&GraphRef::new(GraphNodeType::Observation, "observation-1"))
                .unwrap(),
            vec![relation]
        );
    }

    #[test]
    fn serializes_and_deserializes_graph_relation() {
        let relation = GraphRelation::new(
            GraphRef::new(GraphNodeType::Fact, "fact-1"),
            GraphRef::with_relation(GraphNodeType::Source, "source-1", RelationType::DerivedFrom),
            RelationType::DerivedFrom,
        );

        let encoded = serde_json::to_string(&relation).expect("relation serializes");
        let decoded: GraphRelation = serde_json::from_str(&encoded).expect("relation deserializes");

        assert_eq!(decoded, relation);
    }

    #[test]
    fn rejects_fact_with_missing_source() {
        let mut store = InMemoryGraphMemoryStore::new();
        let fact = sample_fact(
            "fact-1",
            SourceId::new("missing-source"),
            FactStatus::Active,
        );

        let error = store
            .upsert_fact(fact)
            .expect_err("missing source should fail");

        assert!(matches!(error, CoreError::InvalidState(_)));
    }

    #[test]
    fn rejects_observation_with_missing_episode() {
        let mut store = InMemoryGraphMemoryStore::new();
        let source = sample_source("source-1");
        let observation = sample_observation(
            "observation-1",
            EpisodeId::new("missing-episode"),
            source.id.clone(),
        );
        store.upsert_source(source).expect("source stored");

        let error = store
            .upsert_observation(observation)
            .expect_err("missing episode should fail");

        assert!(matches!(error, CoreError::InvalidState(_)));
    }

    #[test]
    fn records_audit_events_without_executing_anything() {
        let mut store = InMemoryGraphMemoryStore::new();
        let task_id = TaskId::new("task-1");
        let event = AuditEvent {
            id: AuditEventId::new("audit-1"),
            event_type: AuditEventType::ActionProposed,
            actor: ActorRef::Agent(AgentId::new("agent-1")),
            workspace_id: Some(WorkspaceId::new("workspace-1")),
            task_id: Some(task_id.clone()),
            proposed_action_id: None,
            decision_id: None,
            payload: json!({"note": "graph memory storage only"}),
            created_at: Utc::now(),
        };

        store
            .record_audit_event(event.clone())
            .expect("audit event stored");

        assert_eq!(
            store
                .list_audit_events_for_workspace(&WorkspaceId::new("workspace-1"))
                .unwrap(),
            vec![event.clone()]
        );
        assert_eq!(
            store.list_audit_events_for_task(&task_id).unwrap(),
            vec![event]
        );
    }

    #[test]
    fn queries_audit_events_by_trace_links_without_execution() {
        let mut store = InMemoryGraphMemoryStore::new();
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
            payload: json!({"note": "trace query only"}),
            created_at: Utc::now(),
        };

        store
            .record_audit_event(event.clone())
            .expect("audit event stored");

        assert_eq!(
            store
                .list_audit_events_for_proposed_action(&proposed_action_id)
                .unwrap(),
            vec![event.clone()]
        );
        assert_eq!(
            store.list_audit_events_for_decision(&decision_id).unwrap(),
            vec![event]
        );
    }
}
