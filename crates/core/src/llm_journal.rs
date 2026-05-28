//! LLM interaction journaling for audit and operator readback (Track C Step C3).
//!
//! Captures prompt summaries, response summaries, provider/model metadata,
//! proposed actions, tool-call intents, Decision Gate outcomes and risk levels
//! for every LLM interaction.
//!
//! # Safety
//! - Journals store prompt/response *summaries*, not raw secrets.
//! - Journaled data is evidence/debugging-only, never authorization.
//! - The journal persists to a JSON-lines file for cross-invocation readback.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::RiskLevel;

/// What kind of LLM interaction was journaled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmInteractionType {
    /// Cognitive synthesis (free-form enrichment of working memory).
    Synthesis,
    /// A tool-call intent produced by the LLM but not yet governed/executed.
    ToolCallIntent,
    /// A direct tool-call that went through governance and execution.
    DirectToolCall,
}

/// A single journaled LLM interaction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlmJournalEntry {
    /// Unique journal entry ID.
    pub id: String,
    /// When the interaction occurred.
    pub created_at: DateTime<Utc>,
    /// What kind of interaction.
    pub interaction_type: LlmInteractionType,
    /// Summary of the prompt sent to the LLM (not the raw prompt — avoid secrets).
    pub prompt_summary: String,
    /// Summary of the response from the LLM.
    pub response_summary: String,
    /// The provider used (e.g., "mock", "openai", "ollama").
    pub provider: String,
    /// The model used, if known.
    pub model: Option<String>,
    /// The original objective or query, if applicable.
    pub objective: Option<String>,
    /// Any proposed actions from this interaction (JSON summaries, not raw types).
    pub proposed_actions: Option<Value>,
    /// Any tool-call intents from this interaction.
    pub tool_call_intents: Option<Value>,
    /// Decision Gate outcomes related to this interaction.
    pub decision_gate_outcomes: Option<Value>,
    /// Risk level, if applicable.
    pub risk_level: Option<RiskLevel>,
    /// Compute Reservoir routing details: why this provider/model was chosen,
    /// including justification, cost/latency/risk trade-offs.
    pub compute_routing: Option<Value>,
}

/// In-memory ring-buffer for LLM interaction journal entries.
///
/// Stores recent entries for operator readback and debugging.
/// Persists to a JSON-lines file for cross-invocation readback.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmJournal {
    entries: Vec<LlmJournalEntry>,
    max_entries: usize,
    next_id: u64,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl LlmJournal {
    /// Create a new journal with the given capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            max_entries,
            next_id: 1,
            path: None,
        }
    }

    /// Create a journal with file persistence at the given path.
    /// If the file exists, existing entries are loaded from it.
    pub fn with_file(max_entries: usize, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut journal = if path.exists() {
            Self::load_from_file(&path).unwrap_or_else(|_| Self::new(max_entries))
        } else {
            Self::new(max_entries)
        };
        journal.max_entries = max_entries;
        journal.path = Some(path);
        journal
    }

    /// Load journal entries from a JSON-lines file.
    pub fn load_from_file(path: &Path) -> Result<Self, io::Error> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut entries = Vec::new();
        let mut next_id = 1u64;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<LlmJournalEntry>(&line) {
                // Extract numeric ID for next_id tracking
                if let Some(num) = entry.id.strip_prefix("llm-journal-") {
                    if let Ok(n) = num.parse::<u64>() {
                        if n >= next_id {
                            next_id = n + 1;
                        }
                    }
                }
                entries.push(entry);
            }
        }

        let entry_count = entries.len();
        Ok(Self {
            entries,
            max_entries: entry_count.max(100),
            next_id,
            path: Some(path.to_path_buf()),
        })
    }

    /// Append a single entry to the JSON-lines file.
    fn append_to_file(&self, entry: &LlmJournalEntry) {
        if let Some(ref path) = self.path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let line = serde_json::to_string(entry).unwrap_or_default();
                let _ = writeln!(file, "{line}");
            }
        }
    }

    /// Add an entry to the journal. If at capacity, the oldest entry is removed.
    /// If a file path is configured, the entry is appended to the JSON-lines file.
    pub fn add_entry(&mut self, entry: LlmJournalEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.append_to_file(&entry);
        self.entries.push(entry);
        self.next_id += 1;
    }

    /// Convenience: build and add a synthesis entry.
    ///
    /// Returns the generated entry ID.
    pub fn add_synthesis(
        &mut self,
        objective: &str,
        provider: &str,
        model: Option<String>,
        prompt_summary: String,
        response_summary: String,
    ) -> String {
        self.add_synthesis_with_routing(
            objective,
            provider,
            model,
            prompt_summary,
            response_summary,
            None,
        )
    }

    /// Convenience: build and add a synthesis entry with optional Compute Reservoir routing info.
    ///
    /// Returns the generated entry ID.
    pub fn add_synthesis_with_routing(
        &mut self,
        objective: &str,
        provider: &str,
        model: Option<String>,
        prompt_summary: String,
        response_summary: String,
        compute_routing: Option<Value>,
    ) -> String {
        let id = format!("llm-journal-{}", self.next_id);
        let entry = LlmJournalEntry {
            id: id.clone(),
            created_at: Utc::now(),
            interaction_type: LlmInteractionType::Synthesis,
            prompt_summary,
            response_summary,
            provider: provider.to_owned(),
            model,
            objective: Some(objective.to_owned()),
            proposed_actions: None,
            tool_call_intents: None,
            decision_gate_outcomes: None,
            risk_level: None,
            compute_routing,
        };
        self.add_entry(entry);
        id
    }

    /// Convenience: build and add a standalone compute routing entry (C4).
    ///
    /// Records how Compute Reservoir would route a cognitive task, including
    /// the selected node, resolved provider, and cost/latency/privacy trade-offs.
    /// The routing is a routing preview, not a resource lease or action approval.
    pub fn add_compute_routing(
        &mut self,
        purpose: &str,
        provider: &str,
        routing_details: Value,
    ) -> String {
        let id = format!("llm-journal-{}", self.next_id);
        let prompt_summary = routing_details
            .get("purpose")
            .and_then(|v| v.as_str())
            .unwrap_or(purpose);
        let entry = LlmJournalEntry {
            id: id.clone(),
            created_at: Utc::now(),
            interaction_type: LlmInteractionType::Synthesis,
            prompt_summary: format!("Compute routing preview: purpose={}", prompt_summary),
            response_summary: format!(
                "Routed via provider={}, node={:?}",
                provider,
                routing_details
                    .get("selected_node_id")
                    .and_then(|v| v.as_str()),
            ),
            provider: "compute-reservoir".to_owned(),
            model: None,
            objective: Some(purpose.to_owned()),
            proposed_actions: None,
            tool_call_intents: None,
            decision_gate_outcomes: None,
            risk_level: None,
            compute_routing: Some(routing_details),
        };
        self.add_entry(entry);
        id
    }

    /// Convenience: build and add a direct tool-call entry.
    ///
    /// Returns the generated entry ID.
    pub fn add_direct_tool_call(
        &mut self,
        tool: &str,
        provider: &str,
        model: Option<String>,
        prompt_summary: String,
        response_summary: String,
        tool_call_intents: Value,
        decision_gate_outcome: Value,
        risk_level: Option<RiskLevel>,
    ) -> String {
        let id = format!("llm-journal-{}", self.next_id);
        let entry = LlmJournalEntry {
            id: id.clone(),
            created_at: Utc::now(),
            interaction_type: LlmInteractionType::DirectToolCall,
            prompt_summary,
            response_summary,
            provider: provider.to_owned(),
            model,
            objective: Some(tool.to_owned()),
            proposed_actions: None,
            tool_call_intents: Some(tool_call_intents),
            decision_gate_outcomes: Some(decision_gate_outcome),
            risk_level,
            compute_routing: None,
        };
        self.add_entry(entry);
        id
    }

    /// Return all entries (most recent last).
    pub fn all_entries(&self) -> &[LlmJournalEntry] {
        &self.entries
    }

    /// Return the N most recent entries (most recent first).
    pub fn recent_entries(&self, n: usize) -> Vec<&LlmJournalEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// Get a single entry by ID.
    pub fn get_entry(&self, id: &str) -> Option<&LlmJournalEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for LlmJournal {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_journal_is_empty() {
        let journal = LlmJournal::new(10);
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
    }

    #[test]
    fn add_synthesis_creates_entry() {
        let mut journal = LlmJournal::new(10);
        let id = journal.add_synthesis(
            "Test objective",
            "mock",
            None,
            "Prompt about test".to_owned(),
            "Synthesis result".to_owned(),
        );
        assert_eq!(journal.len(), 1);
        let entry = journal.get_entry(&id).expect("entry should exist");
        assert_eq!(entry.provider, "mock");
        assert_eq!(entry.interaction_type, LlmInteractionType::Synthesis);
        assert_eq!(entry.objective.as_deref(), Some("Test objective"));
    }

    #[test]
    fn journal_respects_capacity() {
        let mut journal = LlmJournal::new(3);
        for i in 0..5 {
            journal.add_synthesis(
                &format!("Objective {}", i),
                "mock",
                None,
                "prompt".to_owned(),
                "response".to_owned(),
            );
        }
        assert_eq!(journal.len(), 3);
        // The first two entries should have been evicted
        let entries = journal.all_entries();
        assert_eq!(entries[0].objective.as_deref(), Some("Objective 2"));
        assert_eq!(entries[2].objective.as_deref(), Some("Objective 4"));
    }

    #[test]
    fn recent_entries_returns_most_recent_first() {
        let mut journal = LlmJournal::new(10);
        for i in 0..5 {
            journal.add_synthesis(
                &format!("Obj {}", i),
                "mock",
                None,
                "prompt".to_owned(),
                "response".to_owned(),
            );
        }
        let recent = journal.recent_entries(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].objective.as_deref(), Some("Obj 4"));
        assert_eq!(recent[2].objective.as_deref(), Some("Obj 2"));
    }

    #[test]
    fn get_entry_returns_none_for_unknown_id() {
        let journal = LlmJournal::new(10);
        assert!(journal.get_entry("nonexistent").is_none());
    }

    #[test]
    fn add_direct_tool_call_creates_entry_with_governance_data() {
        let mut journal = LlmJournal::new(10);
        let id = journal.add_direct_tool_call(
            "read_file",
            "mock",
            None,
            "LLM requested read_file".to_owned(),
            "Tool executed successfully".to_owned(),
            serde_json::json!({"tool": "read_file", "path": "test.txt"}),
            serde_json::json!({"status": "approved", "risk": "low"}),
            Some(RiskLevel::Low),
        );
        let entry = journal.get_entry(&id).expect("entry should exist");
        assert_eq!(entry.interaction_type, LlmInteractionType::DirectToolCall);
        assert!(entry.tool_call_intents.is_some());
        assert!(entry.decision_gate_outcomes.is_some());
        assert_eq!(entry.risk_level, Some(RiskLevel::Low));
    }

    #[test]
    fn add_synthesis_with_routing_stores_compute_routing() {
        let mut journal = LlmJournal::new(10);
        let routing = serde_json::json!({
            "selected_node_id": "local-smol",
            "resource_kind": "local_llm",
            "expected_cost_cents": 0,
            "expected_latency_ms": 800,
            "justification": "Low complexity task, local-first preference",
        });
        let id = journal.add_synthesis_with_routing(
            "Test objective",
            "ollama",
            Some("qwen3.5:9b".to_owned()),
            "Prompt about test".to_owned(),
            "Synthesis result".to_owned(),
            Some(routing.clone()),
        );
        assert_eq!(journal.len(), 1);
        let entry = journal.get_entry(&id).expect("entry should exist");
        assert_eq!(entry.provider, "ollama");
        assert_eq!(entry.model.as_deref(), Some("qwen3.5:9b"));
        let stored_routing = entry
            .compute_routing
            .as_ref()
            .expect("compute_routing should be set");
        assert_eq!(
            stored_routing["selected_node_id"], "local-smol",
            "routing should contain selected_node_id"
        );
        assert_eq!(
            stored_routing["justification"], "Low complexity task, local-first preference",
            "routing should contain justification"
        );
    }

    #[test]
    fn synthesis_without_routing_has_none() {
        let mut journal = LlmJournal::new(10);
        journal.add_synthesis("obj", "mock", None, "prompt".into(), "response".into());
        let entry = journal.all_entries().last().expect("entry should exist");
        assert!(
            entry.compute_routing.is_none(),
            "entry without routing should have compute_routing = None"
        );
    }

    #[test]
    fn llm_journal_entry_serializes_and_deserializes() {
        let entry = LlmJournalEntry {
            id: "test-entry".to_owned(),
            created_at: Utc::now(),
            interaction_type: LlmInteractionType::Synthesis,
            prompt_summary: "prompt".to_owned(),
            response_summary: "response".to_owned(),
            provider: "mock".to_owned(),
            model: None,
            objective: Some("obj".to_owned()),
            proposed_actions: None,
            tool_call_intents: None,
            decision_gate_outcomes: None,
            risk_level: Some(RiskLevel::Low),
            compute_routing: None,
        };
        let json = serde_json::to_string(&entry).expect("should serialize");
        let decoded: LlmJournalEntry = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(decoded.id, "test-entry");
        assert_eq!(decoded.risk_level, Some(RiskLevel::Low));
    }
}
