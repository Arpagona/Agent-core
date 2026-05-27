//! # Holographic Memory — symbolic associative memory core
//!
//! A deterministic, traceable, distributed-signature associative memory for
//! ARPAGONA Agent Core. No LLM calls, no embeddings, no vector database.
//!
//! ## Core concept
//!
//! Each memory trace is encoded into a **distributed signature** — a set of
//! deterministic u64 bit-positions derived from its keywords, concepts,
//! entities and linked decision IDs. Retrieval works by **resonance**:
//! a query is encoded the same way, and the overlap between query bits and
//! trace bits produces a `ResonanceScore`. Traces above threshold are
//! returned with their linked context.
//!
//! ## Properties
//!
//! - **Deterministic**: same input → same signature → same retrieval.
//! - **Project-scoped**: no memory leaks between `project_id` values.
//! - **Traceable**: every trace preserves its `source_turn_ids`.
//! - **Reconstructive**: the `ReconstructedContext` collects linked
//!   memories and decisions from matched traces without inventing data.
//! - **Non-authoritative**: the store does not authorize actions. It only
//!   provides pattern resonance.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during holographic memory operations.
#[derive(Clone, Debug, PartialEq)]
pub enum HolographicMemoryError {
    /// A trace with this ID already exists in the store.
    TraceAlreadyExists(String),
    /// No trace found with the given ID.
    TraceNotFound(String),
    /// The provided signature is empty (all bit vectors are empty).
    EmptySignature,
    /// Invalid threshold value (must be in 0.0–1.0 range).
    InvalidThreshold(f32),
    /// I/O or serialization error during persistence operations.
    PersistenceError(String),
    /// Generic internal error.
    Internal(String),
}

impl std::fmt::Display for HolographicMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HolographicMemoryError::TraceAlreadyExists(id) => {
                write!(f, "Trace already exists: {}", id)
            }
            HolographicMemoryError::TraceNotFound(id) => {
                write!(f, "Trace not found: {}", id)
            }
            HolographicMemoryError::EmptySignature => {
                write!(f, "Signature is empty (all bit vectors are empty)")
            }
            HolographicMemoryError::InvalidThreshold(t) => {
                write!(f, "Invalid threshold {}: must be in 0.0–1.0", t)
            }
            HolographicMemoryError::PersistenceError(msg) => {
                write!(f, "Persistence error: {}", msg)
            }
            HolographicMemoryError::Internal(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

impl std::error::Error for HolographicMemoryError {}

// ---------------------------------------------------------------------------
// SourceKind
// ---------------------------------------------------------------------------

/// The kind of source that produced this memory trace.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// A turn in a conversation.
    ConversationTurn,
    /// A candidate memory proposed by the system.
    MemoryCandidate,
    /// An architecture decision record.
    ArchitectureDecision,
    /// An audit event.
    AuditEvent,
    /// A manually written note.
    ManualNote,
}

impl SourceKind {
    /// Return all variants for iteration.
    pub fn all() -> Vec<SourceKind> {
        vec![
            SourceKind::ConversationTurn,
            SourceKind::MemoryCandidate,
            SourceKind::ArchitectureDecision,
            SourceKind::AuditEvent,
            SourceKind::ManualNote,
        ]
    }
}

// ---------------------------------------------------------------------------
// DistributedSignature
// ---------------------------------------------------------------------------

/// A distributed symbolic signature encoding the semantic content of a trace.
///
/// Each field holds deterministic u64 bit-positions derived from the
/// corresponding set of terms. The signature is used for resonance-based
/// retrieval: two signatures that share many bits are considered similar.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DistributedSignature {
    /// Bit-positions derived from keywords (symbolic labels).
    pub symbolic_bits: Vec<u64>,
    /// Bit-positions derived from concepts (semantic categories).
    pub concept_bits: Vec<u64>,
    /// Bit-positions derived from entities (named things).
    pub entity_bits: Vec<u64>,
    /// Bit-positions derived from linked decision IDs.
    pub decision_bits: Vec<u64>,
}

impl DistributedSignature {
    /// Create a new empty signature.
    pub fn empty() -> Self {
        Self {
            symbolic_bits: vec![],
            concept_bits: vec![],
            entity_bits: vec![],
            decision_bits: vec![],
        }
    }

    /// Returns true if all bit vectors are empty.
    pub fn is_empty(&self) -> bool {
        self.symbolic_bits.is_empty()
            && self.concept_bits.is_empty()
            && self.entity_bits.is_empty()
            && self.decision_bits.is_empty()
    }
}

// ---------------------------------------------------------------------------
// HolographicTrace
// ---------------------------------------------------------------------------

/// A single holographic memory trace.
///
/// Captures a cognitive experience with its distributed signature, metadata,
/// linked context, and activation history.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HolographicTrace {
    /// Unique identifier for this trace.
    pub id: String,
    /// Project scope — all queries and retrievals are scoped by project.
    pub project_id: String,
    /// The kind of source that produced this trace.
    pub source_kind: SourceKind,
    /// Identifier of the source (e.g. conversation ID, decision ID).
    pub source_id: String,
    /// Turn IDs within the source that contributed to this trace.
    pub source_turn_ids: Vec<String>,
    /// Human-readable summary of the trace content.
    pub content_summary: String,
    /// Keywords associated with this trace.
    pub keywords: Vec<String>,
    /// Semantic concepts associated with this trace.
    pub concepts: Vec<String>,
    /// Named entities referenced by this trace.
    pub entities: Vec<String>,
    /// IDs of other memory traces linked to this one.
    pub linked_memory_ids: Vec<String>,
    /// IDs of decisions linked to this trace.
    pub linked_decision_ids: Vec<String>,
    /// How important this trace is (0.0–1.0).
    pub importance: f32,
    /// How confident the system is in this trace (0.0–1.0).
    pub confidence: f32,
    /// Emotional weight associated with this trace (0.0–1.0).
    pub emotional_weight: f32,
    /// Strategic weight — relevance to long-term goals (0.0–1.0).
    pub strategic_weight: f32,
    /// How many times this trace has been activated (retrieved).
    pub activation_count: u64,
    /// When this trace was created (ISO 8601).
    pub created_at: String,
    /// When this trace was last activated (ISO 8601), if ever.
    pub last_activated_at: Option<String>,
    /// The distributed signature encoding this trace's content.
    pub distributed_signature: DistributedSignature,
}

impl HolographicTrace {
    /// Create a new trace, automatically encoding terms into a signature.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        project_id: String,
        source_kind: SourceKind,
        source_id: String,
        source_turn_ids: Vec<String>,
        content_summary: String,
        keywords: Vec<String>,
        concepts: Vec<String>,
        entities: Vec<String>,
        linked_memory_ids: Vec<String>,
        linked_decision_ids: Vec<String>,
        importance: f32,
        confidence: f32,
        emotional_weight: f32,
        strategic_weight: f32,
        created_at: String,
    ) -> Self {
        let normalized_keywords = normalize_terms(&keywords);
        let normalized_concepts = normalize_terms(&concepts);
        let normalized_entities = normalize_terms(&entities);
        let normalized_decisions = normalize_terms(&linked_decision_ids);

        let distributed_signature = encode_terms_to_signature(
            &normalized_keywords,
            &normalized_concepts,
            &normalized_entities,
            &normalized_decisions,
        );

        Self {
            id,
            project_id,
            source_kind,
            source_id,
            source_turn_ids,
            content_summary,
            keywords: normalized_keywords,
            concepts: normalized_concepts,
            entities: normalized_entities,
            linked_memory_ids,
            linked_decision_ids,
            importance: importance.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            emotional_weight: emotional_weight.clamp(0.0, 1.0),
            strategic_weight: strategic_weight.clamp(0.0, 1.0),
            activation_count: 0,
            created_at,
            last_activated_at: None,
            distributed_signature,
        }
    }
}

// ---------------------------------------------------------------------------
// HolographicQuery
// ---------------------------------------------------------------------------

/// A query to find traces by resonance.
///
/// The query carries its own signature, which is compared against trace
/// signatures during retrieval.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HolographicQuery {
    /// Project scope — only traces with this project_id are considered.
    pub project_id: String,
    /// Free-text description of the query (for context/display only).
    pub text: String,
    /// Keywords to match against trace keywords.
    pub keywords: Vec<String>,
    /// Concepts to match against trace concepts.
    pub concepts: Vec<String>,
    /// Entities to match against trace entities.
    pub entities: Vec<String>,
    /// Pre-computed distributed signature for this query.
    pub distributed_signature: DistributedSignature,
}

impl HolographicQuery {
    /// Create a new query, automatically encoding terms into a signature.
    pub fn new(
        project_id: String,
        text: String,
        keywords: Vec<String>,
        concepts: Vec<String>,
        entities: Vec<String>,
    ) -> Self {
        let normalized_keywords = normalize_terms(&keywords);
        let normalized_concepts = normalize_terms(&concepts);
        let normalized_entities = normalize_terms(&entities);

        let distributed_signature = encode_terms_to_signature(
            &normalized_keywords,
            &normalized_concepts,
            &normalized_entities,
            &[], // decision_ids not used in query encoding for retrieval
        );

        Self {
            project_id,
            text,
            keywords: normalized_keywords,
            concepts: normalized_concepts,
            entities: normalized_entities,
            distributed_signature,
        }
    }

    /// Returns true if the query has no keywords, concepts, or entities.
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty() && self.concepts.is_empty() && self.entities.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ResonanceScore
// ---------------------------------------------------------------------------

/// A multi-dimensional resonance score between a query and a trace.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResonanceScore {
    /// Total combined resonance score.
    pub total: f32,
    /// Overlap of symbolic (keyword) bits.
    pub symbolic_overlap: f32,
    /// Overlap of concept bits.
    pub concept_overlap: f32,
    /// Overlap of entity bits.
    pub entity_overlap: f32,
    /// Overlap of decision bits.
    pub decision_overlap: f32,
    /// Boost from trace importance.
    pub importance_boost: f32,
    /// Boost from trace confidence.
    pub confidence_boost: f32,
    /// Boost from trace activation count (recency/frequency).
    pub activation_boost: f32,
}

impl ResonanceScore {
    /// Create a zero score.
    pub fn zero() -> Self {
        Self {
            total: 0.0,
            symbolic_overlap: 0.0,
            concept_overlap: 0.0,
            entity_overlap: 0.0,
            decision_overlap: 0.0,
            importance_boost: 0.0,
            confidence_boost: 0.0,
            activation_boost: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// ResonanceMatch
// ---------------------------------------------------------------------------

/// A single match result from resonance retrieval.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResonanceMatch {
    /// The trace that matched.
    pub trace: HolographicTrace,
    /// The resonance score for this match.
    pub score: ResonanceScore,
    /// Which keywords from the query matched the trace's keywords.
    pub matched_keywords: Vec<String>,
    /// Which concepts from the query matched the trace's concepts.
    pub matched_concepts: Vec<String>,
    /// Which entities from the query matched the trace's entities.
    pub matched_entities: Vec<String>,
}

// ---------------------------------------------------------------------------
// ReconstructedContext
// ---------------------------------------------------------------------------

/// The result of a resonance retrieval, with linked context expansion.
///
/// This is a **reconstructed** context: it is built entirely from existing
/// trace data. No data is invented. The reconstruction is deterministic.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReconstructedContext {
    /// Project scope.
    pub project_id: String,
    /// The original query text.
    pub query: String,
    /// Resonance matches (sorted by score descending).
    pub matches: Vec<ResonanceMatch>,
    /// IDs of all traces activated during this retrieval.
    pub activated_trace_ids: Vec<String>,
    /// All linked memory IDs collected from matched traces (deduplicated).
    pub linked_memory_ids: Vec<String>,
    /// All linked decision IDs collected from matched traces (deduplicated).
    pub linked_decision_ids: Vec<String>,
    /// Deterministic reconstruction summary (no LLM).
    pub reconstruction_summary: String,
}

impl ReconstructedContext {
    /// Create an empty context for a given project and query.
    pub fn empty(project_id: String, query: String) -> Self {
        Self {
            project_id,
            query,
            matches: vec![],
            activated_trace_ids: vec![],
            linked_memory_ids: vec![],
            linked_decision_ids: vec![],
            reconstruction_summary: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryGraphTraversalResult
// ---------------------------------------------------------------------------

/// The result of a recursive linked-memory graph traversal.
///
/// Produced by `HolographicMemoryStore::traverse_linked_memories`, this
/// describes all traces reachable from a root trace by following
/// `linked_memory_ids` chains up to a configurable depth.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryGraphTraversalResult {
    /// The trace ID that was the root of the traversal.
    pub root_trace_id: String,
    /// All traces visited during the traversal (in order of discovery).
    pub visited_traces: Vec<HolographicTrace>,
    /// The IDs of all visited traces (in order of discovery).
    pub visited_trace_ids: Vec<String>,
    /// The maximum depth reached during traversal (0 = only the root).
    pub reachable_depth: usize,
    /// The depth limit that was configured for this traversal.
    pub max_depth_limit: usize,
    /// Whether a cycle was detected and broken.
    pub cycle_detected: bool,
    /// Whether the traversal hit the depth limit before exhausting the chain.
    pub depth_limit_reached: bool,
    /// Deterministic summary of the traversal (no LLM).
    pub traversal_summary: String,
}

impl MemoryGraphTraversalResult {
    /// Create a result for a single trace with no linked memories.
    pub fn single(root_trace: HolographicTrace) -> Self {
        let root_id = root_trace.id.clone();
        Self {
            root_trace_id: root_id.clone(),
            visited_traces: vec![root_trace],
            visited_trace_ids: vec![root_id.clone()],
            reachable_depth: 0,
            max_depth_limit: 1,
            cycle_detected: false,
            depth_limit_reached: false,
            traversal_summary: format!(
                "Traversal from '{}': visited 1 traces across 0 depth levels.",
                root_id
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic encoding helpers
// ---------------------------------------------------------------------------

/// Hash a single term into a u64 value using a given seed.
///
/// Uses `DefaultHasher` for deterministic non-cryptographic hashing.
/// The seed is injected by hashing it first, so different seeds produce
/// completely different hash values.
fn hash_term_with_seed(term: &str, seed: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    term.hash(&mut hasher);
    hasher.finish()
}

/// Number of bit-positions produced per term per field.
const HASHES_PER_TERM: u64 = 3;

/// Normalize a list of terms: lowercase, trim, deduplicate, remove empties.
fn normalize_terms(terms: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    terms
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

/// Encode a set of terms into a vector of deterministic u64 bit-positions.
///
/// Each term is hashed `hashes_per_term` times with different seeds
/// derived from `base_seed`.
fn encode_term_set(terms: &[String], base_seed: u64, hashes_per_term: u64) -> Vec<u64> {
    let mut bits = BTreeSet::new(); // dedup + sort
    for term in terms {
        for i in 0..hashes_per_term {
            let seed = base_seed.wrapping_add(i);
            bits.insert(hash_term_with_seed(term, seed));
        }
    }
    bits.into_iter().collect()
}

/// Encode keywords, concepts, entities and linked decision IDs into a
/// single distributed signature.
///
/// This function is **deterministic**: the same inputs always produce
/// the same signature.
pub fn encode_terms_to_signature(
    keywords: &[String],
    concepts: &[String],
    entities: &[String],
    linked_decision_ids: &[String],
) -> DistributedSignature {
    let normalized_keywords = normalize_terms(keywords);
    let normalized_concepts = normalize_terms(concepts);
    let normalized_entities = normalize_terms(entities);
    let normalized_decisions = normalize_terms(linked_decision_ids);

    DistributedSignature {
        symbolic_bits: encode_term_set(&normalized_keywords, 42, HASHES_PER_TERM),
        concept_bits: encode_term_set(&normalized_concepts, 100, HASHES_PER_TERM),
        entity_bits: encode_term_set(&normalized_entities, 200, HASHES_PER_TERM),
        decision_bits: encode_term_set(&normalized_decisions, 300, HASHES_PER_TERM),
    }
}

// ---------------------------------------------------------------------------
// Similarity helpers
// ---------------------------------------------------------------------------

/// Compute Jaccard overlap between two sorted u64 vectors.
///
/// Returns `|a ∩ b| / |a ∪ b|`, or 0.0 if both are empty.
fn jaccard_overlap(a: &[u64], b: &[u64]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let set_a: BTreeSet<u64> = a.iter().copied().collect();
    let set_b: BTreeSet<u64> = b.iter().copied().collect();

    let intersection_size = set_a.intersection(&set_b).count();
    let union_size = set_a.union(&set_b).count();

    if union_size == 0 {
        0.0
    } else {
        intersection_size as f32 / union_size as f32
    }
}

/// Compute the multi-dimensional overlap between two distributed signatures.
///
/// Returns a `ResonanceScore` with individual overlap components and boosts.
pub fn signature_overlap(
    query_sig: &DistributedSignature,
    trace_sig: &DistributedSignature,
    importance: f32,
    confidence: f32,
    activation_count: u64,
) -> ResonanceScore {
    let symbolic_overlap = jaccard_overlap(&query_sig.symbolic_bits, &trace_sig.symbolic_bits);
    let concept_overlap = jaccard_overlap(&query_sig.concept_bits, &trace_sig.concept_bits);
    let entity_overlap = jaccard_overlap(&query_sig.entity_bits, &trace_sig.entity_bits);
    let decision_overlap = jaccard_overlap(&query_sig.decision_bits, &trace_sig.decision_bits);

    let importance_boost = importance * 0.10;
    let confidence_boost = confidence * 0.05;
    let activation_boost = (activation_count as f32 * 0.01).min(0.20);

    let total = symbolic_overlap * 0.30
        + concept_overlap * 0.30
        + entity_overlap * 0.30
        + decision_overlap * 0.10
        + importance_boost
        + confidence_boost
        + activation_boost;

    ResonanceScore {
        total,
        symbolic_overlap,
        concept_overlap,
        entity_overlap,
        decision_overlap,
        importance_boost,
        confidence_boost,
        activation_boost,
    }
}

// ---------------------------------------------------------------------------
// Matching helpers
// ---------------------------------------------------------------------------

/// Find which query terms appear in a trace's term list (case-insensitive).
fn find_matching_terms(query_terms: &[String], trace_terms: &[String]) -> Vec<String> {
    let trace_set: HashSet<String> = trace_terms.iter().map(|t| t.to_lowercase()).collect();
    query_terms
        .iter()
        .filter(|qt| trace_set.contains(&qt.to_lowercase()))
        .cloned()
        .collect()
}

/// Build a deterministic reconstruction summary from match results.
fn build_reconstruction_summary(
    matches: &[ResonanceMatch],
    activated_trace_ids: &[String],
    linked_memory_ids: &[String],
    linked_decision_ids: &[String],
) -> String {
    let trace_count = matches.len();
    let top_score = matches.first().map(|m| m.score.total).unwrap_or(0.0);

    let mut parts: Vec<String> = Vec::new();

    parts.push(format!(
        "Found {} matching traces (top score: {:.4}).",
        trace_count, top_score
    ));

    if !activated_trace_ids.is_empty() {
        parts.push(format!(
            "Activated traces ({}): {}",
            activated_trace_ids.len(),
            activated_trace_ids.join(", ")
        ));
    }

    if !linked_decision_ids.is_empty() {
        parts.push(format!(
            "Linked decisions ({}): {}",
            linked_decision_ids.len(),
            linked_decision_ids.join(", ")
        ));
    }

    if !linked_memory_ids.is_empty() {
        parts.push(format!(
            "Linked memories ({}): {}",
            linked_memory_ids.len(),
            linked_memory_ids.join(", ")
        ));
    }

    parts.join("\n")
}

// ---------------------------------------------------------------------------
// HolographicMemoryStore trait
// ---------------------------------------------------------------------------

/// A holographic memory store that supports adding, retrieving, and
/// listing traces, as well as resonance-based retrieval.
pub trait HolographicMemoryStore {
    /// Add a trace to the store.
    ///
    /// Returns an error if a trace with the same ID already exists.
    fn add_trace(&mut self, trace: HolographicTrace) -> Result<(), HolographicMemoryError>;

    /// Get a trace by its ID.
    fn get_trace(&self, trace_id: &str) -> Result<&HolographicTrace, HolographicMemoryError>;

    /// List all traces for a given project.
    fn list_traces(&self, project_id: &str) -> Vec<&HolographicTrace>;

    /// Retrieve traces by resonance with the given query.
    ///
    /// The query is encoded into a signature, compared against all traces
    /// in the same project, and the top matches (by resonance score) are
    /// returned. Traces that match have their `activation_count`
    /// incremented.
    fn retrieve_by_resonance(
        &mut self,
        project_id: &str,
        query: HolographicQuery,
        limit: usize,
    ) -> ReconstructedContext;

    /// Increment the activation count of a trace.
    ///
    /// Returns an error if the trace is not found.
    fn activate_trace(&mut self, trace_id: &str) -> Result<(), HolographicMemoryError>;

    /// Recursively traverse the linked-memory graph starting from a root trace.
    ///
    /// Follows `linked_memory_ids` chains using BFS, respecting the configured
    /// `max_depth`. Detects and breaks cycles. Returns all reachable traces
    /// in discovery order with traversal metadata.
    fn traverse_linked_memories(
        &self,
        root_trace_id: &str,
        max_depth: usize,
    ) -> Result<MemoryGraphTraversalResult, HolographicMemoryError>;
}

// ---------------------------------------------------------------------------
// InMemoryHolographicMemoryStore
// ---------------------------------------------------------------------------

/// An in-memory implementation of `HolographicMemoryStore`.
///
/// All data is stored in a `HashMap<String, HolographicTrace>` keyed by
/// trace ID. Retrieval is a linear scan over traces matching the project.
/// This is suitable for development, testing, and single-node deployments.
#[derive(Clone, Debug)]
pub struct InMemoryHolographicMemoryStore {
    traces: HashMap<String, HolographicTrace>,
}

impl InMemoryHolographicMemoryStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            traces: HashMap::new(),
        }
    }

    /// Return the total number of traces across all projects.
    pub fn len(&self) -> usize {
        self.traces.len()
    }

    /// Returns true if the store contains no traces.
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// Save all traces to a JSON file.
    ///
    /// The file contains a JSON object keyed by trace ID. The serialized
    /// form uses `serde_json` and is human-readable (pretty-printed).
    pub fn save_to_file(&self, path: &str) -> Result<(), HolographicMemoryError> {
        let json = serde_json::to_string_pretty(&self.traces).map_err(|e| {
            HolographicMemoryError::PersistenceError(format!("serialization failed: {}", e))
        })?;
        std::fs::write(path, &json).map_err(|e| {
            HolographicMemoryError::PersistenceError(format!("write failed: {}", e))
        })?;
        Ok(())
    }

    /// Load traces from a JSON file previously written by `save_to_file`.
    ///
    /// Returns a new store pre-populated with the deserialized traces.
    /// If the file does not exist or is invalid, returns a `PersistenceError`.
    pub fn load_from_file(path: &str) -> Result<Self, HolographicMemoryError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| HolographicMemoryError::PersistenceError(format!("read failed: {}", e)))?;
        let traces: HashMap<String, HolographicTrace> =
            serde_json::from_str(&json).map_err(|e| {
                HolographicMemoryError::PersistenceError(format!("deserialization failed: {}", e))
            })?;
        Ok(Self { traces })
    }
}

impl Default for InMemoryHolographicMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HolographicMemoryStore for InMemoryHolographicMemoryStore {
    fn add_trace(&mut self, trace: HolographicTrace) -> Result<(), HolographicMemoryError> {
        let id = trace.id.clone();
        if self.traces.contains_key(&id) {
            return Err(HolographicMemoryError::TraceAlreadyExists(id));
        }
        self.traces.insert(id, trace);
        Ok(())
    }

    fn get_trace(&self, trace_id: &str) -> Result<&HolographicTrace, HolographicMemoryError> {
        self.traces
            .get(trace_id)
            .ok_or_else(|| HolographicMemoryError::TraceNotFound(trace_id.to_owned()))
    }

    fn list_traces(&self, project_id: &str) -> Vec<&HolographicTrace> {
        self.traces
            .values()
            .filter(|t| t.project_id == project_id)
            .collect()
    }

    fn retrieve_by_resonance(
        &mut self,
        project_id: &str,
        query: HolographicQuery,
        limit: usize,
    ) -> ReconstructedContext {
        // If query has no terms, return empty context
        if query.is_empty() {
            return ReconstructedContext::empty(project_id.to_owned(), query.text.clone());
        }

        // Collect project traces + compute resonance scores
        let mut scored: Vec<(ResonanceScore, String)> = Vec::new();
        let mut matched_keywords_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut matched_concepts_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut matched_entities_map: HashMap<String, Vec<String>> = HashMap::new();

        for trace in self.traces.values() {
            if trace.project_id != project_id {
                continue;
            }

            let score = signature_overlap(
                &query.distributed_signature,
                &trace.distributed_signature,
                trace.importance,
                trace.confidence,
                trace.activation_count,
            );

            // Only keep traces with at least one non-zero overlap dimension.
            // Boosts (importance, confidence, activation) are ranking factors,
            // not match-creators — they should never make a zero-overlap trace
            // appear as a match.
            let has_overlap = score.symbolic_overlap > 1e-9
                || score.concept_overlap > 1e-9
                || score.entity_overlap > 1e-9
                || score.decision_overlap > 1e-9;
            if !has_overlap {
                continue;
            }

            let matched_keywords = find_matching_terms(&query.keywords, &trace.keywords);
            let matched_concepts = find_matching_terms(&query.concepts, &trace.concepts);
            let matched_entities = find_matching_terms(&query.entities, &trace.entities);

            scored.push((score.clone(), trace.id.clone()));
            matched_keywords_map.insert(trace.id.clone(), matched_keywords);
            matched_concepts_map.insert(trace.id.clone(), matched_concepts);
            matched_entities_map.insert(trace.id.clone(), matched_entities);
        }

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.0.total
                .partial_cmp(&a.0.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top N
        let top: Vec<(ResonanceScore, String)> = scored.into_iter().take(limit).collect();

        // Build matches
        let mut matches: Vec<ResonanceMatch> = Vec::new();
        let mut activated_trace_ids: Vec<String> = Vec::new();
        let mut linked_memory_set: BTreeSet<String> = BTreeSet::new();
        let mut linked_decision_set: BTreeSet<String> = BTreeSet::new();

        for (score, trace_id) in &top {
            if let Some(trace) = self.traces.get(trace_id) {
                let matched_keywords = matched_keywords_map.remove(trace_id).unwrap_or_default();
                let matched_concepts = matched_concepts_map.remove(trace_id).unwrap_or_default();
                let matched_entities = matched_entities_map.remove(trace_id).unwrap_or_default();

                matches.push(ResonanceMatch {
                    trace: trace.clone(),
                    score: score.clone(),
                    matched_keywords,
                    matched_concepts,
                    matched_entities,
                });

                activated_trace_ids.push(trace_id.clone());
                linked_memory_set.extend(trace.linked_memory_ids.clone());
                linked_decision_set.extend(trace.linked_decision_ids.clone());
            }
        }

        // Activate matched traces
        for trace_id in &activated_trace_ids {
            if let Some(trace) = self.traces.get_mut(trace_id) {
                trace.activation_count = trace.activation_count.saturating_add(1);
            }
        }

        let linked_memory_ids: Vec<String> = linked_memory_set.into_iter().collect();
        let linked_decision_ids: Vec<String> = linked_decision_set.into_iter().collect();

        let reconstruction_summary = build_reconstruction_summary(
            &matches,
            &activated_trace_ids,
            &linked_memory_ids,
            &linked_decision_ids,
        );

        ReconstructedContext {
            project_id: project_id.to_owned(),
            query: query.text.clone(),
            matches,
            activated_trace_ids,
            linked_memory_ids,
            linked_decision_ids,
            reconstruction_summary,
        }
    }

    fn activate_trace(&mut self, trace_id: &str) -> Result<(), HolographicMemoryError> {
        let trace = self
            .traces
            .get_mut(trace_id)
            .ok_or_else(|| HolographicMemoryError::TraceNotFound(trace_id.to_owned()))?;
        trace.activation_count = trace.activation_count.saturating_add(1);
        Ok(())
    }

    fn traverse_linked_memories(
        &self,
        root_trace_id: &str,
        max_depth: usize,
    ) -> Result<MemoryGraphTraversalResult, HolographicMemoryError> {
        // Verify root trace exists
        let root_trace = self
            .traces
            .get(root_trace_id)
            .ok_or_else(|| HolographicMemoryError::TraceNotFound(root_trace_id.to_owned()))?
            .clone();

        if root_trace.linked_memory_ids.is_empty() || max_depth == 0 {
            let result = MemoryGraphTraversalResult::single(root_trace);
            return Ok(result);
        }

        // BFS traversal using VecDeque
        let mut visited: HashSet<String> = HashSet::new();
        let mut discovery_order: Vec<String> = Vec::new();
        let mut visited_traces_vec: Vec<HolographicTrace> = Vec::new();
        let mut cycle_detected = false;
        let mut depth_limit_reached = false;

        // (trace_id, current_depth)
        let mut queue: std::collections::VecDeque<(String, usize)> =
            std::collections::VecDeque::new();

        // Start with root
        visited.insert(root_trace_id.to_owned());
        discovery_order.push(root_trace_id.to_owned());
        visited_traces_vec.push(root_trace.clone());

        // Enqueue root's linked memories at depth 1
        for linked_id in &root_trace.linked_memory_ids {
            queue.push_back((linked_id.clone(), 1));
        }

        let mut max_reached_depth: usize = 0;

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth > max_depth {
                depth_limit_reached = true;
                continue;
            }

            if depth > max_reached_depth {
                max_reached_depth = depth;
            }

            // Cycle detection
            if visited.contains(&current_id) {
                cycle_detected = true;
                continue;
            }

            // Look up the trace
            if let Some(trace) = self.traces.get(&current_id) {
                visited.insert(current_id.clone());
                discovery_order.push(current_id.clone());
                visited_traces_vec.push(trace.clone());

                // Enqueue this trace's linked memories at next depth
                let next_depth = depth + 1;
                for linked_id in &trace.linked_memory_ids {
                    if !visited.contains(linked_id) {
                        queue.push_back((linked_id.clone(), next_depth));
                    } else {
                        cycle_detected = true;
                    }
                }
            } else {
                // linked_memory_id references a trace that doesn't exist
                // Still mark it visited so we don't retry it
                visited.insert(current_id.clone());
            }
        }

        // Build summary
        let parts: Vec<String> = {
            let mut p = Vec::new();
            p.push(format!(
                "Traversal from '{}': visited {} traces across {} depth levels.",
                root_trace_id,
                visited_traces_vec.len(),
                max_reached_depth
            ));
            if cycle_detected {
                p.push("Cycle(s) detected and broken.".to_owned());
            }
            if depth_limit_reached {
                p.push(format!(
                    "Depth limit ({}) reached; chain may extend further.",
                    max_depth
                ));
            }
            p
        };

        Ok(MemoryGraphTraversalResult {
            root_trace_id: root_trace_id.to_owned(),
            visited_traces: visited_traces_vec,
            visited_trace_ids: discovery_order,
            reachable_depth: max_reached_depth,
            max_depth_limit: max_depth,
            cycle_detected,
            depth_limit_reached,
            traversal_summary: parts.join("\n"),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_trace(
        id: &str,
        project_id: &str,
        source_kind: SourceKind,
        keywords: Vec<String>,
        concepts: Vec<String>,
        entities: Vec<String>,
        linked_decision_ids: Vec<String>,
        linked_memory_ids: Vec<String>,
        importance: f32,
        confidence: f32,
    ) -> HolographicTrace {
        HolographicTrace::new(
            id.to_owned(),
            project_id.to_owned(),
            source_kind,
            format!("source-{id}"),
            vec![format!("turn-{id}-1"), format!("turn-{id}-2")],
            format!("Summary for {id}"),
            keywords,
            concepts,
            entities,
            linked_memory_ids,
            linked_decision_ids,
            importance,
            confidence,
            0.0, // emotional_weight
            0.0, // strategic_weight
            "2026-05-26T00:00:00Z".to_owned(),
        )
    }

    fn make_query(
        project_id: &str,
        text: &str,
        keywords: Vec<String>,
        concepts: Vec<String>,
        entities: Vec<String>,
    ) -> HolographicQuery {
        HolographicQuery::new(
            project_id.to_owned(),
            text.to_owned(),
            keywords,
            concepts,
            entities,
        )
    }

    // -----------------------------------------------------------------------
    // 1. add_trace_and_list_by_project
    // -----------------------------------------------------------------------

    #[test]
    fn add_trace_and_list_by_project() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace_a = make_trace(
            "trace-1",
            "project-alpha",
            SourceKind::ConversationTurn,
            vec!["hello".to_owned()],
            vec!["greeting".to_owned()],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        let trace_b = make_trace(
            "trace-2",
            "project-alpha",
            SourceKind::MemoryCandidate,
            vec!["world".to_owned()],
            vec!["planet".to_owned()],
            vec![],
            vec![],
            vec![],
            0.3,
            0.6,
        );

        store.add_trace(trace_a).unwrap();
        store.add_trace(trace_b).unwrap();

        let traces = store.list_traces("project-alpha");
        assert_eq!(traces.len(), 2);
    }

    // -----------------------------------------------------------------------
    // 2. project_scope_prevents_memory_leak
    // -----------------------------------------------------------------------

    #[test]
    fn project_scope_prevents_memory_leak() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace_a = make_trace(
            "trace-a",
            "project-alpha",
            SourceKind::ConversationTurn,
            vec!["secret".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        let trace_b = make_trace(
            "trace-b",
            "project-beta",
            SourceKind::ConversationTurn,
            vec!["public".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );

        store.add_trace(trace_a).unwrap();
        store.add_trace(trace_b).unwrap();

        let alpha_traces = store.list_traces("project-alpha");
        assert_eq!(alpha_traces.len(), 1);
        assert_eq!(alpha_traces[0].id, "trace-a");

        let beta_traces = store.list_traces("project-beta");
        assert_eq!(beta_traces.len(), 1);
        assert_eq!(beta_traces[0].id, "trace-b");
    }

    // -----------------------------------------------------------------------
    // 3. deterministic_signature_encoding
    // -----------------------------------------------------------------------

    #[test]
    fn deterministic_signature_encoding() {
        let keywords = vec!["alpha".to_owned(), "beta".to_owned()];
        let concepts = vec!["concept-x".to_owned()];
        let entities = vec!["entity-1".to_owned()];
        let decisions = vec!["decision-123".to_owned()];

        let sig1 = encode_terms_to_signature(&keywords, &concepts, &entities, &decisions);
        let sig2 = encode_terms_to_signature(&keywords, &concepts, &entities, &decisions);

        assert_eq!(sig1.symbolic_bits, sig2.symbolic_bits);
        assert_eq!(sig1.concept_bits, sig2.concept_bits);
        assert_eq!(sig1.entity_bits, sig2.entity_bits);
        assert_eq!(sig1.decision_bits, sig2.decision_bits);
    }

    // -----------------------------------------------------------------------
    // 4. same_terms_same_signature
    // -----------------------------------------------------------------------

    #[test]
    fn same_terms_same_signature() {
        // Two different calls with the same terms, different order
        let a = vec!["hello".to_owned(), "world".to_owned()];
        let b = vec!["world".to_owned(), "hello".to_owned()];

        let sig_a = encode_terms_to_signature(&a, &[], &[], &[]);
        let sig_b = encode_terms_to_signature(&b, &[], &[], &[]);

        assert_eq!(sig_a.symbolic_bits, sig_b.symbolic_bits);
    }

    // -----------------------------------------------------------------------
    // 5. different_terms_different_signature
    // -----------------------------------------------------------------------

    #[test]
    fn different_terms_different_signature() {
        let sig_a = encode_terms_to_signature(&["foo".to_owned()], &[], &[], &[]);
        let sig_b = encode_terms_to_signature(&["bar".to_owned()], &[], &[], &[]);

        assert_ne!(sig_a.symbolic_bits, sig_b.symbolic_bits);
    }

    // -----------------------------------------------------------------------
    // 6. resonance_retrieval_matches_keyword
    // -----------------------------------------------------------------------

    #[test]
    fn resonance_retrieval_matches_keyword() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "match-1",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned(), "performance".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        let query = make_query(
            "proj",
            "looking for rust",
            vec!["rust".to_owned()],
            vec![],
            vec![],
        );
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert!(!ctx.matches.is_empty(), "expected at least one match");
        assert!(ctx.matches[0].matched_keywords.contains(&"rust".to_owned()));
    }

    // -----------------------------------------------------------------------
    // 7. resonance_retrieval_matches_concept_without_exact_text
    // -----------------------------------------------------------------------

    #[test]
    fn resonance_retrieval_matches_concept_without_exact_text() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "conc-1",
            "proj",
            SourceKind::ArchitectureDecision,
            vec![],
            vec!["machine-learning".to_owned()],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        // Query with the same concept — should match via signature overlap
        let query = make_query(
            "proj",
            "ml concepts",
            vec![],
            vec!["machine-learning".to_owned()],
            vec![],
        );
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert!(!ctx.matches.is_empty(), "expected concept match");
        assert!(ctx.matches[0]
            .matched_concepts
            .contains(&"machine-learning".to_owned()));
    }

    // -----------------------------------------------------------------------
    // 8. resonance_retrieval_matches_entity
    // -----------------------------------------------------------------------

    #[test]
    fn resonance_retrieval_matches_entity() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "ent-1",
            "proj",
            SourceKind::AuditEvent,
            vec![],
            vec![],
            vec!["arpagona".to_owned()],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        let query = make_query(
            "proj",
            "company",
            vec![],
            vec![],
            vec!["arpagona".to_owned()],
        );
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert!(!ctx.matches.is_empty(), "expected entity match");
        assert!(ctx.matches[0]
            .matched_entities
            .contains(&"arpagona".to_owned()));
    }

    // -----------------------------------------------------------------------
    // 9. high_confidence_scores_above_low_confidence
    // -----------------------------------------------------------------------

    #[test]
    fn high_confidence_scores_above_low_confidence() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Both traces have the same keyword, but different confidence
        let trace_high = make_trace(
            "high-conf",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5, // importance
            1.0, // confidence
        );
        let trace_low = make_trace(
            "low-conf",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5, // importance
            0.1, // confidence
        );

        store.add_trace(trace_high).unwrap();
        store.add_trace(trace_low).unwrap();

        let query = make_query("proj", "rust lang", vec!["rust".to_owned()], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert_eq!(ctx.matches.len(), 2, "expected both traces to match");
        assert_eq!(
            ctx.matches[0].trace.id, "high-conf",
            "high confidence trace should rank first"
        );
        assert!(
            ctx.matches[0].score.total > ctx.matches[1].score.total,
            "high confidence score should be greater"
        );
    }

    // -----------------------------------------------------------------------
    // 10. importance_boost_affects_ranking
    // -----------------------------------------------------------------------

    #[test]
    fn importance_boost_affects_ranking() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Both traces have the same keyword, but different importance
        let trace_high = make_trace(
            "high-imp",
            "proj",
            SourceKind::ArchitectureDecision,
            vec!["memory".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            1.0, // importance
            0.5, // confidence
        );
        let trace_low = make_trace(
            "low-imp",
            "proj",
            SourceKind::ConversationTurn,
            vec!["memory".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.0, // importance
            0.5, // confidence
        );

        store.add_trace(trace_high).unwrap();
        store.add_trace(trace_low).unwrap();

        let query = make_query("proj", "memory", vec!["memory".to_owned()], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert_eq!(ctx.matches.len(), 2);
        assert_eq!(
            ctx.matches[0].trace.id, "high-imp",
            "high importance trace should rank first"
        );
    }

    // -----------------------------------------------------------------------
    // 11. activation_count_increases_after_retrieval
    // -----------------------------------------------------------------------

    #[test]
    fn activation_count_increases_after_retrieval() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "act-1",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        // First retrieval
        let query = make_query("proj", "rust", vec!["rust".to_owned()], vec![], vec![]);
        let ctx1 = store.retrieve_by_resonance("proj", query.clone(), 10);
        assert_eq!(ctx1.activated_trace_ids.len(), 1);

        let trace = store.get_trace("act-1").unwrap();
        assert_eq!(
            trace.activation_count, 1,
            "activation count should be 1 after first retrieval"
        );

        // Second retrieval
        let ctx2 = store.retrieve_by_resonance("proj", query, 10);
        assert_eq!(ctx2.activated_trace_ids.len(), 1);

        let trace = store.get_trace("act-1").unwrap();
        assert_eq!(
            trace.activation_count, 2,
            "activation count should be 2 after second retrieval"
        );
    }

    // -----------------------------------------------------------------------
    // 12. empty_query_returns_empty_context
    // -----------------------------------------------------------------------

    #[test]
    fn empty_query_returns_empty_context() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "some-trace",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        let query = make_query("proj", "", vec![], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert!(
            ctx.matches.is_empty(),
            "empty query should produce no matches"
        );
        assert!(ctx.activated_trace_ids.is_empty());
    }

    // -----------------------------------------------------------------------
    // 13. linked_decisions_are_returned
    // -----------------------------------------------------------------------

    #[test]
    fn linked_decisions_are_returned() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "dec-link-1",
            "proj",
            SourceKind::ArchitectureDecision,
            vec!["auth".to_owned()],
            vec![],
            vec![],
            vec!["decision-abc".to_owned(), "decision-xyz".to_owned()],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        let query = make_query("proj", "auth", vec!["auth".to_owned()], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert!(!ctx.matches.is_empty());
        assert!(
            ctx.linked_decision_ids.contains(&"decision-abc".to_owned()),
            "linked decision decision-abc should be returned"
        );
        assert!(
            ctx.linked_decision_ids.contains(&"decision-xyz".to_owned()),
            "linked decision decision-xyz should be returned"
        );
    }

    // -----------------------------------------------------------------------
    // 14. linked_memories_are_returned
    // -----------------------------------------------------------------------

    #[test]
    fn linked_memories_are_returned() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "mem-link-1",
            "proj",
            SourceKind::MemoryCandidate,
            vec!["memory".to_owned()],
            vec![],
            vec![],
            vec![],
            vec!["mem-001".to_owned(), "mem-002".to_owned()],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        let query = make_query("proj", "memory", vec!["memory".to_owned()], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert!(!ctx.matches.is_empty());
        assert!(
            ctx.linked_memory_ids.contains(&"mem-001".to_owned()),
            "linked memory mem-001 should be returned"
        );
        assert!(
            ctx.linked_memory_ids.contains(&"mem-002".to_owned()),
            "linked memory mem-002 should be returned"
        );
    }

    // -----------------------------------------------------------------------
    // 15. source_turn_ids_are_preserved
    // -----------------------------------------------------------------------

    #[test]
    fn source_turn_ids_are_preserved() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "turn-preserve",
            "proj",
            SourceKind::ConversationTurn,
            vec!["hello".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        // The helper creates source_turn_ids = ["turn-turn-preserve-1", "turn-turn-preserve-2"]
        let stored = store.get_trace("turn-preserve").unwrap();
        assert_eq!(stored.source_turn_ids.len(), 2);
        assert!(stored
            .source_turn_ids
            .contains(&"turn-turn-preserve-1".to_owned()));
        assert!(stored
            .source_turn_ids
            .contains(&"turn-turn-preserve-2".to_owned()));
    }

    // -----------------------------------------------------------------------
    // 16. no_trace_above_threshold_returns_empty_context
    // -----------------------------------------------------------------------

    #[test]
    fn no_trace_above_threshold_returns_empty_context() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "unique-trace",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        // Query with completely different terms — produces zero overlap in all
        // dimensions. Boosts (importance, confidence) are not match-creators,
        // so the trace is excluded.
        let query = make_query(
            "proj",
            "unrelated",
            vec!["quantum".to_owned(), "physics".to_owned()],
            vec![],
            vec![],
        );
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert_eq!(
            ctx.matches.len(),
            0,
            "completely different terms should produce no matches \
             (zero overlap in all dimensions; boosts are ranking factors, \
             not match-creators)"
        );
    }

    // -----------------------------------------------------------------------
    // 17. retrieval_order_is_score_descending
    // -----------------------------------------------------------------------

    #[test]
    fn retrieval_order_is_score_descending() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Three traces with keywords that will all match but with different
        // confidence/importance combinations
        let t1 = make_trace(
            "t1",
            "proj",
            SourceKind::ConversationTurn,
            vec!["test".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.0, // importance
            0.0, // confidence
        );
        let t2 = make_trace(
            "t2",
            "proj",
            SourceKind::ConversationTurn,
            vec!["test".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5, // importance
            0.5, // confidence
        );
        let t3 = make_trace(
            "t3",
            "proj",
            SourceKind::ConversationTurn,
            vec!["test".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            1.0, // importance
            1.0, // confidence
        );

        store.add_trace(t1).unwrap();
        store.add_trace(t2).unwrap();
        store.add_trace(t3).unwrap();

        let query = make_query("proj", "test", vec!["test".to_owned()], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("proj", query, 10);

        assert_eq!(ctx.matches.len(), 3);
        // Verify descending order
        for i in 1..ctx.matches.len() {
            assert!(
                ctx.matches[i - 1].score.total >= ctx.matches[i].score.total,
                "matches should be in descending score order"
            );
        }
        // The highest importance+confidence trace should be first
        assert_eq!(ctx.matches[0].trace.id, "t3");
    }

    // -----------------------------------------------------------------------
    // 18. activation_does_not_cross_project_scope
    // -----------------------------------------------------------------------

    #[test]
    fn activation_does_not_cross_project_scope() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace_a = make_trace(
            "alpha-trace",
            "project-alpha",
            SourceKind::ConversationTurn,
            vec!["shared-keyword".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        let trace_b = make_trace(
            "beta-trace",
            "project-beta",
            SourceKind::ConversationTurn,
            vec!["shared-keyword".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );

        store.add_trace(trace_a).unwrap();
        store.add_trace(trace_b).unwrap();

        // Query in project-alpha
        let query = make_query(
            "project-alpha",
            "shared",
            vec!["shared-keyword".to_owned()],
            vec![],
            vec![],
        );
        let ctx = store.retrieve_by_resonance("project-alpha", query, 10);

        assert_eq!(ctx.matches.len(), 1, "only alpha trace should match");
        assert_eq!(ctx.matches[0].trace.id, "alpha-trace");

        // Verify beta trace was not activated
        let beta_trace = store.get_trace("beta-trace").unwrap();
        assert_eq!(
            beta_trace.activation_count, 0,
            "beta trace should not have been activated"
        );
    }

    // -----------------------------------------------------------------------
    // Additional: duplicate trace ID is rejected
    // -----------------------------------------------------------------------

    #[test]
    fn duplicate_trace_id_is_rejected() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace_a = make_trace(
            "same-id",
            "proj",
            SourceKind::ConversationTurn,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        let trace_b = make_trace(
            "same-id",
            "proj",
            SourceKind::ConversationTurn,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );

        store.add_trace(trace_a).unwrap();
        let result = store.add_trace(trace_b);
        assert!(
            matches!(result, Err(HolographicMemoryError::TraceAlreadyExists(_))),
            "adding a trace with duplicate ID should fail"
        );
    }

    // -----------------------------------------------------------------------
    // Additional: trace not found
    // -----------------------------------------------------------------------

    #[test]
    fn get_nonexistent_trace_returns_error() {
        let store: InMemoryHolographicMemoryStore = InMemoryHolographicMemoryStore::new();

        let result = store.get_trace("nonexistent");
        assert!(
            matches!(result, Err(HolographicMemoryError::TraceNotFound(_))),
            "getting a nonexistent trace should fail"
        );
    }

    // -----------------------------------------------------------------------
    // Additional: SourceKind serialization
    // -----------------------------------------------------------------------

    #[test]
    fn source_kind_serialization() {
        for kind in SourceKind::all() {
            let json = serde_json::to_string(&kind).expect("should serialize");
            let back: SourceKind = serde_json::from_str(&json).expect("should deserialize");
            assert_eq!(kind, back);
        }
    }

    // -----------------------------------------------------------------------
    // Additional: signature normalization (lowercase, trim, dedup)
    // -----------------------------------------------------------------------

    #[test]
    fn signature_normalization() {
        let keywords_a = vec!["  Rust ".to_owned(), "rust".to_owned(), "".to_owned()];
        let keywords_b = vec!["rust".to_owned()];

        let sig_a = encode_terms_to_signature(&keywords_a, &[], &[], &[]);
        let sig_b = encode_terms_to_signature(&keywords_b, &[], &[], &[]);

        assert_eq!(
            sig_a.symbolic_bits, sig_b.symbolic_bits,
            "normalization should treat '  Rust ', 'rust', and '' the same as 'rust'"
        );
    }

    // -----------------------------------------------------------------------
    // Persistence tests
    // -----------------------------------------------------------------------

    #[test]
    fn save_and_load_roundtrip() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "persist-1",
            "proj",
            SourceKind::ConversationTurn,
            vec!["hello".to_owned()],
            vec!["greeting".to_owned()],
            vec!["world".to_owned()],
            vec!["decision-1".to_owned()],
            vec!["mem-1".to_owned()],
            0.5,
            0.8,
        );
        store.add_trace(trace).unwrap();

        let tmp = std::env::temp_dir().join("holographic-test-roundtrip.json");
        let path = tmp.to_str().unwrap().to_owned();

        // Save
        store.save_to_file(&path).expect("save should succeed");

        // Load into a new store
        let loaded =
            InMemoryHolographicMemoryStore::load_from_file(&path).expect("load should succeed");

        assert_eq!(loaded.len(), 1, "loaded store should have 1 trace");
        let loaded_trace = loaded.get_trace("persist-1").expect("trace should exist");
        assert_eq!(loaded_trace.content_summary, "Summary for persist-1");
        assert_eq!(loaded_trace.keywords, vec!["hello"]);
        assert_eq!(loaded_trace.concepts, vec!["greeting"]);
        assert_eq!(loaded_trace.entities, vec!["world"]);
        assert!(loaded_trace
            .linked_decision_ids
            .contains(&"decision-1".to_owned()));
        assert!(loaded_trace.linked_memory_ids.contains(&"mem-1".to_owned()));
        assert_eq!(loaded_trace.source_kind, SourceKind::ConversationTurn);
        assert_eq!(loaded_trace.importance, 0.5);
        assert_eq!(loaded_trace.confidence, 0.8);
        assert_eq!(loaded_trace.activation_count, 0);

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_and_load_preserves_multiple_traces() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let t1 = make_trace(
            "t1",
            "proj",
            SourceKind::ConversationTurn,
            vec!["alpha".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
        );
        let t2 = make_trace(
            "t2",
            "proj",
            SourceKind::ArchitectureDecision,
            vec!["beta".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.3,
            0.6,
        );
        let t3 = make_trace(
            "t3",
            "other-proj",
            SourceKind::MemoryCandidate,
            vec!["gamma".to_owned()],
            vec![],
            vec![],
            vec![],
            vec![],
            0.7,
            0.9,
        );
        store.add_trace(t1).unwrap();
        store.add_trace(t2).unwrap();
        store.add_trace(t3).unwrap();

        let tmp = std::env::temp_dir().join("holographic-test-multi.json");
        let path = tmp.to_str().unwrap().to_owned();

        store.save_to_file(&path).expect("save should succeed");
        let loaded =
            InMemoryHolographicMemoryStore::load_from_file(&path).expect("load should succeed");

        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded.list_traces("proj").len(), 2);
        assert_eq!(loaded.list_traces("other-proj").len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let tmp = std::env::temp_dir().join("holographic-test-nonexistent.json");
        let path = tmp.to_str().unwrap().to_owned();

        let result = InMemoryHolographicMemoryStore::load_from_file(&path);
        assert!(
            matches!(result, Err(HolographicMemoryError::PersistenceError(_))),
            "loading a nonexistent file should return PersistenceError"
        );
    }

    #[test]
    fn save_empty_store_and_load() {
        let store = InMemoryHolographicMemoryStore::new();

        let tmp = std::env::temp_dir().join("holographic-test-empty.json");
        let path = tmp.to_str().unwrap().to_owned();

        store.save_to_file(&path).expect("save should succeed");
        let loaded =
            InMemoryHolographicMemoryStore::load_from_file(&path).expect("load should succeed");

        assert_eq!(loaded.len(), 0);
        assert!(loaded.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_and_load_preserves_signature() {
        let mut store = InMemoryHolographicMemoryStore::new();

        let trace = make_trace(
            "sig-test",
            "proj",
            SourceKind::ConversationTurn,
            vec!["rust".to_owned(), "performance".to_owned()],
            vec!["systems".to_owned()],
            vec!["compiler".to_owned()],
            vec!["dec-1".to_owned()],
            vec![],
            0.5,
            0.8,
        );
        let orig_sig = trace.distributed_signature.clone();
        store.add_trace(trace).unwrap();

        let tmp = std::env::temp_dir().join("holographic-test-sig.json");
        let path = tmp.to_str().unwrap().to_owned();

        store.save_to_file(&path).expect("save should succeed");
        let loaded =
            InMemoryHolographicMemoryStore::load_from_file(&path).expect("load should succeed");

        let loaded_trace = loaded.get_trace("sig-test").expect("trace should exist");
        assert_eq!(
            loaded_trace.distributed_signature, orig_sig,
            "distributed signature should survive save/load roundtrip"
        );

        let _ = std::fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // Traversal tests — recursive linked-memory graph
    // -----------------------------------------------------------------------

    #[test]
    fn traverse_single_trace_no_links() {
        let mut store = InMemoryHolographicMemoryStore::new();
        let trace = make_trace(
            "root",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            1.0,
            1.0,
        );
        store.add_trace(trace).unwrap();

        let result = store
            .traverse_linked_memories("root", 5)
            .expect("traversal should succeed");

        assert_eq!(result.visited_trace_ids, vec!["root"]);
        assert_eq!(result.reachable_depth, 0);
        assert!(!result.cycle_detected);
        assert!(!result.depth_limit_reached);
        assert!(
            result.traversal_summary.contains("visited 1 traces"),
            "summary: {}",
            result.traversal_summary
        );
    }

    #[test]
    fn traverse_basic_chain() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Chain: root -> mid -> leaf
        let root = make_trace(
            "root",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["mid".to_owned()],
            1.0,
            1.0,
        );
        let mid = make_trace(
            "mid",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["leaf".to_owned()],
            1.0,
            1.0,
        );
        let leaf = make_trace(
            "leaf",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            1.0,
            1.0,
        );
        store.add_trace(root).unwrap();
        store.add_trace(mid).unwrap();
        store.add_trace(leaf).unwrap();

        let result = store
            .traverse_linked_memories("root", 10)
            .expect("traversal should succeed");

        assert_eq!(result.visited_trace_ids, vec!["root", "mid", "leaf"]);
        assert_eq!(result.reachable_depth, 2);
        assert_eq!(result.visited_traces.len(), 3);
        assert!(!result.cycle_detected);
        assert!(!result.depth_limit_reached);
    }

    #[test]
    fn traverse_depth_limit() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Chain: root -> a -> b -> c -> d
        let mut prev_id = "d".to_owned();
        for id in &["c", "b", "a", "root"] {
            let t = make_trace(
                id,
                "proj",
                SourceKind::ManualNote,
                vec![],
                vec![],
                vec![],
                vec![],
                vec![prev_id.clone()],
                1.0,
                1.0,
            );
            store.add_trace(t).unwrap();
            prev_id = id.to_string();
        }
        // The chain: root -> a -> b -> c -> d
        // root links to "a", "a" links to "b", "b" links to "c", "c" links to "d"

        // depth limit of 2 should visit: root, a, b (depth 0=root, 1=a, 2=b)
        let result = store
            .traverse_linked_memories("root", 2)
            .expect("traversal should succeed");

        assert_eq!(result.visited_trace_ids, vec!["root", "a", "b"]);
        assert_eq!(result.reachable_depth, 2);
        assert!(result.depth_limit_reached);
        assert_eq!(result.visited_traces.len(), 3);
        assert!(result.traversal_summary.contains("Depth limit"));
    }

    #[test]
    fn traverse_cycle_detection() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Cycle: root -> a -> root (back edge)
        let root = make_trace(
            "root",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["a".to_owned()],
            1.0,
            1.0,
        );
        let a = make_trace(
            "a",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["root".to_owned()],
            1.0,
            1.0,
        );
        store.add_trace(root).unwrap();
        store.add_trace(a).unwrap();

        let result = store
            .traverse_linked_memories("root", 10)
            .expect("traversal should succeed");

        // Should visit root, then a. When a tries to link back to root, cycle is detected.
        assert_eq!(result.visited_trace_ids, vec!["root", "a"]);
        assert!(result.cycle_detected);
        assert!(result.traversal_summary.contains("Cycle"));
    }

    #[test]
    fn traverse_diamond_no_duplicates() {
        let mut store = InMemoryHolographicMemoryStore::new();

        // Diamond: root -> [a, b] -> shared
        let root = make_trace(
            "root",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["a".to_owned(), "b".to_owned()],
            1.0,
            1.0,
        );
        let a = make_trace(
            "a",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["shared".to_owned()],
            1.0,
            1.0,
        );
        let b = make_trace(
            "b",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["shared".to_owned()],
            1.0,
            1.0,
        );
        let shared = make_trace(
            "shared",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            1.0,
            1.0,
        );
        store.add_trace(root).unwrap();
        store.add_trace(a).unwrap();
        store.add_trace(b).unwrap();
        store.add_trace(shared).unwrap();

        let result = store
            .traverse_linked_memories("root", 10)
            .expect("traversal should succeed");

        // root, a, b visited (depth 1), shared visited once (depth 2).
        // "shared" is only visited once (either via a or b), the second path triggers cycle.
        assert_eq!(result.visited_trace_ids.len(), 4);
        assert_eq!(result.visited_trace_ids[0], "root");
        assert_eq!(result.visited_traces.len(), 4);
        assert!(!result.depth_limit_reached);
        // Second path to "shared" is a cycle
        assert!(result.cycle_detected);
    }

    #[test]
    fn traverse_nonexistent_root_returns_error() {
        let store = InMemoryHolographicMemoryStore::new();
        let result = store.traverse_linked_memories("does-not-exist", 5);
        assert!(
            matches!(result, Err(HolographicMemoryError::TraceNotFound(_))),
            "traversing a nonexistent root should return TraceNotFound"
        );
    }

    #[test]
    fn traverse_max_depth_zero_returns_root_only() {
        let mut store = InMemoryHolographicMemoryStore::new();
        let root = make_trace(
            "root",
            "proj",
            SourceKind::ManualNote,
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["a".to_owned()],
            1.0,
            1.0,
        );
        store.add_trace(root).unwrap();

        let result = store
            .traverse_linked_memories("root", 0)
            .expect("traversal with depth 0 should succeed");

        assert_eq!(result.visited_trace_ids, vec!["root"]);
        assert_eq!(result.reachable_depth, 0);
    }
}
