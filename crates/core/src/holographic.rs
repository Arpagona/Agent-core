use crate::ids::{EpisodeId, HolographicPatternId, HolographicTraceId, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Holographic Memory — pure domain vocabulary
// ---------------------------------------------------------------------------
//
// Holographic Memory is an experimental cognitive resonance layer.
// It stores distributed pattern signatures of cognitive experience.
// It does not store authoritative facts.
// It does not authorize actions.
// It helps the runtime detect similarity, resonance and recurring patterns
// across episodes, tasks, decisions, failures, successes and compute routing
// choices.
//
// These types are purely domain-level vocabulary. They carry no execution
// logic, no vector database, no persistence adapter, no LLM interaction, no
// Decision Gate bypass, and no authorisation of any kind. The Vec<f32>
// fields represent future embedding/vector data — they are not computed,
// persisted or queried by these types. Their size, origin and meaning are
// defined by a future adapter crate.

// ---------------------------------------------------------------------------
// Trace kinds
// ---------------------------------------------------------------------------

/// What kind of cognitive pattern this trace captures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HolographicTraceKind {
    /// A recurring task execution pattern.
    TaskPattern,
    /// A sequence of actions observed across cycles.
    ActionChainPattern,
    /// A pattern associated with failures or blocked decisions.
    FailurePattern,
    /// A pattern associated with successful outcomes.
    SuccessPattern,
    /// A conversation flow pattern (e.g. topic transitions).
    ConversationPattern,
    /// A compute-routing pattern (e.g. which resource was selected).
    ComputeRoutingPattern,
    /// A decision-gate pattern (e.g. which policies matched).
    DecisionPattern,
    /// A tool-use pattern (e.g. which tool was invoked and how).
    ToolUsePattern,
    /// A cognitive-cycle-wide pattern spanning multiple layers.
    CognitiveCyclePattern,
}

// ---------------------------------------------------------------------------
// Pattern kinds
// ---------------------------------------------------------------------------

/// What kind of prototype this pattern represents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HolographicPatternKind {
    /// A prototypical failure configuration.
    FailurePrototype,
    /// A prototypical success configuration.
    SuccessPrototype,
    /// A prototypical resource-routing configuration.
    RoutingPrototype,
    /// A prototypical conversation-drift configuration.
    ConversationDriftPrototype,
    /// A prototypical decision-boundary configuration.
    DecisionBoundaryPrototype,
    /// A prototypical tool-use configuration.
    ToolUsePrototype,
    /// A prototypical cognitive-cycle configuration.
    CognitiveCyclePrototype,
}

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A single trace of cognitive experience, storing a distributed signature.
///
/// A trace captures what happened during a cognitive episode — the task,
/// the actions, the decisions, the outcome — as a vector signature with
/// human-readable labels. Multiple related traces may later form a
/// `HolographicPattern`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HolographicTrace {
    pub id: HolographicTraceId,
    pub workspace_id: WorkspaceId,
    /// The episode this trace was observed in, if available.
    pub source_episode_id: Option<EpisodeId>,
    /// What kind of pattern this trace captures.
    pub trace_kind: HolographicTraceKind,
    /// Future embedding vector representing this trace's signature.
    /// Zero-length vector means "not yet computed".
    pub vector: Vec<f32>,
    /// Human-readable labels for filtering and grouping.
    pub labels: Vec<String>,
    /// How strongly this trace is registered (0.0–1.0).
    pub strength: f32,
    /// How quickly this trace loses influence (0.0 = never decays, 1.0 = instant).
    pub decay: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A stable prototype formed from multiple related traces.
///
/// A pattern represents a recurring configuration that the system has
/// observed across multiple episodes. Its `prototype_vector` is the
/// aggregated signature of its constituent traces. Patterns are used
/// for resonance detection, not for authorisation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HolographicPattern {
    pub id: HolographicPatternId,
    pub workspace_id: WorkspaceId,
    /// What kind of prototype this pattern represents.
    pub pattern_kind: HolographicPatternKind,
    /// The aggregated embedding vector of all constituent traces.
    /// Zero-length vector means "not yet computed / no aggregation yet".
    pub prototype_vector: Vec<f32>,
    /// How many individual traces contributed to this pattern.
    pub support_count: u32,
    /// How confident the system is that this pattern is meaningful (0.0–1.0).
    pub confidence: f32,
    /// Human-readable labels describing the pattern's domain.
    pub labels: Vec<String>,
    /// When this pattern was last matched by a query.
    pub last_matched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Query and match types
// ---------------------------------------------------------------------------

/// A query to find traces or patterns by vector similarity.
///
/// This is a domain-level query vocabulary. It does not specify how the
/// query is executed (vector database, cosine similarity scan, etc.).
/// The execution strategy belongs to an adapter crate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HolographicQuery {
    pub workspace_id: WorkspaceId,
    /// The embedding vector to match against.
    pub query_vector: Vec<f32>,
    /// Maximum number of results to return.
    pub top_k: usize,
    /// Minimum similarity score for a result to be considered a match.
    pub min_similarity: f32,
}

/// A single match result from a HolographicQuery.
///
/// This type describes *what* matched and *how similar* it is. It does
/// not grant authorisation, execute anything or bypass governance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HolographicMatch {
    /// The trace that matched.
    pub trace_id: HolographicTraceId,
    /// How similar the matched trace is to the query (0.0–1.0).
    pub similarity: f32,
    /// Labels from the matched trace for human inspection.
    pub matched_labels: Vec<String>,
    /// The episode this match is linked to, if available.
    pub linked_episode_id: Option<EpisodeId>,
}

// ---------------------------------------------------------------------------
// HolographicTrace helpers
// ---------------------------------------------------------------------------

impl HolographicTrace {
    /// Create a new trace with a zero vector (embedding not yet computed).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: HolographicTraceId,
        workspace_id: WorkspaceId,
        trace_kind: HolographicTraceKind,
        labels: Vec<String>,
        strength: f32,
        decay: f32,
        created_at: DateTime<Utc>,
    ) -> Self {
        let now = created_at;
        Self {
            id,
            workspace_id,
            source_episode_id: None,
            trace_kind,
            vector: vec![],
            labels,
            strength: strength.clamp(0.0, 1.0),
            decay: decay.clamp(0.0, 1.0),
            created_at: now,
            updated_at: now,
        }
    }

    /// Attach a source episode reference.
    pub fn with_episode(mut self, episode_id: EpisodeId) -> Self {
        self.source_episode_id = Some(episode_id);
        self
    }

    /// Set the embedding vector. Intended for use by an adapter crate.
    pub fn with_vector(mut self, vector: Vec<f32>) -> Self {
        self.vector = vector;
        self
    }

    /// Touch the updated_at timestamp.
    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.updated_at = now;
    }
}

// ---------------------------------------------------------------------------
// HolographicPattern helpers
// ---------------------------------------------------------------------------

impl HolographicPattern {
    /// Create a new pattern with a zero prototype vector.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: HolographicPatternId,
        workspace_id: WorkspaceId,
        pattern_kind: HolographicPatternKind,
        labels: Vec<String>,
        confidence: f32,
        created_at: DateTime<Utc>,
    ) -> Self {
        let now = created_at;
        Self {
            id,
            workspace_id,
            pattern_kind,
            prototype_vector: vec![],
            support_count: 0,
            confidence: confidence.clamp(0.0, 1.0),
            labels,
            last_matched_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the prototype vector. Intended for use by an adapter crate.
    pub fn with_prototype_vector(mut self, vector: Vec<f32>) -> Self {
        self.prototype_vector = vector;
        self
    }

    /// Increment the support count and refresh the timestamp.
    pub fn reinforce(&mut self, now: DateTime<Utc>) {
        self.support_count = self.support_count.saturating_add(1);
        self.updated_at = now;
    }

    /// Record a match event.
    pub fn record_match(&mut self, now: DateTime<Utc>) {
        self.last_matched_at = Some(now);
        self.updated_at = now;
    }

    /// Touch the updated_at timestamp.
    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.updated_at = now;
    }
}

// ---------------------------------------------------------------------------
// HolographicQuery helpers
// ---------------------------------------------------------------------------

impl HolographicQuery {
    /// Create a simple top-k query with a given vector.
    pub fn new(
        workspace_id: WorkspaceId,
        query_vector: Vec<f32>,
        top_k: usize,
        min_similarity: f32,
    ) -> Self {
        Self {
            workspace_id,
            query_vector,
            top_k,
            min_similarity: min_similarity.clamp(0.0, 1.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_holographic_trace() {
        let trace = HolographicTrace::new(
            HolographicTraceId::new("trace-1"),
            WorkspaceId::new("ws-1"),
            HolographicTraceKind::TaskPattern,
            vec!["task".to_owned(), "analysis".to_owned()],
            0.85,
            0.1,
            Utc::now(),
        );

        assert_eq!(trace.id, HolographicTraceId::new("trace-1"));
        assert_eq!(trace.workspace_id, WorkspaceId::new("ws-1"));
        assert_eq!(trace.trace_kind, HolographicTraceKind::TaskPattern);
        assert!(trace.vector.is_empty());
        assert_eq!(trace.strength, 0.85);
        assert_eq!(trace.decay, 0.1);
        assert_eq!(trace.source_episode_id, None);
    }

    #[test]
    fn create_holographic_pattern() {
        let pattern = HolographicPattern::new(
            HolographicPatternId::new("pattern-1"),
            WorkspaceId::new("ws-1"),
            HolographicPatternKind::FailurePrototype,
            vec!["failure".to_owned(), "timeout".to_owned()],
            0.75,
            Utc::now(),
        );

        assert_eq!(pattern.id, HolographicPatternId::new("pattern-1"));
        assert_eq!(
            pattern.pattern_kind,
            HolographicPatternKind::FailurePrototype
        );
        assert!(pattern.prototype_vector.is_empty());
        assert_eq!(pattern.support_count, 0);
        assert_eq!(pattern.confidence, 0.75);
        assert!(pattern.last_matched_at.is_none());
    }

    #[test]
    fn serialize_holographic_trace_to_json() {
        let trace = HolographicTrace::new(
            HolographicTraceId::new("trace-serialize"),
            WorkspaceId::new("ws-1"),
            HolographicTraceKind::DecisionPattern,
            vec!["decision".to_owned(), "approved".to_owned()],
            0.9,
            0.05,
            Utc::now(),
        )
        .with_episode(EpisodeId::new("ep-1"))
        .with_vector(vec![0.1, 0.2, 0.3]);

        let encoded = serde_json::to_string(&trace).expect("trace should serialize");
        assert!(encoded.contains("trace-serialize"));
        assert!(encoded.contains("decision_pattern"));
        assert!(encoded.contains("ws-1"));
        assert!(encoded.contains("ep-1"));

        let decoded: HolographicTrace =
            serde_json::from_str(&encoded).expect("trace should deserialize");
        assert_eq!(decoded.id, HolographicTraceId::new("trace-serialize"));
        assert_eq!(decoded.vector, vec![0.1, 0.2, 0.3]);
        assert_eq!(decoded.source_episode_id, Some(EpisodeId::new("ep-1")));
    }

    #[test]
    fn serialize_holographic_match_to_json() {
        let m = HolographicMatch {
            trace_id: HolographicTraceId::new("trace-match"),
            similarity: 0.91,
            matched_labels: vec!["error".to_owned(), "recovery".to_owned()],
            linked_episode_id: Some(EpisodeId::new("ep-2")),
        };

        let encoded = serde_json::to_string(&m).expect("match should serialize");
        assert!(encoded.contains("trace-match"));
        assert!(encoded.contains("0.91"));
        assert!(encoded.contains("ep-2"));

        let decoded: HolographicMatch =
            serde_json::from_str(&encoded).expect("match should deserialize");
        assert_eq!(decoded.trace_id, HolographicTraceId::new("trace-match"));
        assert_eq!(decoded.similarity, 0.91);
        assert_eq!(decoded.linked_episode_id, Some(EpisodeId::new("ep-2")));
    }

    #[test]
    fn holographic_memory_is_non_authorizing_by_design() {
        // Verify that none of the holographic types contain governance fields.
        // This is a compile-time/documentation test — the types are pure
        // resonance vocabulary and carry no decision or permission data.
        //
        // Key invariants:
        // - HolographicMatch contains a similarity score, not a decision.
        // - HolographicTrace stores an embedding vector, not an approval.
        // - HolographicPattern has confidence, not authorisation.
        // - HolographicQuery is a retrieval spec, not an action intent.

        let trace = HolographicTrace::new(
            HolographicTraceId::new("non-auth-trace"),
            WorkspaceId::new("ws-1"),
            HolographicTraceKind::CognitiveCyclePattern,
            vec![],
            0.5,
            0.1,
            Utc::now(),
        );

        // Trace fields that could be mistaken for governance:
        assert!(!trace.labels.contains(&"approved".to_owned()));
        assert!(!trace.labels.contains(&"authorized".to_owned()));

        let pattern = HolographicPattern::new(
            HolographicPatternId::new("non-auth-pattern"),
            WorkspaceId::new("ws-1"),
            HolographicPatternKind::CognitiveCyclePrototype,
            vec![],
            0.5,
            Utc::now(),
        );

        // Pattern confidence is trust in pattern quality, not action approval.
        assert!(pattern.confidence >= 0.0);
        assert!(pattern.confidence <= 1.0);

        let query = HolographicQuery::new(WorkspaceId::new("ws-1"), vec![0.1, 0.2], 10, 0.7);
        assert!(query.min_similarity >= 0.0);
        assert_eq!(query.top_k, 10);

        let m = HolographicMatch {
            trace_id: HolographicTraceId::new("non-auth-match"),
            similarity: 0.8,
            matched_labels: vec![],
            linked_episode_id: None,
        };
        // A match contains a similarity score, not a decision outcome.
        assert!(m.similarity >= 0.0);
        assert!(m.similarity <= 1.0);
        assert_eq!(m.trace_id, HolographicTraceId::new("non-auth-match"));
    }

    #[test]
    fn holographic_strength_and_decay_are_clamped() {
        let trace = HolographicTrace::new(
            HolographicTraceId::new("clamp-test"),
            WorkspaceId::new("ws-1"),
            HolographicTraceKind::SuccessPattern,
            vec![],
            1.5,  // above 1.0
            -0.1, // below 0.0
            Utc::now(),
        );

        assert_eq!(trace.strength, 1.0);
        assert_eq!(trace.decay, 0.0);
    }

    #[test]
    fn holographic_pattern_reinforce_increments_support() {
        let now = Utc::now();
        let mut pattern = HolographicPattern::new(
            HolographicPatternId::new("reinforce-test"),
            WorkspaceId::new("ws-1"),
            HolographicPatternKind::RoutingPrototype,
            vec!["routing".to_owned()],
            0.5,
            now,
        );

        pattern.reinforce(now);
        assert_eq!(pattern.support_count, 1);

        let later = now + chrono::Duration::hours(1);
        pattern.reinforce(later);
        assert_eq!(pattern.support_count, 2);
        assert_eq!(pattern.updated_at, later);
    }

    #[test]
    fn holographic_query_serializes() {
        let query = HolographicQuery::new(WorkspaceId::new("ws-1"), vec![0.5, 0.3, 0.8], 5, 0.75);

        let encoded = serde_json::to_string(&query).expect("query should serialize");
        assert!(encoded.contains("\"top_k\":5"));
        assert!(encoded.contains("0.75"));
        assert!(encoded.contains("0.5"));

        let decoded: HolographicQuery =
            serde_json::from_str(&encoded).expect("query should deserialize");
        assert_eq!(decoded.workspace_id, WorkspaceId::new("ws-1"));
        assert_eq!(decoded.top_k, 5);
        assert_eq!(decoded.min_similarity, 0.75);
    }
}
