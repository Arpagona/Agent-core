//! File-based governance audit store for the MCP server.
//!
//! Persists governance decisions (Approved / Blocked / RequiresOverride) as
//! JSON-lines to a file on disk. Entries survive server restarts and can be
//! read back by the CLI for operator supervision.
//!
//! # Format
//!
//! Each line is a JSON-serialized [`McpGovernanceAuditRecord`] object.
//! Lines are appended to the file; the store is append-only.
//!
//! # Invariants
//!
//! - Never stores secrets, passwords, or credentials.
//! - Readback is evidence-only, not authorization.
//! - Malformed lines from older formats are silently skipped on load.

use arpagona_agent_core::AuditEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// A single audit record for an MCP governance decision.
///
/// Stores the governance outcome, the tool call context, a human-readable
/// summary, and the full `AuditEvent` for integration with the existing
/// audit system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpGovernanceAuditRecord {
    /// The governance outcome: "Approved", "Blocked", or "RequiresOverride".
    pub outcome: String,
    /// The name of the tool that was called.
    pub tool_name: String,
    /// The arguments passed to the tool.
    pub arguments: Value,
    /// Human-readable summary of why this decision was made.
    pub summary: String,
    /// When this decision was made.
    pub created_at: DateTime<Utc>,
    /// The full `AuditEvent` for integration with the existing audit system.
    pub audit_event: AuditEvent,
}

/// File-based store for MCP governance audit records.
///
/// Maintains an in-memory list for fast reads during the same server session,
/// and appends every record to a JSON-lines file for durable persistence.
pub struct McpGovernanceAuditStore {
    path: PathBuf,
    entries: Vec<McpGovernanceAuditRecord>,
}

impl McpGovernanceAuditStore {
    /// Create or load an audit store at the given file path.
    ///
    /// If the file already exists, existing entries are loaded from it
    /// (cross-invocation persistence). If the file does not exist, an
    /// empty store is created — the parent directory is created on the
    /// first `record()` call.
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let entries = if path.exists() {
            Self::load_entries(&path)?
        } else {
            Vec::new()
        };
        Ok(Self { path, entries })
    }

    /// Load all entries from an existing JSON-lines file.
    fn load_entries(path: &Path) -> io::Result<Vec<McpGovernanceAuditRecord>> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Silently skip unparseable lines (older format, corrupted entry).
            if let Ok(record) = serde_json::from_str::<McpGovernanceAuditRecord>(trimmed) {
                entries.push(record);
            }
        }
        Ok(entries)
    }

    /// Record a new governance audit entry.
    ///
    /// Appends to both the in-memory list and the file on disk.
    /// Creates the parent directory if it does not exist.
    pub fn record(&mut self, entry: McpGovernanceAuditRecord) -> io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // Serialize once
        let line = serde_json::to_string(&entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Append to file atomically per line
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")?;

        // Add to in-memory list
        self.entries.push(entry);

        Ok(())
    }

    /// Return the N most recent entries (newest first).
    pub fn recent(&self, limit: usize) -> Vec<&McpGovernanceAuditRecord> {
        self.entries.iter().rev().take(limit).collect()
    }

    /// Return all entries (oldest first).
    pub fn all(&self) -> &[McpGovernanceAuditRecord] {
        &self.entries
    }

    /// Number of entries in the store.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the store has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Path to the audit file on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::{
        ActorRef, AuditEventId, AuditEventType, DecisionId, ProposedActionId,
    };
    use chrono::Utc;
    use serde_json::json;

    fn make_test_record(tool_name: &str, outcome: &str) -> McpGovernanceAuditRecord {
        let audit_event = AuditEvent {
            id: AuditEventId::new(format!("mcp-audit-{tool_name}")),
            event_type: AuditEventType::DecisionCreated,
            actor: ActorRef::System,
            workspace_id: None,
            task_id: None,
            proposed_action_id: Some(ProposedActionId::new(format!("mcp-act-{tool_name}"))),
            decision_id: Some(DecisionId::new(format!("mcp-dec-{tool_name}"))),
            payload: json!({
                "causal_trace": {
                    "tool": tool_name,
                    "outcome": outcome,
                }
            }),
            created_at: Utc::now(),
        };
        McpGovernanceAuditRecord {
            outcome: outcome.to_owned(),
            tool_name: tool_name.to_owned(),
            arguments: json!({"path": "Cargo.toml"}),
            summary: format!("{outcome}: Read-only tool with ProposeToolUse permission"),
            created_at: Utc::now(),
            audit_event,
        }
    }

    #[test]
    fn test_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let store = McpGovernanceAuditStore::new(&path).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_record_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = McpGovernanceAuditStore::new(&path).unwrap();

        let record = make_test_record("read_file", "Approved");
        store.record(record).unwrap();

        assert_eq!(store.len(), 1);
        let recent = store.recent(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].outcome, "Approved");
        assert_eq!(recent[0].tool_name, "read_file");
        assert_eq!(
            recent[0].audit_event.event_type,
            AuditEventType::DecisionCreated
        );
    }

    #[test]
    fn test_persistence_across_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        // First session: record entries
        {
            let mut store = McpGovernanceAuditStore::new(&path).unwrap();
            assert!(store.is_empty());
            store
                .record(make_test_record("read_file", "Approved"))
                .unwrap();
            store
                .record(make_test_record("list_files", "Approved"))
                .unwrap();
            assert_eq!(store.len(), 2);
        } // store dropped, file persists

        // Second session: load from same file
        {
            let store = McpGovernanceAuditStore::new(&path).unwrap();
            assert_eq!(store.len(), 2, "Should have loaded persisted entries");
            assert_eq!(store.recent(1)[0].tool_name, "list_files");
            assert_eq!(store.all()[0].tool_name, "read_file");
        }
    }

    #[test]
    fn test_multiple_records_and_recent_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = McpGovernanceAuditStore::new(&path).unwrap();

        for i in 0..10 {
            let mut record = make_test_record("read_file", "Approved");
            record.tool_name = format!("tool_{i}");
            store.record(record).unwrap();
        }

        assert_eq!(store.len(), 10);
        let recent = store.recent(3);
        assert_eq!(recent.len(), 3);
        // Newest first
        assert_eq!(recent[0].tool_name, "tool_9");
        assert_eq!(recent[2].tool_name, "tool_7");

        // If limit > entries, return all
        let recent_all = store.recent(100);
        assert_eq!(recent_all.len(), 10);
    }

    #[test]
    fn test_blocked_and_override_outcomes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut store = McpGovernanceAuditStore::new(&path).unwrap();

        store
            .record(make_test_record("read_file", "Approved"))
            .unwrap();
        store
            .record(make_test_record("write_data", "Blocked"))
            .unwrap();
        store
            .record(make_test_record("delete_file", "RequiresOverride"))
            .unwrap();

        let recent = store.recent(10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].outcome, "RequiresOverride");
        assert_eq!(recent[1].outcome, "Blocked");
        assert_eq!(recent[2].outcome, "Approved");
    }

    #[test]
    fn test_store_has_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let store = McpGovernanceAuditStore::new(&path).unwrap();
        assert_eq!(store.path(), &path);
    }
}
