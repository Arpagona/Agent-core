//! Holographic conversation bridge — encodes conversation turns as
//! `arpagona-holographic-memory` `HolographicTrace` objects with
//! distributed signatures and resonance retrieval.
//!
//! This module bridges the `arpagona-conversation-memory` crate (which provides
//! symbolic conversation resonance types) with the `arpagona-holographic-memory`
//! crate (which provides deterministic distributed-signature storage and
//! resonance via `InMemoryHolographicMemoryStore`).
//!
//! # Usage
//!
//! ```rust,ignore
//! use arpagona_conversation_memory::holographic_bridge::{
//!     HolographicConversationBridge, ConversationTurn, Conversation,
//! };
//!
//! let mut bridge = HolographicConversationBridge::new("my-project");
//! bridge.add_turn(ConversationTurn {
//!     role: "user".into(),
//!     content: "What is the architecture?".into(),
//!     turn_id: "turn-1".into(),
//!     importance: 0.7,
//! });
//!
//! // Find similar traces across all turns
//! let similar = bridge.find_similar_for_turns(&turns, 5);
//! ```

use arpagona_holographic_memory::{
    HolographicMemoryError, HolographicMemoryStore, HolographicQuery, HolographicTrace,
    InMemoryHolographicMemoryStore, ReconstructedContext, SourceKind,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Input types for conversation turn data
// ---------------------------------------------------------------------------

/// A single turn in a conversation.
///
/// Each turn represents one message in a conversation (user message, assistant
/// response, system instruction, or tool result).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Role of the participant: "user", "assistant", "system", or "tool".
    pub role: String,
    /// Content of the turn (free text).
    pub content: String,
    /// Unique identifier for this turn within the conversation.
    pub turn_id: String,
    /// How important this turn is (0.0–1.0, default 0.7).
    #[serde(default = "default_importance")]
    pub importance: f32,
}

fn default_importance() -> f32 {
    0.7
}

/// A full conversation with metadata, composed of multiple turns.
///
/// This type can be deserialized from a JSON file for bulk processing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation identifier.
    pub conversation_id: String,
    /// Project scope for all traces in this conversation.
    pub project_id: String,
    /// Optional human-readable title for the conversation.
    pub title: Option<String>,
    /// The ordered turns that make up this conversation.
    pub turns: Vec<ConversationTurn>,
}

// ---------------------------------------------------------------------------
// Keyword extraction helpers
// ---------------------------------------------------------------------------

/// Common English stop words filtered out during keyword extraction.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "could", "should", "may", "might", "can", "shall", "to",
    "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through", "during",
    "before", "after", "above", "below", "between", "out", "off", "over", "under", "again",
    "further", "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
    "every", "both", "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only",
    "own", "same", "so", "than", "too", "very", "just", "because", "but", "and", "or", "if",
    "while", "that", "this", "these", "those", "it", "its", "i", "me", "my", "we", "our", "you",
    "your", "he", "him", "his", "she", "her", "they", "them", "their", "what", "which", "who",
    "whom", "up", "down", "about", "above", "after", "also", "any", "back", "been", "being", "did",
    "does", "doing", "get", "got", "has", "having", "here", "how", "into", "like", "make", "made",
    "making", "more", "most", "much", "must", "need", "new", "now", "one", "other", "our", "over",
    "really", "said", "say", "says", "see", "should", "some", "such", "take", "than", "them",
    "then", "there", "these", "they", "thing", "things", "think", "this", "those", "through",
    "too", "under", "upon", "very", "was", "way", "well", "were", "what", "when", "where", "which",
    "while", "who", "why", "will", "with", "would", "yes",
];

/// Extract significant keywords from text content by splitting on non-alphanumeric
/// characters, lowercasing, filtering stop words and short words, and deduplicating.
///
/// Returns at most 20 keywords.
pub fn extract_keywords(content: &str) -> Vec<String> {
    let stop_set: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    let mut seen = HashSet::new();
    content
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() > 2 && !stop_set.contains(w.as_str()))
        .filter(|w| seen.insert(w.clone()))
        .take(20)
        .collect()
}

/// Derive concepts from a turn's role.
///
/// Each role maps to a standardised concept label used for resonance matching.
pub fn derive_concepts(role: &str) -> Vec<String> {
    match role.to_lowercase().as_str() {
        "user" => vec!["user_query".to_owned()],
        "assistant" => vec!["assistant_response".to_owned()],
        "system" => vec!["system_instruction".to_owned()],
        "tool" => vec!["tool_result".to_owned()],
        _ => vec!["unknown_turn".to_owned()],
    }
}

/// Convert a role string to a `SourceKind` for holographic trace metadata.
fn role_to_source_kind(role: &str) -> SourceKind {
    match role.to_lowercase().as_str() {
        "user" | "assistant" | "system" | "tool" => SourceKind::ConversationTurn,
        _ => SourceKind::ManualNote,
    }
}

// ---------------------------------------------------------------------------
// HolographicConversationBridge
// ---------------------------------------------------------------------------

/// Bridge between structured conversation turns and holographic memory traces.
///
/// Processes conversation turns (user messages, assistant responses, system
/// instructions, tool results) into `HolographicTrace` objects using the
/// `arpagona-holographic-memory` crate's distributed-signature store.
///
/// Key operations:
/// - `process_turn()` — encode a single turn as a holographic trace and store it
/// - `process_conversation()` — process a full `Conversation` in one call
/// - `find_similar_for_turns()` — build a resonance query from given turns and
///   search the store for related patterns
///
/// The underlying `InMemoryHolographicMemoryStore` supports JSON persistence
/// via `save_to_file()` / `load_from_file()`.
pub struct HolographicConversationBridge {
    project_id: String,
    store: InMemoryHolographicMemoryStore,
}

impl HolographicConversationBridge {
    /// Create a new empty bridge for the given project.
    pub fn new(project_id: &str) -> Self {
        Self {
            project_id: project_id.to_owned(),
            store: InMemoryHolographicMemoryStore::new(),
        }
    }

    /// Create a bridge backed by an existing store.
    pub fn with_store(project_id: &str, store: InMemoryHolographicMemoryStore) -> Self {
        Self {
            project_id: project_id.to_owned(),
            store,
        }
    }

    /// Consume the bridge and return the underlying store.
    pub fn into_store(self) -> InMemoryHolographicMemoryStore {
        self.store
    }

    /// Reference to the underlying holographic memory store.
    pub fn store(&self) -> &InMemoryHolographicMemoryStore {
        &self.store
    }

    /// Mutable reference to the underlying holographic memory store.
    pub fn store_mut(&mut self) -> &mut InMemoryHolographicMemoryStore {
        &mut self.store
    }

    /// Process a single conversation turn, encoding it as a holographic trace
    /// with a deterministic distributed signature.
    ///
    /// Keywords are extracted from the turn's content. Concepts are derived from
    /// the turn's role. Returns the trace ID assigned.
    pub fn process_turn(&mut self, turn: &ConversationTurn) -> String {
        let keywords = extract_keywords(&turn.content);
        let concepts = derive_concepts(&turn.role);
        let source_kind = role_to_source_kind(&turn.role);
        let now = Utc::now().to_rfc3339();

        let trace = HolographicTrace::new(
            turn.turn_id.clone(),                        // id
            self.project_id.clone(),                     // project_id
            source_kind,                                 // source_kind
            format!("conv-{}", self.project_id),         // source_id
            vec![turn.turn_id.clone()],                  // source_turn_ids
            format!("[{}] {}", turn.role, turn.content), // content_summary
            keywords,                                    // keywords
            concepts,                                    // concepts
            vec![],                                      // entities
            vec![],                                      // linked_memory_ids
            vec![],                                      // linked_decision_ids
            turn.importance,                             // importance
            0.8,                                         // confidence
            0.0,                                         // emotional_weight
            0.0,                                         // strategic_weight
            now,                                         // created_at
        );

        let trace_id = trace.id.clone();
        let _ = self.store.add_trace(trace);
        trace_id
    }

    /// Process a full `Conversation`, encoding every turn as a holographic trace.
    ///
    /// Returns the number of traces added to the store.
    pub fn process_conversation(&mut self, conversation: &Conversation) -> usize {
        let count = conversation.turns.len();
        for turn in &conversation.turns {
            self.process_turn(turn);
        }
        count
    }

    /// Find traces in the store that resonate with the given turns.
    ///
    /// Builds a single resonance query from all turns' keywords and concepts,
    /// then searches the store. The returned `ReconstructedContext` includes
    /// matched traces, activated trace IDs, and linked decisions.
    pub fn find_similar_for_turns(
        &mut self,
        turns: &[ConversationTurn],
        limit: usize,
    ) -> ReconstructedContext {
        let mut all_keywords: Vec<String> = Vec::new();
        let mut all_concepts: Vec<String> = Vec::new();

        for turn in turns {
            all_keywords.extend(extract_keywords(&turn.content));
            all_concepts.extend(derive_concepts(&turn.role));
        }

        // Deduplicate
        let kw_set: HashSet<String> = all_keywords.into_iter().collect();
        let conc_set: HashSet<String> = all_concepts.into_iter().collect();

        let query = HolographicQuery::new(
            self.project_id.clone(),
            format!("find similar to {} turns", turns.len()),
            kw_set.into_iter().collect(),
            conc_set.into_iter().collect(),
            vec![],
        );

        self.store
            .retrieve_by_resonance(&self.project_id, query, limit)
    }

    /// Save the underlying store to a JSON file.
    pub fn save_to_file(&self, path: &str) -> Result<(), HolographicMemoryError> {
        self.store.save_to_file(path)
    }

    /// Load a store from a JSON file and wrap it in a bridge.
    pub fn load_from_file(project_id: &str, path: &str) -> Result<Self, HolographicMemoryError> {
        let store = InMemoryHolographicMemoryStore::load_from_file(path)?;
        Ok(Self::with_store(project_id, store))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Keyword extraction
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_keywords_removes_stop_words() {
        let content = "The architecture is modular and scalable with caching layers";
        let keywords = extract_keywords(content);
        assert!(
            !keywords.contains(&"the".to_string()),
            "stop word 'the' should be removed"
        );
        assert!(
            !keywords.contains(&"is".to_string()),
            "stop word 'is' should be removed"
        );
        assert!(
            !keywords.contains(&"and".to_string()),
            "stop word 'and' should be removed"
        );
        assert!(keywords.contains(&"architecture".to_string()));
        assert!(keywords.contains(&"modular".to_string()));
        assert!(keywords.contains(&"scalable".to_string()));
        assert!(keywords.contains(&"caching".to_string()));
        assert!(keywords.contains(&"layers".to_string()));
    }

    #[test]
    fn test_extract_keywords_empty_input() {
        let keywords = extract_keywords("");
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_extract_keywords_short_words_filtered() {
        let content = "a an it is up go ok hi";
        let keywords = extract_keywords(content);
        assert!(keywords.is_empty(), "short words should be filtered out");
    }

    #[test]
    fn test_extract_keywords_deduplicates() {
        let content = "memory memory memory memory";
        let keywords = extract_keywords(content);
        assert_eq!(keywords.len(), 1);
        assert_eq!(keywords[0], "memory");
    }

    // -----------------------------------------------------------------------
    // Concept derivation
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_concepts_by_role() {
        assert_eq!(derive_concepts("user"), vec!["user_query"]);
        assert_eq!(derive_concepts("assistant"), vec!["assistant_response"]);
        assert_eq!(derive_concepts("system"), vec!["system_instruction"]);
        assert_eq!(derive_concepts("tool"), vec!["tool_result"]);
        assert_eq!(derive_concepts("unknown"), vec!["unknown_turn"]);
    }

    #[test]
    fn test_derive_concepts_case_insensitive() {
        assert_eq!(derive_concepts("User"), vec!["user_query"]);
        assert_eq!(derive_concepts("ASSISTANT"), vec!["assistant_response"]);
        assert_eq!(derive_concepts("System"), vec!["system_instruction"]);
        assert_eq!(derive_concepts("Tool"), vec!["tool_result"]);
    }

    // -----------------------------------------------------------------------
    // Single turn processing
    // -----------------------------------------------------------------------

    #[test]
    fn test_bridge_processes_single_turn() {
        let mut bridge = HolographicConversationBridge::new("test-project");
        let turn = ConversationTurn {
            role: "user".into(),
            content: "What is the architecture of the system?".into(),
            turn_id: "turn-1".into(),
            importance: 0.7,
        };
        let trace_id = bridge.process_turn(&turn);
        assert_eq!(trace_id, "turn-1");
        assert_eq!(bridge.store().len(), 1);

        let traces = bridge.store().list_traces("test-project");
        assert_eq!(traces.len(), 1);
        assert!(traces[0].content_summary.contains("[user]"));
        assert!(traces[0].keywords.contains(&"architecture".to_string()));
        assert!(traces[0].keywords.contains(&"system".to_string()));
    }

    #[test]
    fn test_bridge_processes_user_turn_with_correct_concept() {
        let mut bridge = HolographicConversationBridge::new("concept-test");
        bridge.process_turn(&ConversationTurn {
            role: "user".into(),
            content: "How does the memory module work?".into(),
            turn_id: "t1".into(),
            importance: 0.7,
        });
        let traces = bridge.store().list_traces("concept-test");
        assert_eq!(traces.len(), 1);
        assert!(traces[0].concepts.contains(&"user_query".to_string()));
        assert_eq!(traces[0].source_kind, SourceKind::ConversationTurn);
    }

    // -----------------------------------------------------------------------
    // Multi-turn processing
    // -----------------------------------------------------------------------

    #[test]
    fn test_bridge_processes_multiple_turns() {
        let mut bridge = HolographicConversationBridge::new("multi-turn-project");
        let turns = vec![
            ConversationTurn {
                role: "user".into(),
                content: "How does the memory module work?".into(),
                turn_id: "t1".into(),
                importance: 0.7,
            },
            ConversationTurn {
                role: "assistant".into(),
                content: "The memory module uses distributed signatures for resonance retrieval."
                    .into(),
                turn_id: "t2".into(),
                importance: 0.8,
            },
        ];
        for turn in &turns {
            bridge.process_turn(turn);
        }
        assert_eq!(bridge.store().len(), 2);

        let traces = bridge.store().list_traces("multi-turn-project");
        assert_eq!(traces.len(), 2);
        assert!(
            traces
                .iter()
                .any(|t| t.concepts.contains(&"user_query".to_string())),
            "user trace should have user_query concept"
        );
        assert!(
            traces
                .iter()
                .any(|t| t.concepts.contains(&"assistant_response".to_string())),
            "assistant trace should have assistant_response concept"
        );
    }

    // -----------------------------------------------------------------------
    // Full conversation
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_conversation_processing() {
        let mut bridge = HolographicConversationBridge::new("full-conv");
        let conversation = Conversation {
            conversation_id: "conv-1".into(),
            project_id: "full-conv".into(),
            title: Some("Test conversation".into()),
            turns: vec![
                ConversationTurn {
                    role: "user".into(),
                    content: "Explain holographic memory.".into(),
                    turn_id: "t1".into(),
                    importance: 0.7,
                },
                ConversationTurn {
                    role: "assistant".into(),
                    content:
                        "Holographic memory stores distributed signatures and retrieves by resonance."
                            .into(),
                    turn_id: "t2".into(),
                    importance: 0.8,
                },
            ],
        };
        let count = bridge.process_conversation(&conversation);
        assert_eq!(count, 2);
        assert_eq!(bridge.store().len(), 2);
    }

    // -----------------------------------------------------------------------
    // Resonance / find-similar
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_similar_after_processing() {
        let mut bridge = HolographicConversationBridge::new("find-sim");

        // Conversation 1: about memory
        let turns1 = vec![
            ConversationTurn {
                role: "user".into(),
                content: "Tell me about memory management.".into(),
                turn_id: "a1".into(),
                importance: 0.7,
            },
            ConversationTurn {
                role: "assistant".into(),
                content: "Memory management handles allocation and deallocation.".into(),
                turn_id: "a2".into(),
                importance: 0.8,
            },
        ];
        for t in &turns1 {
            bridge.process_turn(t);
        }

        // Conversation 2: about networking (different role → different concept)
        bridge.process_turn(&ConversationTurn {
            role: "tool".into(),
            content: "What about networking protocols?".into(),
            turn_id: "b1".into(),
            importance: 0.6,
        });

        // Find similar to the memory conversation
        let similar = bridge.find_similar_for_turns(&turns1, 5);
        assert!(
            !similar.matches.is_empty(),
            "should find at least one match for memory turns"
        );

        // The memory conversation's own traces should be among matches
        let matched_ids: Vec<String> = similar.matches.iter().map(|m| m.trace.id.clone()).collect();
        assert!(
            matched_ids.contains(&"a1".to_string()),
            "turn a1 should match (same keywords)"
        );
        assert!(
            matched_ids.contains(&"a2".to_string()),
            "turn a2 should match (same keywords)"
        );

        // Networking turn uses role "tool" → concept "tool_result", which
        // is different from the memory conversation's concepts ("user_query",
        // "assistant_response"). Since concept bits differ and keywords
        // ("what", "about", "networking", "protocols") do not overlap with
        // memory keywords, there should be no resonance between them.
        assert!(
            !matched_ids.contains(&"b1".to_string()),
            "networking turn b1 should not match memory keyword query"
        );
    }

    #[test]
    fn test_find_similar_with_no_matches() {
        let mut bridge = HolographicConversationBridge::new("empty-sim");
        bridge.process_turn(&ConversationTurn {
            role: "user".into(),
            content: "Hello world.".into(),
            turn_id: "hw1".into(),
            importance: 0.5,
        });

        // Query with completely different keywords
        let query_turns = vec![ConversationTurn {
            role: "tool".into(),
            content: "Supercalifragilisticexpialidocious quantum flux capacitor.".into(),
            turn_id: "qt1".into(),
            importance: 0.5,
        }];

        let similar = bridge.find_similar_for_turns(&query_turns, 5);
        assert!(
            similar.matches.is_empty(),
            "no matches expected for unrelated keywords"
        );
    }

    // -----------------------------------------------------------------------
    // Save/load round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_and_load_round_trip() {
        let mut bridge = HolographicConversationBridge::new("save-load");
        bridge.process_turn(&ConversationTurn {
            role: "user".into(),
            content: "Test save and load.".into(),
            turn_id: "tl1".into(),
            importance: 0.5,
        });

        let tmp = std::env::temp_dir().join("holographic-bridge-test.json");
        let path = tmp.to_str().unwrap().to_owned();
        bridge.save_to_file(&path).unwrap();

        let loaded = HolographicConversationBridge::load_from_file("save-load", &path).unwrap();
        assert_eq!(loaded.store().len(), 1);
        let traces = loaded.store().list_traces("save-load");
        assert_eq!(traces[0].id, "tl1");
        assert!(traces[0].content_summary.contains("Test save and load"));

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // Distributed signature creation
    // -----------------------------------------------------------------------

    #[test]
    fn test_process_turn_creates_distributed_signature() {
        let mut bridge = HolographicConversationBridge::new("sig-test");
        bridge.process_turn(&ConversationTurn {
            role: "user".into(),
            content: "Explain distributed signatures.".into(),
            turn_id: "sig1".into(),
            importance: 0.7,
        });
        let traces = bridge.store().list_traces("sig-test");
        assert_eq!(traces.len(), 1);
        let sig = &traces[0].distributed_signature;
        assert!(
            !sig.symbolic_bits.is_empty(),
            "keywords should produce signature bits"
        );
        assert!(
            !sig.concept_bits.is_empty(),
            "concepts should produce signature bits"
        );
    }
}
