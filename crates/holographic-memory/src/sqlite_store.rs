//! # SQLite-backed Holographic Memory Store
//!
//! A durable [`HolographicMemoryStore`](crate::HolographicMemoryStore) implementation
//! using SQLite via `rusqlite`. Traces are serialized to JSON and stored in a
//! `holographic_traces` table. The store persists across connection drop/reopen
//! cycles, enabling trace survival across server restarts.
//!
//! ## Design
//!
//! The store maintains an **in-memory cache** (`HashMap<String, HolographicTrace>`)
//! alongside the SQLite connection. On construction, all existing traces are loaded
//! from SQLite into the cache. On every mutation (add, activate), both the cache
//! and SQLite are updated. This allows `get_trace()` and `list_traces()` to return
//! references as required by the `HolographicMemoryStore` trait while keeping the
//! SQLite database as the durable backing store.
//!
//! ## Schema
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS holographic_traces (
//!     id              TEXT PRIMARY KEY,
//!     project_id      TEXT NOT NULL,
//!     created_at      TEXT NOT NULL,
//!     importance      REAL NOT NULL DEFAULT 0.0,
//!     confidence      REAL NOT NULL DEFAULT 0.0,
//!     activation_count INTEGER NOT NULL DEFAULT 0,
//!     trace_json      TEXT NOT NULL
//! );
//! ```
//!
//! ## Usage
//!
//! ```no_run
//! use arpagona_holographic_memory::sqlite_store::SqliteHolographicMemoryStore;
//!
//! // Create a new store backed by a file
//! let store = SqliteHolographicMemoryStore::new("my_memory.db").unwrap();
//!
//! // Or use an in-memory SQLite database (for testing)
//! let test_store = SqliteHolographicMemoryStore::in_memory().unwrap();
//! ```

use crate::{
    build_reconstruction_summary, find_matching_terms, signature_overlap, HolographicMemoryError,
    HolographicMemoryStore, HolographicQuery, HolographicTrace, MemoryGraphTraversalResult,
    ReconstructedContext, ResonanceMatch, ResonanceScore,
};
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

/// A SQLite-backed implementation of `HolographicMemoryStore`.
///
/// Combines an in-memory cache (for reference returns) with SQLite
/// persistence (for durability across restarts).
pub struct SqliteHolographicMemoryStore {
    conn: Connection,
    /// In-memory cache of all traces, keyed by trace ID.
    /// Sync'd to SQLite on every mutation.
    cache: HashMap<String, HolographicTrace>,
}

impl std::fmt::Debug for SqliteHolographicMemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteHolographicMemoryStore")
            .field("trace_count", &self.cache.len())
            .finish()
    }
}

impl SqliteHolographicMemoryStore {
    /// Create a new store backed by a file at the given path.
    ///
    /// The database file is created if it does not exist. The schema is
    /// automatically created on first connection. All existing traces are
    /// loaded into the in-memory cache.
    pub fn new(path: &str) -> Result<Self, HolographicMemoryError> {
        let conn = Connection::open(path).map_err(|e| {
            HolographicMemoryError::PersistenceError(format!("failed to open SQLite db: {e}"))
        })?;
        let mut store = Self {
            conn,
            cache: HashMap::new(),
        };
        store.create_schema()?;
        store.load_cache()?;
        Ok(store)
    }

    /// Create a store backed by an in-memory SQLite database.
    ///
    /// Data is lost when the store is dropped. Useful for testing.
    pub fn in_memory() -> Result<Self, HolographicMemoryError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            HolographicMemoryError::PersistenceError(format!(
                "failed to open in-memory SQLite db: {e}"
            ))
        })?;
        let mut store = Self {
            conn,
            cache: HashMap::new(),
        };
        store.create_schema()?;
        Ok(store)
    }

    /// Create the schema tables if they don't exist.
    fn create_schema(&mut self) -> Result<(), HolographicMemoryError> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS holographic_traces (
                    id              TEXT PRIMARY KEY,
                    project_id      TEXT NOT NULL,
                    created_at      TEXT NOT NULL,
                    importance      REAL NOT NULL DEFAULT 0.0,
                    confidence      REAL NOT NULL DEFAULT 0.0,
                    activation_count INTEGER NOT NULL DEFAULT 0,
                    trace_json      TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_traces_project_id
                    ON holographic_traces(project_id);

                CREATE INDEX IF NOT EXISTS idx_traces_created_at
                    ON holographic_traces(created_at);
                ",
            )
            .map_err(|e| {
                HolographicMemoryError::PersistenceError(format!(
                    "failed to create SQLite schema: {e}"
                ))
            })?;
        Ok(())
    }

    /// Load all traces from SQLite into the in-memory cache.
    fn load_cache(&mut self) -> Result<(), HolographicMemoryError> {
        let mut stmt = self
            .conn
            .prepare("SELECT trace_json FROM holographic_traces")
            .map_err(|e| HolographicMemoryError::PersistenceError(format!("query failed: {e}")))?;

        let traces: Vec<HolographicTrace> = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(
                        HolographicMemoryError::PersistenceError(format!(
                            "deserialization failed: {e}"
                        )),
                    ))
                })
            })
            .map_err(|e| HolographicMemoryError::PersistenceError(format!("query failed: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        for trace in traces {
            self.cache.insert(trace.id.clone(), trace);
        }
        Ok(())
    }

    /// Insert a trace row into SQLite.
    fn insert_trace_row(&mut self, trace: &HolographicTrace) -> Result<(), HolographicMemoryError> {
        let trace_json = serde_json::to_string(trace).map_err(|e| {
            HolographicMemoryError::PersistenceError(format!("serialization failed: {e}"))
        })?;

        self.conn
            .execute(
                "INSERT INTO holographic_traces (id, project_id, created_at, importance, confidence, activation_count, trace_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    trace.id,
                    trace.project_id,
                    trace.created_at,
                    trace.importance,
                    trace.confidence,
                    trace.activation_count,
                    trace_json,
                ],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint") {
                    HolographicMemoryError::TraceAlreadyExists(trace.id.clone())
                } else {
                    HolographicMemoryError::PersistenceError(format!("insert failed: {e}"))
                }
            })?;
        Ok(())
    }

    /// Return the total number of traces across all projects.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the store contains no traces.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl HolographicMemoryStore for SqliteHolographicMemoryStore {
    fn add_trace(&mut self, trace: HolographicTrace) -> Result<(), HolographicMemoryError> {
        let id = trace.id.clone();
        if self.cache.contains_key(&id) {
            return Err(HolographicMemoryError::TraceAlreadyExists(id));
        }
        self.insert_trace_row(&trace)?;
        self.cache.insert(id, trace);
        Ok(())
    }

    fn get_trace(&self, trace_id: &str) -> Result<&HolographicTrace, HolographicMemoryError> {
        self.cache
            .get(trace_id)
            .ok_or_else(|| HolographicMemoryError::TraceNotFound(trace_id.to_owned()))
    }

    fn list_traces(&self, project_id: &str) -> Vec<&HolographicTrace> {
        self.cache
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
        if query.is_empty() {
            return ReconstructedContext::empty(project_id.to_owned(), query.text.clone());
        }

        let mut scored: Vec<(ResonanceScore, String)> = Vec::new();
        let mut matched_keywords_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut matched_concepts_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut matched_entities_map: HashMap<String, Vec<String>> = HashMap::new();

        for trace in self.cache.values() {
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

            let has_overlap = score.symbolic_overlap > 1e-9
                || score.concept_overlap > 1e-9
                || score.entity_overlap > 1e-9
                || score.decision_overlap > 1e-9
                || score.embedding_overlap > 1e-9;
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

        scored.sort_by(|a, b| {
            b.0.total
                .partial_cmp(&a.0.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top: Vec<(ResonanceScore, String)> = scored.into_iter().take(limit).collect();

        let mut matches: Vec<ResonanceMatch> = Vec::new();
        let mut activated_trace_ids: Vec<String> = Vec::new();
        let mut linked_memory_set: BTreeSet<String> = BTreeSet::new();
        let mut linked_decision_set: BTreeSet<String> = BTreeSet::new();

        for (score, trace_id) in &top {
            if let Some(trace) = self.cache.get(trace_id) {
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

        // Activate matched traces: update cache + SQLite
        for trace_id in &activated_trace_ids {
            if let Some(trace) = self.cache.get_mut(trace_id) {
                trace.activation_count = trace.activation_count.saturating_add(1);
                // Sync both column and trace_json to SQLite
                if let Ok(json) = serde_json::to_string(&*trace) {
                    let _ = self.conn.execute(
                        "UPDATE holographic_traces SET activation_count = activation_count + 1, trace_json = ?1 WHERE id = ?2",
                        params![json, trace_id],
                    );
                }
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
            .cache
            .get_mut(trace_id)
            .ok_or_else(|| HolographicMemoryError::TraceNotFound(trace_id.to_owned()))?;
        trace.activation_count = trace.activation_count.saturating_add(1);

        // Sync both the column and the trace_json to SQLite
        let trace_json = serde_json::to_string(&*trace).map_err(|e| {
            HolographicMemoryError::PersistenceError(format!("serialization failed: {e}"))
        })?;

        let rows = self
            .conn
            .execute(
                "UPDATE holographic_traces SET activation_count = activation_count + 1, trace_json = ?1 WHERE id = ?2",
                params![trace_json, trace_id],
            )
            .map_err(|e| HolographicMemoryError::PersistenceError(format!("update failed: {e}")))?;

        if rows == 0 {
            return Err(HolographicMemoryError::TraceNotFound(trace_id.to_owned()));
        }
        Ok(())
    }

    fn traverse_linked_memories(
        &self,
        root_trace_id: &str,
        max_depth: usize,
    ) -> Result<MemoryGraphTraversalResult, HolographicMemoryError> {
        let root_trace = self
            .cache
            .get(root_trace_id)
            .ok_or_else(|| HolographicMemoryError::TraceNotFound(root_trace_id.to_owned()))?
            .clone();

        if root_trace.linked_memory_ids.is_empty() || max_depth == 0 {
            return Ok(MemoryGraphTraversalResult::single(root_trace));
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut discovery_order: Vec<String> = Vec::new();
        let mut visited_traces_vec: Vec<HolographicTrace> = Vec::new();
        let mut cycle_detected = false;
        let mut depth_limit_reached = false;
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        visited.insert(root_trace_id.to_owned());
        discovery_order.push(root_trace_id.to_owned());
        visited_traces_vec.push(root_trace.clone());

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
            if visited.contains(&current_id) {
                cycle_detected = true;
                continue;
            }
            if let Some(trace) = self.cache.get(&current_id) {
                visited.insert(current_id.clone());
                discovery_order.push(current_id.clone());
                visited_traces_vec.push(trace.clone());

                if depth < max_depth {
                    for linked_id in &trace.linked_memory_ids {
                        queue.push_back((linked_id.clone(), depth + 1));
                    }
                }
            }
        }

        let visited_count = visited_traces_vec.len();
        Ok(MemoryGraphTraversalResult {
            root_trace_id: root_trace_id.to_owned(),
            visited_traces: visited_traces_vec,
            visited_trace_ids: discovery_order,
            reachable_depth: max_reached_depth,
            max_depth_limit: max_depth,
            cycle_detected,
            depth_limit_reached,
            traversal_summary: format!(
                "Traversal from '{}': visited {} traces across {} depth levels{}.",
                root_trace_id,
                visited_count,
                max_reached_depth,
                if depth_limit_reached {
                    " (depth limit reached)"
                } else {
                    ""
                }
            ),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceKind;

    fn make_trace(id: &str, project_id: &str, keywords: Vec<&str>) -> HolographicTrace {
        HolographicTrace::new(
            id.to_owned(),
            project_id.to_owned(),
            SourceKind::ConversationTurn,
            "source-1".to_owned(),
            vec!["turn-1".to_owned()],
            format!("Test trace {id}"),
            keywords.into_iter().map(|s| s.to_owned()).collect(),
            vec!["test".to_owned()],
            vec![],
            vec![],
            vec![],
            0.5,
            0.8,
            0.1,
            0.3,
            "2026-05-27T00:00:00Z".to_owned(),
        )
    }

    // ---- Basic CRUD ----

    #[test]
    fn new_store_is_empty() {
        let store = SqliteHolographicMemoryStore::in_memory().unwrap();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn add_and_retrieve_trace() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        let trace = make_trace("t1", "proj", vec!["hello"]);
        store.add_trace(trace).unwrap();
        assert_eq!(store.len(), 1);

        let fetched = store.get_trace("t1").unwrap();
        assert_eq!(fetched.id, "t1");
        assert_eq!(fetched.project_id, "proj");
    }

    #[test]
    fn add_duplicate_returns_error() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store.add_trace(make_trace("dup", "p", vec!["a"])).unwrap();
        let err = store
            .add_trace(make_trace("dup", "p", vec!["b"]))
            .unwrap_err();
        assert!(
            matches!(&err, HolographicMemoryError::TraceAlreadyExists(id) if id == "dup"),
            "expected TraceAlreadyExists, got {err}"
        );
    }

    #[test]
    fn get_nonexistent_returns_error() {
        let store = SqliteHolographicMemoryStore::in_memory().unwrap();
        let err = store.get_trace("ghost").unwrap_err();
        assert!(
            matches!(&err, HolographicMemoryError::TraceNotFound(id) if id == "ghost"),
            "expected TraceNotFound, got {err}"
        );
    }

    #[test]
    fn list_traces_scoped_to_project() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("a-1", "proj-a", vec!["x"]))
            .unwrap();
        store
            .add_trace(make_trace("b-1", "proj-b", vec!["x"]))
            .unwrap();
        store
            .add_trace(make_trace("a-2", "proj-a", vec!["y"]))
            .unwrap();

        let proj_a = store.list_traces("proj-a");
        assert_eq!(proj_a.len(), 2);

        let proj_b = store.list_traces("proj-b");
        assert_eq!(proj_b.len(), 1);

        let proj_c = store.list_traces("proj-c");
        assert!(proj_c.is_empty());
    }

    // ---- Activation ----

    #[test]
    fn activate_trace_increments_count() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("act", "p", vec!["test"]))
            .unwrap();
        store.activate_trace("act").unwrap();

        let trace = store.get_trace("act").unwrap();
        assert_eq!(trace.activation_count, 1);
    }

    #[test]
    fn activate_multiple_times() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("act", "p", vec!["test"]))
            .unwrap();
        for _ in 0..5 {
            store.activate_trace("act").unwrap();
        }
        let trace = store.get_trace("act").unwrap();
        assert_eq!(trace.activation_count, 5);
    }

    #[test]
    fn activate_nonexistent_returns_error() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        let err = store.activate_trace("ghost").unwrap_err();
        assert!(
            matches!(&err, HolographicMemoryError::TraceNotFound(id) if id == "ghost"),
            "expected TraceNotFound, got {err}"
        );
    }

    // ---- Resonance retrieval ----

    #[test]
    fn retrieve_by_resonance_matches_correct_trace() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("t1", "p", vec!["hello", "world"]))
            .unwrap();
        store
            .add_trace(make_trace("t2", "p", vec!["foo", "bar"]))
            .unwrap();

        let query = HolographicQuery::new(
            "p".to_owned(),
            "find".to_owned(),
            vec!["hello".to_owned()],
            vec!["test".to_owned()],
            vec![],
        );

        let ctx = store.retrieve_by_resonance("p", query, 5);
        // Both traces share concept "test" with the query, so both match.
        // t1 has keyword + concept overlap (ranks first), t2 has only concept overlap.
        assert_eq!(ctx.matches.len(), 2, "both traces share concept 'test'");
        assert_eq!(
            ctx.matches[0].trace.id, "t1",
            "t1 ranks higher: keyword + concept"
        );
        assert!(ctx.matches[0]
            .matched_keywords
            .contains(&"hello".to_owned()));
    }

    #[test]
    fn retrieve_by_resonance_empty_query_returns_empty() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("t1", "p", vec!["hello"]))
            .unwrap();
        let query =
            HolographicQuery::new("p".to_owned(), "empty".to_owned(), vec![], vec![], vec![]);
        let ctx = store.retrieve_by_resonance("p", query, 5);
        assert!(ctx.matches.is_empty());
    }

    #[test]
    fn retrieve_by_resonance_scoped_to_project() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("t1", "proj-a", vec!["hello"]))
            .unwrap();
        store
            .add_trace(make_trace("t2", "proj-b", vec!["hello"]))
            .unwrap();

        let query = HolographicQuery::new(
            "proj-a".to_owned(),
            "find".to_owned(),
            vec!["hello".to_owned()],
            vec!["test".to_owned()],
            vec![],
        );
        let ctx = store.retrieve_by_resonance("proj-a", query, 5);
        assert_eq!(ctx.matches.len(), 1);
        assert_eq!(ctx.matches[0].trace.id, "t1");
    }

    #[test]
    fn retrieval_activates_traces() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        store
            .add_trace(make_trace("t1", "p", vec!["hello"]))
            .unwrap();

        let q1 = HolographicQuery::new(
            "p".to_owned(),
            "q".to_owned(),
            vec!["hello".to_owned()],
            vec!["test".to_owned()],
            vec![],
        );
        store.retrieve_by_resonance("p", q1, 5);
        let trace = store.get_trace("t1").unwrap();
        assert_eq!(
            trace.activation_count, 1,
            "retrieval should increment activation"
        );
    }

    // ---- Linked memory traversal ----

    #[test]
    fn traverse_linked_memories_from_sqlite() {
        let mut store = SqliteHolographicMemoryStore::in_memory().unwrap();
        let mut root = make_trace("root", "p", vec!["root"]);
        root.linked_memory_ids = vec!["c1".to_owned(), "c2".to_owned()];
        store.add_trace(root).unwrap();
        store.add_trace(make_trace("c1", "p", vec!["c1"])).unwrap();
        store.add_trace(make_trace("c2", "p", vec!["c2"])).unwrap();

        let result = store.traverse_linked_memories("root", 3).unwrap();
        assert_eq!(result.visited_traces.len(), 3);
        assert_eq!(result.reachable_depth, 1);
    }

    #[test]
    fn traverse_nonexistent_root_returns_error() {
        let store = SqliteHolographicMemoryStore::in_memory().unwrap();
        let err = store.traverse_linked_memories("ghost", 3).unwrap_err();
        assert!(matches!(&err, HolographicMemoryError::TraceNotFound(id) if id == "ghost"));
    }

    // ---- Persistence across drop/reopen ----

    #[test]
    fn persistence_across_drop_reopen_cycle() {
        let dir = std::env::temp_dir();
        let db_path = dir.join(format!("hm_test_{}.db", std::process::id()));
        let path_str = db_path.to_str().unwrap().to_owned();
        let _ = std::fs::remove_file(&path_str);

        // Phase 1: add traces
        {
            let mut store = SqliteHolographicMemoryStore::new(&path_str).unwrap();
            store
                .add_trace(make_trace("p1", "proj", vec!["hello"]))
                .unwrap();
            store
                .add_trace(make_trace("p2", "proj", vec!["world"]))
                .unwrap();
            assert_eq!(store.len(), 2);
        }

        // Phase 2: reopen and verify
        {
            let store = SqliteHolographicMemoryStore::new(&path_str).unwrap();
            assert_eq!(store.len(), 2, "traces survive drop/reopen");
            let t1 = store.get_trace("p1").unwrap();
            assert_eq!(t1.project_id, "proj");
        }

        let _ = std::fs::remove_file(&path_str);
    }

    #[test]
    fn persistence_activation_survives_reopen() {
        let dir = std::env::temp_dir();
        let db_path = dir.join(format!("hm_act_{}.db", std::process::id()));
        let path_str = db_path.to_str().unwrap().to_owned();
        let _ = std::fs::remove_file(&path_str);

        // Phase 1: add and activate
        {
            let mut store = SqliteHolographicMemoryStore::new(&path_str).unwrap();
            store
                .add_trace(make_trace("act", "p", vec!["hello"]))
                .unwrap();
            store.activate_trace("act").unwrap();
        }

        // Phase 2: reopen, verify activation persisted
        {
            let mut store = SqliteHolographicMemoryStore::new(&path_str).unwrap();

            // Activation was 1 from phase 1
            let q = HolographicQuery::new(
                "p".to_owned(),
                "q".to_owned(),
                vec!["hello".to_owned()],
                vec!["test".to_owned()],
                vec![],
            );
            let ctx = store.retrieve_by_resonance("p", q, 5);
            assert_eq!(ctx.matches.len(), 1);
            // Trace had activation=1 from phase 1. The match clone captures
            // pre-activation state, so match shows activation_count=1.
            assert_eq!(
                ctx.matches[0].trace.activation_count, 1,
                "match captures pre-activation state"
            );
            // The cache is updated during retrieval though
            let trace = store.get_trace("act").unwrap();
            assert_eq!(
                trace.activation_count, 2,
                "activation persisted and incremented on retrieval"
            );
        }

        let _ = std::fs::remove_file(&path_str);
    }

    #[test]
    fn persistence_multiple_projects() {
        let dir = std::env::temp_dir();
        let db_path = dir.join(format!("hm_multi_{}.db", std::process::id()));
        let path_str = db_path.to_str().unwrap().to_owned();
        let _ = std::fs::remove_file(&path_str);

        {
            let mut store = SqliteHolographicMemoryStore::new(&path_str).unwrap();
            store
                .add_trace(make_trace("a-1", "proj-a", vec!["x"]))
                .unwrap();
            store
                .add_trace(make_trace("b-1", "proj-b", vec!["y"]))
                .unwrap();
        }

        {
            let store = SqliteHolographicMemoryStore::new(&path_str).unwrap();
            assert_eq!(store.len(), 2);
            assert_eq!(store.list_traces("proj-a").len(), 1);
            assert_eq!(store.list_traces("proj-b").len(), 1);
        }

        let _ = std::fs::remove_file(&path_str);
    }
}
