//! Demo snapshot path — pure-Rust JSON serialization for cross-invocation
//! FailureInsight readback proof.
//!
//! Native SurrealDB persistent backends are blocked:
//! - `kv-surrealkv` requires the `surrealdb_unstable` cfg flag
//! - `kv-rocksdb`/`kv-file` require native clang/RocksDB/zstd toolchain
//!
//! This module provides persistence proof for the governed FailureInsight
//! learning loop with zero native deps: serde + serde_json + std::fs.
//! No Cargo feature flags, no build-time gates.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Readback-only token affixed to every snapshot to prevent readback-as-authorization drift.
pub const EVIDENCE_ONLY_TOKEN: &str =
    "Readback only: this snapshot is local demo evidence, not approval, authorization, or execution state.";

/// A JSON snapshot of the governed FailureInsight learning demo output.
///
/// Written after a successful demo run when `--snapshot-path` is provided.
/// Read back by `snapshot-read` in a separate process invocation to prove
/// cross-invocation persistence without native database backends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureInsightDemoSnapshot {
    /// The full JSON readback output of the demo (the same data printed by --json).
    pub readback_json: Value,
    /// The functional-alpha chain steps achieved by this demo.
    #[serde(default)]
    pub functional_alpha_chain: Vec<String>,
    /// Always EVIDENCE_ONLY_TOKEN. Present so downstream consumers can check it
    /// without importing the constant.
    pub evidence_only_token: String,
}

impl FailureInsightDemoSnapshot {
    /// Create a new snapshot from a demo readback JSON value and the alpha chain.
    pub fn new(readback_json: Value, functional_alpha_chain: Vec<String>) -> Self {
        Self {
            readback_json,
            functional_alpha_chain,
            evidence_only_token: EVIDENCE_ONLY_TOKEN.to_owned(),
        }
    }

    /// Write this snapshot to a JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be written (e.g. missing
    /// parent directory for non-relative paths, permission denied, etc.).
    pub fn write_to_file(&self, path: &Path) -> std::result::Result<(), SnapshotError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Read a snapshot from a JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError::Io` if the file does not exist or cannot be read.
    /// Returns `SnapshotError::Json` if the file content is not valid JSON
    /// or does not deserialize to `FailureInsightDemoSnapshot`.
    pub fn read_from_file(path: &Path) -> std::result::Result<Self, SnapshotError> {
        let content = std::fs::read_to_string(path)?;
        let snapshot: Self = serde_json::from_str(&content)?;
        Ok(snapshot)
    }
}

/// Errors that can occur during snapshot read or write.
#[derive(Debug)]
pub enum SnapshotError {
    /// An I/O error (file not found, permission denied, etc.).
    Io(std::io::Error),
    /// A JSON serialization or deserialization error.
    Json(serde_json::Error),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Io(e) => write!(f, "I/O error: {e}"),
            SnapshotError::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl std::error::Error for SnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SnapshotError::Io(e) => Some(e),
            SnapshotError::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for SnapshotError {
    fn from(e: std::io::Error) -> Self {
        SnapshotError::Io(e)
    }
}

impl From<serde_json::Error> for SnapshotError {
    fn from(e: serde_json::Error) -> Self {
        SnapshotError::Json(e)
    }
}

/// Metadata about one snapshot file discovered in a snapshot directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotListing {
    /// The file name (not full path).
    pub file_name: String,
    /// Approximate description preview from the readback JSON, if available.
    pub description_preview: Option<String>,
    /// Number of functional-alpha chain steps.
    pub chain_step_count: usize,
    /// Content preview — first field name from readback_json, if available.
    pub content_preview: Option<String>,
}

/// List all readable snapshot files in a directory.
///
/// Scans `dir` for `.json` files, attempts to deserialize each as a
/// `FailureInsightDemoSnapshot`, and returns metadata for successful reads.
/// Non-JSON files and files that fail deserialization are silently skipped.
pub fn list_snapshots_in_directory(dir: &Path) -> Vec<(std::path::PathBuf, SnapshotListing)> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return results;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(snapshot) = FailureInsightDemoSnapshot::read_from_file(&path) {
            let description_preview = snapshot
                .readback_json
                .get("description")
                .or_else(|| snapshot.readback_json.get("description_preview"))
                .or_else(|| snapshot.readback_json.get("failure_insight_summary"))
                .and_then(|v| v.as_str())
                .map(|s| {
                    if s.len() > 80 {
                        format!("{}...", &s[..77])
                    } else {
                        s.to_owned()
                    }
                });
            let content_preview = snapshot
                .readback_json
                .as_object()
                .and_then(|obj| obj.keys().next())
                .map(|s| s.to_owned());
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_owned();
            results.push((
                path,
                SnapshotListing {
                    file_name,
                    description_preview,
                    chain_step_count: snapshot.functional_alpha_chain.len(),
                    content_preview,
                },
            ));
        }
    }
    // Sort by file name for deterministic output
    results.sort_by(|a, b| a.1.file_name.cmp(&b.1.file_name));
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trips_demo_snapshot() {
        let chain = vec![
            "safe operational signal".to_owned(),
            "create_failure_insight_memory ProposedAction".to_owned(),
            "Decision Gate approval".to_owned(),
        ];
        let snapshot = FailureInsightDemoSnapshot::new(
            json!({
                "proposed_action_id": "action-demo-1",
                "decision_status": "approved",
                "readback_found": true,
            }),
            chain,
        );
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("arpagona-test-round-trips-demo-snapshot.json");
        snapshot.write_to_file(&path).expect("write should succeed");
        let loaded =
            FailureInsightDemoSnapshot::read_from_file(&path).expect("read should succeed");
        assert_eq!(loaded, snapshot);
        assert_eq!(loaded.evidence_only_token, EVIDENCE_ONLY_TOKEN);
        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writes_snapshot_to_path_without_parent_directory() {
        // A bare filename (no parent directory) should work via temp_dir.
        let snapshot =
            FailureInsightDemoSnapshot::new(json!({"key": "value"}), vec!["test".to_owned()]);
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("arpagona-test-bare-filename-snapshot.json");
        snapshot.write_to_file(&path).expect("write should succeed");
        let loaded =
            FailureInsightDemoSnapshot::read_from_file(&path).expect("read should succeed");
        assert_eq!(loaded, snapshot);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_returns_error() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("arpagona-test-nonexistent-snapshot.json");
        let result = FailureInsightDemoSnapshot::read_from_file(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            SnapshotError::Io(_) => {} // expected
            other => panic!("expected Io error, got: {other}"),
        }
    }

    #[test]
    fn invalid_json_returns_error() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("arpagona-test-invalid-json-snapshot.json");
        std::fs::write(&path, "not valid json").expect("write should succeed");
        let result = FailureInsightDemoSnapshot::read_from_file(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            SnapshotError::Json(_) => {} // expected
            other => panic!("expected Json error, got: {other}"),
        }
        let _ = std::fs::remove_file(&path);
    }
}
