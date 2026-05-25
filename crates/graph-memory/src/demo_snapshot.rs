<<<<<<< HEAD
//! Demo snapshot persistence for the FailureInsight governed learning loop.
//!
//! This provides a pure-Rust, file-based snapshot/readback path so the
//! local alpha demo can prove readback across separate process invocations,
//! without requiring native SurrealDB backends (kv-rocksdb/kv-file/zstd-sys)
//! or unstable cfg flags (surrealdb_unstable).
//!
//! The snapshot file is a JSON encoding of the demo readback state.
//! It is not a production persistence mechanism, not an action authorization
//! path, and not a replacement for real Graph Memory persistence. It is a
//! development-only proof that the governed learning loop output can survive
//! a separate process run.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A snapshot of the FailureInsight demo readback state, suitable for
/// JSON file persistence and cross-invocation inspection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailureInsightDemoSnapshot {
    /// The serialized readback payload.
    pub readback_json: serde_json::Value,
    /// Human-readable functional alpha chain steps.
    pub functional_alpha_chain: Vec<String>,
    /// A token indicating the demo ran to completion and the snapshot is
    /// evidence, not authorization.
=======
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
>>>>>>> origin/main
    pub evidence_only_token: String,
}

impl FailureInsightDemoSnapshot {
<<<<<<< HEAD
    /// Create a new demo snapshot from the readback JSON value.
    pub fn new(readback_json: serde_json::Value) -> Self {
        Self {
            readback_json,
            functional_alpha_chain: vec![
                "safe operational signal".to_owned(),
                "create_failure_insight_memory ProposedAction".to_owned(),
                "Decision Gate approval".to_owned(),
                "decision audit event".to_owned(),
                "approved local Graph Memory persistence".to_owned(),
                "FailureInsight readback with decision/audit trace proof".to_owned(),
                "demo snapshot written for cross-invocation readback proof".to_owned(),
            ],
            evidence_only_token: "Readback only: this snapshot is local demo evidence, not approval, authorization, or execution state.".to_owned(),
        }
    }

    /// Write the snapshot to the given file path as pretty-printed JSON.
    ///
    /// Creates parent directories if they do not exist.
    /// Paths without a parent directory (e.g. `snapshot.json`) are written
    /// directly to the current working directory without any `create_dir_all` call.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), DemoSnapshotError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
=======
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
>>>>>>> origin/main
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
<<<<<<< HEAD
}

/// Error type for demo snapshot operations.
#[derive(Debug)]
pub enum DemoSnapshotError {
    /// I/O error during read or write.
    Io(std::io::Error),
    /// JSON serialization error.
    Serialization(serde_json::Error),
}

impl std::fmt::Display for DemoSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DemoSnapshotError::Io(err) => write!(f, "demo snapshot I/O error: {err}"),
            DemoSnapshotError::Serialization(err) => {
                write!(f, "demo snapshot serialization error: {err}")
            }
=======

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
>>>>>>> origin/main
        }
    }
}

<<<<<<< HEAD
impl std::error::Error for DemoSnapshotError {}

impl From<std::io::Error> for DemoSnapshotError {
    fn from(err: std::io::Error) -> Self {
        DemoSnapshotError::Io(err)
    }
}

impl From<serde_json::Error> for DemoSnapshotError {
    fn from(err: serde_json::Error) -> Self {
        DemoSnapshotError::Serialization(err)
    }
}

/// Read a FailureInsight demo snapshot from a JSON file.
pub fn read_failure_insight_demo_snapshot(
    path: impl AsRef<Path>,
) -> Result<FailureInsightDemoSnapshot, DemoSnapshotError> {
    let json = std::fs::read_to_string(path.as_ref())?;
    let snapshot: FailureInsightDemoSnapshot = serde_json::from_str(&json)?;
    Ok(snapshot)
=======
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
>>>>>>> origin/main
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
<<<<<<< HEAD
    use std::fs;

    #[test]
    fn round_trips_demo_snapshot() {
        let dir = std::env::temp_dir().join("arpagona-demo-snapshot-test");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("snapshot.json");

        let snapshot = FailureInsightDemoSnapshot::new(json!({
            "signal_type": "runtime_observation",
            "proposed_action_id": "action-demo-1",
            "memory_write_kind": "create_failure_insight_memory",
            "decision_status": "Approved",
            "decision_reason": "Low risk local demo",
            "readback_found": true,
            "readback_audit_event_count": 1,
            "readback_relation_count": 2,
        }));

        snapshot.write_to_file(&path).expect("write should succeed");

        let loaded = read_failure_insight_demo_snapshot(&path).expect("read should succeed");
        assert_eq!(loaded.readback_json["decision_status"], "Approved");
        assert!(loaded.readback_json["readback_found"].as_bool().unwrap());
        assert_eq!(
            loaded.readback_json["readback_audit_event_count"]
                .as_u64()
                .unwrap(),
            1
        );
        assert!(
            loaded.evidence_only_token.contains("Readback only"),
            "evidence-only token should be present"
        );
        assert!(
            loaded
                .functional_alpha_chain
                .contains(&"demo snapshot written for cross-invocation readback proof".to_owned()),
            "snapshot step should be in the chain"
        );

        let _ = fs::remove_dir_all(&dir);
=======

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
>>>>>>> origin/main
    }

    #[test]
    fn writes_snapshot_to_path_without_parent_directory() {
<<<<<<< HEAD
        // A bare filename like "bare.json" has parent() == Some("")
        // where as_os_str().is_empty() is true.  write_to_file must
        // skip the create_dir_all call in that case.
        let original_cwd = std::env::current_dir().expect("get current dir");
        let dir = std::env::temp_dir().join("arpagona-demo-bare-json-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        std::env::set_current_dir(&dir).expect("change to temp dir");

        let snapshot = FailureInsightDemoSnapshot::new(json!({
            "test": "bare_filename_path",
        }));

        snapshot
            .write_to_file("bare.json")
            .expect("write with bare filename should succeed");

        let loaded = read_failure_insight_demo_snapshot("bare.json").expect("read should succeed");
        assert_eq!(loaded.readback_json["test"], "bare_filename_path");

        let _ = fs::remove_dir_all(&dir);
        std::env::set_current_dir(&original_cwd).expect("restore original cwd");
=======
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
>>>>>>> origin/main
    }

    #[test]
    fn missing_file_returns_error() {
<<<<<<< HEAD
        let result = read_failure_insight_demo_snapshot("/nonexistent/path/snapshot.json");
        assert!(result.is_err());
        match result {
            Err(DemoSnapshotError::Io(_)) => {} // expected
            _ => panic!("expected I/O error for missing file"),
=======
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("arpagona-test-nonexistent-snapshot.json");
        let result = FailureInsightDemoSnapshot::read_from_file(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            SnapshotError::Io(_) => {} // expected
            other => panic!("expected Io error, got: {other}"),
>>>>>>> origin/main
        }
    }

    #[test]
    fn invalid_json_returns_error() {
<<<<<<< HEAD
        let dir = std::env::temp_dir().join("arpagona-demo-invalid-json-test");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("bad.json");
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(&path, "not valid json").expect("write test file");

        let result = read_failure_insight_demo_snapshot(&path);
        assert!(result.is_err());
        match result {
            Err(DemoSnapshotError::Serialization(_)) => {} // expected
            _ => panic!("expected serialization error for invalid JSON"),
        }

        let _ = fs::remove_dir_all(&dir);
=======
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
>>>>>>> origin/main
    }
}
