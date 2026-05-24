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
    pub evidence_only_token: String,
}

impl FailureInsightDemoSnapshot {
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
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
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
        }
    }
}

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
    }

    #[test]
    fn writes_snapshot_to_path_without_parent_directory() {
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
    }

    #[test]
    fn missing_file_returns_error() {
        let result = read_failure_insight_demo_snapshot("/nonexistent/path/snapshot.json");
        assert!(result.is_err());
        match result {
            Err(DemoSnapshotError::Io(_)) => {} // expected
            _ => panic!("expected I/O error for missing file"),
        }
    }

    #[test]
    fn invalid_json_returns_error() {
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
    }
}
