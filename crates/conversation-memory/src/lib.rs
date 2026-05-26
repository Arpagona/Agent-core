//! Symbolic holographic conversation memory for ARPAGONA Agent Core.
//!
//! This crate provides a **symbolic** (non-vector, non-embedding) resonance memory
//! layer for conversation traces. It stores structured traces with keywords,
//! concepts, entities, decision links, and scoring metadata, and retrieves them
//! by resonance — partial keyword/concept/entity matching without LLM calls or
//! vector databases.
//!
//! # What "holographic" means here
//!
//! A hologram stores information about the whole in every part. Similarly, a
//! `HolographicTrace` in this crate stores a distributed signature of a
//! conversational moment: not just the raw text, but the *keywords*, *concepts*,
//! *entities*, and *decision links* that together can reconstruct context.
//! Querying by any fragment (a keyword, a concept, an entity) can resonate with
//! related traces — no exact match is required.
//!
//! # Difference from vector search
//!
//! Vector search (embeddings) would compute semantic similarity via neural
//! representations. This crate uses explicit symbolic matching:
//! - Keyword overlap
//! - Concept overlap
//! - Entity overlap
//! - Linked decision relevance
//! - Importance, confidence, recency, activation frequency
//!
//! This is simpler, fully deterministic, auditable, and does not require any
//! external model or database. Future work can add local embeddings as an
//! additional resonance signal.
//!
//! # Difference from raw memory / structured facts
//!
//! - `GraphMemoryStore` (structured facts): authoritative facts about entities.
//! - `HolographicPattern` (core crate, vector-oriented): future pattern detection.
//! - `HolographicTrace` (this crate, symbolic): conversation resonance traces.
//!
//! This layer does NOT store authoritative facts. It stores *resonance
//! signatures* — traces of what was discussed, decided, or observed. It never
//! invents content; it only links existing traces together.
//!
//! # Scope
//!
//! All operations are scoped by `project_id` (a string identifier). No trace
//! from project A can leak into a resonance query for project B.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A symbolic trace of a conversational moment.
///
/// Each trace stores a distributed signature: keywords, concepts, entities,
/// decision links, and metadata. This is NOT a raw transcript — it is a
/// curated set of signals that can later resonate with partial queries.
///
/// # Fields
///
/// * `id` — unique identifier for this trace.
/// * `project_id` — scope: all operations are scoped to a project.
/// * `source_memory_id` — optional reference to a source memory/fact.
/// * `source_turn_ids` — the conversation turn IDs that produced this trace.
/// * `content_summary` — a short human-readable summary (not raw text).
/// * `keywords` — explicit keywords for resonance matching.
/// * `entities` — named entities referenced (people, tools, documents, etc.).
/// * `concepts` — higher-level concepts this trace relates to.
/// * `linked_decision_ids` — decisions linked to this trace.
/// * `importance` — how important this trace is (0.0–1.0).
/// * `confidence` — how reliable/verified this trace is (0.0–1.0).
/// * `recency_score` — a pre-computed recency boost (0.0–1.0).
/// * `activation_count` — how many times this trace has been retrieved.
/// * `created_at` — when the trace was created.
/// * `last_activated_at` — when the trace was last retrieved/activated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HolographicTrace {
    pub id: String,
    pub project_id: String,
    pub source_memory_id: Option<String>,
    pub source_turn_ids: Vec<String>,
    pub content_summary: String,
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub concepts: Vec<String>,
    pub linked_decision_ids: Vec<String>,
    pub importance: f64,
    pub confidence: f64,
    pub recency_score: f64,
    pub activation_count: u64,
    pub created_at: DateTime<Utc>,
    pub last_activated_at: Option<DateTime<Utc>>,
}

/// A score produced during resonance matching.
///
/// Each contributing factor produces a `ResonanceScore` so the caller can
/// inspect *why* a trace matched.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResonanceScore {
    /// Short label for the scoring factor (e.g. "keyword_match", "concept_match").
    pub factor: String,
    /// The score contributed by this factor (0.0–1.0).
    pub score: f64,
    /// Human-readable explanation.
    pub rationale: String,
}

/// A single match result from a resonance query.
///
/// Contains the matched trace, the total resonance score, and a breakdown
/// of individual scores by factor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResonanceMatch {
    /// The matched trace.
    pub trace: HolographicTrace,
    /// Total resonance score (sum of all factor scores, capped at 1.0).
    pub total_score: f64,
    /// Breakdown of how the score was computed.
    pub score_factors: Vec<ResonanceScore>,
}

/// Reconstructed context from a set of resonance matches.
///
/// After retrieving matching traces, `ReconstructedContext` collates their
/// signals into a coherent summary: the concepts, entities, keywords, and
/// decisions that were active across the matched traces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReconstructedContext {
    /// The project this context belongs to.
    pub project_id: String,
    /// All concepts mentioned across matched traces, deduplicated.
    pub active_concepts: Vec<String>,
    /// All entities mentioned across matched traces, deduplicated.
    pub active_entities: Vec<String>,
    /// All keywords across matched traces, deduplicated.
    pub active_keywords: Vec<String>,
    /// All unique decision references across matched traces.
    pub linked_decision_ids: Vec<String>,
    /// The traces that contributed to this reconstruction.
    pub source_traces: Vec<HolographicTrace>,
    /// Total number of traces considered.
    pub trace_count: usize,
    /// Timestamp of the context reconstruction.
    pub reconstructed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// ConversationMemoryStore
// ---------------------------------------------------------------------------

/// In-memory store for `HolographicTrace` instances with resonance retrieval.
///
/// All operations are scoped by `project_id`. The store is purely symbolic:
/// no vector database, no LLM calls, no embeddings.
///
/// # Scoring rules
///
/// When a resonance query is performed, each trace is scored as follows:
///
/// 1. **Keyword match** (+0.25 per match, max +0.75): for each query keyword
///    found in the trace's keywords.
/// 2. **Concept match** (+0.20 per match, max +0.60): for each query concept
///    found in the trace's concepts.
/// 3. **Entity match** (+0.15 per match, max +0.45): for each query entity
///    found in the trace's entities.
/// 4. **Importance** (+0.10 × importance): base importance contribution.
/// 5. **Confidence** (+0.10 × confidence): base confidence contribution.
/// 6. **Recency** (+0.10 × recency_score): pre-computed recency boost.
/// 7. **Activation** (+0.05 × min(activation_count, 5) / 5): mild activation
///    bonus capped at 5 activations.
///
/// The total score is capped at 1.0. A score of 0.0 means no resonance.
/// Traces with total_score <= 0.0 are excluded from results.
#[derive(Clone, Debug, Default)]
pub struct ConversationMemoryStore {
    traces: HashMap<String, HolographicTrace>,
    next_id: u64,
}

impl ConversationMemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            traces: HashMap::new(),
            next_id: 1,
        }
    }

    /// Generate a unique trace ID.
    fn generate_id(&mut self) -> String {
        let id = format!("trace-{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a holographic trace to the store.
    ///
    /// If the trace has no `id`, one is auto-generated.
    /// Returns the ID assigned to the trace.
    pub fn add_holographic_trace(&mut self, mut trace: HolographicTrace) -> String {
        if trace.id.is_empty() {
            trace.id = self.generate_id();
        }
        let id = trace.id.clone();
        self.traces.insert(id.clone(), trace);
        id
    }

    /// List all holographic traces for a given project, ordered by creation
    /// time (newest first).
    pub fn list_holographic_traces(&self, project_id: &str) -> Vec<&HolographicTrace> {
        let mut result: Vec<&HolographicTrace> = self
            .traces
            .values()
            .filter(|t| t.project_id == project_id)
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result
    }

    /// Retrieve traces by symbolic resonance with the given query.
    ///
    /// The query can contain keywords, concepts, and entities. Traces are
    /// scored according to the rules in [`ConversationMemoryStore`].
    ///
    /// Returns up to `top_k` matches, sorted by descending score.
    pub fn retrieve_by_resonance(
        &self,
        project_id: &str,
        query: &ResonanceQuery,
        top_k: usize,
    ) -> Vec<ResonanceMatch> {
        let mut matches: Vec<ResonanceMatch> = self
            .traces
            .values()
            .filter(|t| t.project_id == project_id)
            .filter_map(|trace| {
                let (total_score, score_factors) = compute_resonance(trace, query);
                if total_score > 0.0 {
                    Some(ResonanceMatch {
                        trace: trace.clone(),
                        total_score: total_score.min(1.0),
                        score_factors,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by descending score
        matches.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches.truncate(top_k);
        matches
    }

    /// Activate a trace by its ID: increment activation_count and update
    /// last_activated_at.
    ///
    /// Returns `true` if the trace was found and activated, `false` otherwise.
    pub fn activate_trace(&mut self, trace_id: &str) -> bool {
        if let Some(trace) = self.traces.get_mut(trace_id) {
            trace.activation_count = trace.activation_count.saturating_add(1);
            trace.last_activated_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Reconstruct context from the top resonance matches for a query.
    ///
    /// Collates concepts, entities, keywords, and decision references from
    /// the matching traces into a single `ReconstructedContext`.
    pub fn reconstruct_context(
        &self,
        project_id: &str,
        query: &ResonanceQuery,
        top_k: usize,
    ) -> ReconstructedContext {
        let matches = self.retrieve_by_resonance(project_id, query, top_k);
        let source_traces: Vec<HolographicTrace> = matches.into_iter().map(|m| m.trace).collect();

        let mut active_concepts: Vec<String> = Vec::new();
        let mut active_entities: Vec<String> = Vec::new();
        let mut active_keywords: Vec<String> = Vec::new();
        let mut linked_decision_ids: Vec<String> = Vec::new();

        for trace in &source_traces {
            for concept in &trace.concepts {
                if !active_concepts.contains(concept) {
                    active_concepts.push(concept.clone());
                }
            }
            for entity in &trace.entities {
                if !active_entities.contains(entity) {
                    active_entities.push(entity.clone());
                }
            }
            for kw in &trace.keywords {
                if !active_keywords.contains(kw) {
                    active_keywords.push(kw.clone());
                }
            }
            for did in &trace.linked_decision_ids {
                if !linked_decision_ids.contains(did) {
                    linked_decision_ids.push(did.clone());
                }
            }
        }

        ReconstructedContext {
            project_id: project_id.to_owned(),
            active_concepts,
            active_entities,
            active_keywords,
            linked_decision_ids,
            trace_count: source_traces.len(),
            source_traces,
            reconstructed_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// Query type
// ---------------------------------------------------------------------------

/// A query for resonance retrieval.
///
/// Contains symbolic signals (keywords, concepts, entities) to match against
/// stored `HolographicTrace` instances.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResonanceQuery {
    /// Keywords to match against trace keywords.
    pub keywords: Vec<String>,
    /// Concepts to match against trace concepts.
    pub concepts: Vec<String>,
    /// Entities to match against trace entities.
    pub entities: Vec<String>,
    /// Optional minimum confidence threshold.
    pub min_confidence: Option<f64>,
    /// Optional minimum importance threshold.
    pub min_importance: Option<f64>,
}

impl ResonanceQuery {
    /// Create a new resonance query with the given keywords, concepts, and entities.
    pub fn new(keywords: Vec<String>, concepts: Vec<String>, entities: Vec<String>) -> Self {
        Self {
            keywords,
            concepts,
            entities,
            min_confidence: None,
            min_importance: None,
        }
    }

    /// Set a minimum confidence threshold.
    pub fn with_min_confidence(mut self, threshold: f64) -> Self {
        self.min_confidence = Some(threshold.clamp(0.0, 1.0));
        self
    }

    /// Set a minimum importance threshold.
    pub fn with_min_importance(mut self, threshold: f64) -> Self {
        self.min_importance = Some(threshold.clamp(0.0, 1.0));
        self
    }
}

// ---------------------------------------------------------------------------
// Resonance scoring
// ---------------------------------------------------------------------------

/// Compute the resonance score for a single trace against a query.
///
/// Returns `(total_score, score_factors)`.
fn compute_resonance(
    trace: &HolographicTrace,
    query: &ResonanceQuery,
) -> (f64, Vec<ResonanceScore>) {
    let mut scores: Vec<ResonanceScore> = Vec::new();
    let mut total: f64 = 0.0;

    // Apply threshold filters first
    if let Some(min_conf) = query.min_confidence {
        if trace.confidence < min_conf {
            return (0.0, scores);
        }
    }
    if let Some(min_imp) = query.min_importance {
        if trace.importance < min_imp {
            return (0.0, scores);
        }
    }

    // Track whether we had any symbolic match (keyword, concept, or entity)
    let mut has_symbolic_match = false;

    // 1. Keyword match
    if !query.keywords.is_empty() {
        let trace_kw_lower: Vec<String> = trace.keywords.iter().map(|k| k.to_lowercase()).collect();
        let matched_keywords: Vec<&String> = query
            .keywords
            .iter()
            .filter(|qk| trace_kw_lower.contains(&qk.to_lowercase()))
            .collect();
        let kw_count = matched_keywords.len();
        let keyword_score = (kw_count as f64 * 0.25).min(0.75);
        if keyword_score > 0.0 {
            has_symbolic_match = true;
            scores.push(ResonanceScore {
                factor: "keyword_match".to_owned(),
                score: keyword_score,
                rationale: format!(
                    "matched {} keyword(s): {:?}",
                    kw_count,
                    matched_keywords
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                ),
            });
            total += keyword_score;
        }
    }

    // 2. Concept match
    if !query.concepts.is_empty() {
        let trace_conc_lower: Vec<String> =
            trace.concepts.iter().map(|c| c.to_lowercase()).collect();
        let matched_concepts: Vec<&String> = query
            .concepts
            .iter()
            .filter(|qc| trace_conc_lower.contains(&qc.to_lowercase()))
            .collect();
        let concept_count = matched_concepts.len();
        let concept_score = (concept_count as f64 * 0.20).min(0.60);
        if concept_score > 0.0 {
            has_symbolic_match = true;
            scores.push(ResonanceScore {
                factor: "concept_match".to_owned(),
                score: concept_score,
                rationale: format!(
                    "matched {} concept(s): {:?}",
                    concept_count,
                    matched_concepts
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                ),
            });
            total += concept_score;
        }
    }

    // 3. Entity match
    if !query.entities.is_empty() {
        let trace_ent_lower: Vec<String> =
            trace.entities.iter().map(|e| e.to_lowercase()).collect();
        let matched_entities: Vec<&String> = query
            .entities
            .iter()
            .filter(|qe| trace_ent_lower.contains(&qe.to_lowercase()))
            .collect();
        let entity_count = matched_entities.len();
        let entity_score = (entity_count as f64 * 0.15).min(0.45);
        if entity_score > 0.0 {
            has_symbolic_match = true;
            scores.push(ResonanceScore {
                factor: "entity_match".to_owned(),
                score: entity_score,
                rationale: format!(
                    "matched {} entity(ies): {:?}",
                    entity_count,
                    matched_entities
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                ),
            });
            total += entity_score;
        }
    }

    // If no symbolic match and query has signals, return 0
    // This ensures empty queries don't hallucinate, and wrong-keyword queries don't match
    if !query.keywords.is_empty() || !query.concepts.is_empty() || !query.entities.is_empty() {
        if !has_symbolic_match {
            return (0.0, scores);
        }
    } else {
        // Empty query: no symbolic signals → no results
        return (0.0, scores);
    }

    // 4. Importance
    let importance_score = trace.importance * 0.10;
    scores.push(ResonanceScore {
        factor: "importance".to_owned(),
        score: importance_score,
        rationale: format!("trace importance is {:.2}", trace.importance),
    });
    total += importance_score;

    // 5. Confidence
    let confidence_score = trace.confidence * 0.10;
    scores.push(ResonanceScore {
        factor: "confidence".to_owned(),
        score: confidence_score,
        rationale: format!("trace confidence is {:.2}", trace.confidence),
    });
    total += confidence_score;

    // 6. Recency
    let recency_score = trace.recency_score * 0.10;
    scores.push(ResonanceScore {
        factor: "recency".to_owned(),
        score: recency_score,
        rationale: format!("recency score is {:.2}", trace.recency_score),
    });
    total += recency_score;

    // 7. Activation bonus (capped at 5)
    let activation_bonus = (trace.activation_count.min(5) as f64 / 5.0) * 0.05;
    scores.push(ResonanceScore {
        factor: "activation".to_owned(),
        score: activation_bonus,
        rationale: format!("activated {} time(s) (capped at 5)", trace.activation_count),
    });
    total += activation_bonus;

    (total, scores)
}

// ---------------------------------------------------------------------------
// Trace builder
// ---------------------------------------------------------------------------

/// Builder for `HolographicTrace`.
///
/// Makes it easy to create traces for testing and production use.
#[derive(Clone, Debug, Default)]
pub struct HolographicTraceBuilder {
    id: Option<String>,
    project_id: Option<String>,
    source_memory_id: Option<String>,
    source_turn_ids: Vec<String>,
    content_summary: String,
    keywords: Vec<String>,
    entities: Vec<String>,
    concepts: Vec<String>,
    linked_decision_ids: Vec<String>,
    importance: f64,
    confidence: f64,
    recency_score: f64,
}

impl HolographicTraceBuilder {
    /// Create a new builder with the given project_id and content_summary.
    pub fn new(project_id: &str, content_summary: &str) -> Self {
        Self {
            project_id: Some(project_id.to_owned()),
            content_summary: content_summary.to_owned(),
            ..Default::default()
        }
    }

    /// Set the trace ID. If not set, one will be auto-generated.
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_owned());
        self
    }

    /// Set the source memory ID.
    pub fn with_source_memory_id(mut self, id: &str) -> Self {
        self.source_memory_id = Some(id.to_owned());
        self
    }

    /// Add source turn IDs.
    pub fn with_source_turn_ids(mut self, ids: Vec<String>) -> Self {
        self.source_turn_ids = ids;
        self
    }

    /// Set keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Set entities.
    pub fn with_entities(mut self, entities: Vec<String>) -> Self {
        self.entities = entities;
        self
    }

    /// Set concepts.
    pub fn with_concepts(mut self, concepts: Vec<String>) -> Self {
        self.concepts = concepts;
        self
    }

    /// Set linked decision IDs.
    pub fn with_linked_decision_ids(mut self, ids: Vec<String>) -> Self {
        self.linked_decision_ids = ids;
        self
    }

    /// Set importance (0.0–1.0).
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set confidence (0.0–1.0).
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set recency score (0.0–1.0).
    pub fn with_recency_score(mut self, score: f64) -> Self {
        self.recency_score = score.clamp(0.0, 1.0);
        self
    }

    /// Build the trace.
    pub fn build(self) -> HolographicTrace {
        let now = Utc::now();
        HolographicTrace {
            id: self.id.unwrap_or_default(),
            project_id: self
                .project_id
                .expect("HolographicTraceBuilder: project_id is required"),
            source_memory_id: self.source_memory_id,
            source_turn_ids: self.source_turn_ids,
            content_summary: self.content_summary,
            keywords: self.keywords,
            entities: self.entities,
            concepts: self.concepts,
            linked_decision_ids: self.linked_decision_ids,
            importance: self.importance.clamp(0.0, 1.0),
            confidence: self.confidence.clamp(0.0, 1.0),
            recency_score: self.recency_score.clamp(0.0, 1.0),
            activation_count: 0,
            created_at: now,
            last_activated_at: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_trace(
        project_id: &str,
        summary: &str,
        keywords: Vec<String>,
        concepts: Vec<String>,
        entities: Vec<String>,
    ) -> HolographicTrace {
        HolographicTraceBuilder::new(project_id, summary)
            .with_keywords(keywords)
            .with_concepts(concepts)
            .with_entities(entities)
            .with_importance(0.7)
            .with_confidence(0.8)
            .with_recency_score(0.5)
            .with_source_turn_ids(vec!["turn-1".to_owned(), "turn-2".to_owned()])
            .build()
    }

    // 1. trace can be added and listed by project_id
    #[test]
    fn test_add_and_list_by_project() {
        let mut store = ConversationMemoryStore::new();

        let t1 = create_test_trace(
            "proj-alpha",
            "discussed memory model",
            vec!["memory".to_owned(), "model".to_owned()],
            vec![],
            vec![],
        );
        let t2 = create_test_trace(
            "proj-beta",
            "discussed deployment",
            vec!["deploy".to_owned()],
            vec![],
            vec![],
        );
        let t3 = create_test_trace(
            "proj-alpha",
            "reviewed performance",
            vec!["performance".to_owned()],
            vec![],
            vec![],
        );

        store.add_holographic_trace(t1);
        store.add_holographic_trace(t2);
        store.add_holographic_trace(t3);

        let alpha_traces = store.list_holographic_traces("proj-alpha");
        assert_eq!(alpha_traces.len(), 2, "should have 2 traces for proj-alpha");

        let beta_traces = store.list_holographic_traces("proj-beta");
        assert_eq!(beta_traces.len(), 1, "should have 1 trace for proj-beta");

        // Newest first
        assert_eq!(alpha_traces[0].content_summary, "reviewed performance");
        assert_eq!(alpha_traces[1].content_summary, "discussed memory model");
    }

    // 2. resonance retrieval matches keyword
    #[test]
    fn test_resonance_keyword_match() {
        let mut store = ConversationMemoryStore::new();

        store.add_holographic_trace(create_test_trace(
            "proj-x",
            "discussed async execution",
            vec!["async".to_owned(), "execution".to_owned()],
            vec![],
            vec![],
        ));
        store.add_holographic_trace(create_test_trace(
            "proj-x",
            "discussed UI layout",
            vec!["ui".to_owned(), "layout".to_owned()],
            vec![],
            vec![],
        ));

        let query = ResonanceQuery::new(vec!["async".to_owned()], vec![], vec![]);
        let results = store.retrieve_by_resonance("proj-x", &query, 10);

        assert_eq!(results.len(), 1, "keyword 'async' should match 1 trace");
        assert!(results[0].trace.content_summary.contains("async"));
        assert!(results[0].total_score > 0.0);
    }

    // 3. resonance retrieval matches concept even if exact text differs
    #[test]
    fn test_resonance_concept_match() {
        let mut store = ConversationMemoryStore::new();

        // Trace about "parallel computing" with concept "concurrency"
        store.add_holographic_trace(create_test_trace(
            "proj-x",
            "discussed parallel processing",
            vec!["parallel".to_owned()],
            vec!["concurrency".to_owned()],
            vec![],
        ));

        // Trace about UI (no matching concept)
        store.add_holographic_trace(create_test_trace(
            "proj-x",
            "discussed design",
            vec!["design".to_owned()],
            vec!["ux".to_owned()],
            vec![],
        ));

        // Query by concept "concurrency"
        let query = ResonanceQuery::new(vec![], vec!["concurrency".to_owned()], vec![]);
        let results = store.retrieve_by_resonance("proj-x", &query, 10);

        assert_eq!(
            results.len(),
            1,
            "concept 'concurrency' should match the parallel trace"
        );
        assert!(results[0].trace.content_summary.contains("parallel"));
    }

    // 4. retrieval does not leak traces from another project
    #[test]
    fn test_no_project_leak() {
        let mut store = ConversationMemoryStore::new();

        store.add_holographic_trace(create_test_trace(
            "proj-secure",
            "top secret plan",
            vec!["secret".to_owned()],
            vec!["security".to_owned()],
            vec!["vault".to_owned()],
        ));
        store.add_holographic_trace(create_test_trace(
            "proj-public",
            "public roadmap",
            vec!["roadmap".to_owned()],
            vec!["planning".to_owned()],
            vec!["users".to_owned()],
        ));

        // Query proj-public with "secret" keyword — should NOT match proj-secure's trace
        let query = ResonanceQuery::new(vec!["secret".to_owned()], vec![], vec![]);
        let results = store.retrieve_by_resonance("proj-public", &query, 10);
        assert_eq!(
            results.len(),
            0,
            "should not leak secure trace into public project"
        );

        // Query proj-secure with "secret"
        let query2 = ResonanceQuery::new(vec!["secret".to_owned()], vec![], vec![]);
        let results2 = store.retrieve_by_resonance("proj-secure", &query2, 10);
        assert_eq!(
            results2.len(),
            1,
            "should find secret trace in secure project"
        );
    }

    // 5. activation_count increases after retrieval or activate_trace
    #[test]
    fn test_activation_count_increments() {
        let mut store = ConversationMemoryStore::new();

        let trace = create_test_trace(
            "proj-x",
            "important discussion",
            vec!["important".to_owned()],
            vec![],
            vec![],
        );
        let id = store.add_holographic_trace(trace);

        // Initial activation count should be 0
        let traces = store.list_holographic_traces("proj-x");
        assert_eq!(traces[0].activation_count, 0);

        // Activate twice
        assert!(store.activate_trace(&id));
        assert!(store.activate_trace(&id));

        let traces = store.list_holographic_traces("proj-x");
        assert_eq!(traces[0].activation_count, 2);

        // Activate unknown ID returns false
        assert!(!store.activate_trace("nonexistent"));
    }

    // 6. linked decisions are returned with reconstructed context
    #[test]
    fn test_linked_decisions_in_reconstructed_context() {
        let mut store = ConversationMemoryStore::new();

        let trace = HolographicTraceBuilder::new("proj-x", "decided on architecture")
            .with_keywords(vec!["architecture".to_owned()])
            .with_concepts(vec!["design".to_owned()])
            .with_entities(vec!["rust".to_owned()])
            .with_linked_decision_ids(vec!["decision-1".to_owned(), "decision-2".to_owned()])
            .with_importance(0.9)
            .with_confidence(0.9)
            .build();
        store.add_holographic_trace(trace);

        let query = ResonanceQuery::new(vec!["architecture".to_owned()], vec![], vec![]);
        let context = store.reconstruct_context("proj-x", &query, 10);

        assert!(context
            .linked_decision_ids
            .contains(&"decision-1".to_owned()));
        assert!(context
            .linked_decision_ids
            .contains(&"decision-2".to_owned()));
        assert!(context.active_concepts.contains(&"design".to_owned()));
        assert!(context.active_entities.contains(&"rust".to_owned()));
        assert!(context.active_keywords.contains(&"architecture".to_owned()));
        assert_eq!(context.trace_count, 1);
    }

    // 7. low confidence traces score lower than high confidence traces
    #[test]
    fn test_confidence_affects_score() {
        let mut store = ConversationMemoryStore::new();

        let high_conf = HolographicTraceBuilder::new("proj-x", "well-known fact")
            .with_keywords(vec!["fact".to_owned()])
            .with_confidence(0.95)
            .with_importance(0.5)
            .with_recency_score(0.5)
            .build();
        let low_conf = HolographicTraceBuilder::new("proj-x", "unverified rumor")
            .with_keywords(vec!["fact".to_owned()])
            .with_confidence(0.15)
            .with_importance(0.5)
            .with_recency_score(0.5)
            .build();

        let id_high = store.add_holographic_trace(high_conf);
        let _id_low = store.add_holographic_trace(low_conf);

        let query = ResonanceQuery::new(vec!["fact".to_owned()], vec![], vec![]);
        let results = store.retrieve_by_resonance("proj-x", &query, 10);

        assert_eq!(results.len(), 2, "both traces should match");
        // High confidence should score higher (all other factors are equal)
        let high_idx = if results[0].trace.id == id_high { 0 } else { 1 };
        let low_idx = if high_idx == 0 { 1 } else { 0 };
        assert!(
            results[high_idx].total_score > results[low_idx].total_score,
            "high confidence trace should score higher than low confidence"
        );
    }

    // 8. source_turn_ids are preserved
    #[test]
    fn test_source_turn_ids_preserved() {
        let mut store = ConversationMemoryStore::new();

        let trace = HolographicTraceBuilder::new("proj-x", "discussed deployment")
            .with_source_turn_ids(vec![
                "turn-5".to_owned(),
                "turn-6".to_owned(),
                "turn-7".to_owned(),
            ])
            .with_keywords(vec!["deploy".to_owned()])
            .build();

        let id = store.add_holographic_trace(trace);

        let traces = store.list_holographic_traces("proj-x");
        let saved = traces
            .iter()
            .find(|t| t.id == id)
            .expect("trace should exist");
        assert_eq!(saved.source_turn_ids, vec!["turn-5", "turn-6", "turn-7"]);
    }

    // 9. empty query returns no arbitrary hallucinated context
    #[test]
    fn test_empty_query_no_hallucination() {
        let mut store = ConversationMemoryStore::new();

        store.add_holographic_trace(create_test_trace(
            "proj-x",
            "some discussion",
            vec!["key".to_owned()],
            vec!["concept".to_owned()],
            vec!["entity".to_owned()],
        ));

        // Empty query — no keywords, no concepts, no entities
        let query = ResonanceQuery::default();
        let results = store.retrieve_by_resonance("proj-x", &query, 10);

        assert_eq!(results.len(), 0, "empty query should return no results");
    }

    // ── Additional edge case tests ─────────────────────────────────────────

    #[test]
    fn test_auto_generated_id() {
        let mut store = ConversationMemoryStore::new();

        let trace = create_test_trace("p", "auto-id", vec![], vec![], vec![]);
        let id = store.add_holographic_trace(trace);
        assert!(
            id.starts_with("trace-"),
            "auto-generated ID should start with 'trace-'"
        );
    }

    #[test]
    fn test_threshold_filters_exclude_low_confidence() {
        let mut store = ConversationMemoryStore::new();

        let trace = HolographicTraceBuilder::new("p", "low confidence fact")
            .with_keywords(vec!["fact".to_owned()])
            .with_confidence(0.3)
            .build();
        store.add_holographic_trace(trace);

        let query =
            ResonanceQuery::new(vec!["fact".to_owned()], vec![], vec![]).with_min_confidence(0.5);
        let results = store.retrieve_by_resonance("p", &query, 10);
        assert_eq!(
            results.len(),
            0,
            "should exclude traces below min_confidence"
        );
    }

    #[test]
    fn test_threshold_filters_exclude_low_importance() {
        let mut store = ConversationMemoryStore::new();

        let trace = HolographicTraceBuilder::new("p", "low importance note")
            .with_keywords(vec!["note".to_owned()])
            .with_importance(0.2)
            .build();
        store.add_holographic_trace(trace);

        let query =
            ResonanceQuery::new(vec!["note".to_owned()], vec![], vec![]).with_min_importance(0.5);
        let results = store.retrieve_by_resonance("p", &query, 10);
        assert_eq!(
            results.len(),
            0,
            "should exclude traces below min_importance"
        );
    }

    #[test]
    fn test_top_k_limits_results() {
        let mut store = ConversationMemoryStore::new();

        for i in 0..5 {
            let trace = HolographicTraceBuilder::new("p", &format!("trace {}", i))
                .with_keywords(vec!["common".to_owned()])
                .with_importance(0.5)
                .with_confidence(0.5)
                .with_recency_score(0.5)
                .build();
            store.add_holographic_trace(trace);
        }

        let query = ResonanceQuery::new(vec!["common".to_owned()], vec![], vec![]);
        let results = store.retrieve_by_resonance("p", &query, 3);
        assert_eq!(results.len(), 3, "top_k=3 should return 3 results max");

        // With top_k=10, should return all 5
        let results_all = store.retrieve_by_resonance("p", &query, 10);
        assert_eq!(results_all.len(), 5);
    }

    #[test]
    fn test_reconstruct_context_empty_when_no_match() {
        let store = ConversationMemoryStore::new();
        let query = ResonanceQuery::new(vec!["nonexistent".to_owned()], vec![], vec![]);
        let context = store.reconstruct_context("p", &query, 10);
        assert_eq!(context.trace_count, 0);
        assert!(context.active_concepts.is_empty());
        assert!(context.active_entities.is_empty());
        assert!(context.source_traces.is_empty());
    }
}
