//! Integration test for cross-invocation demo snapshot readback.
//!
//! Runs the full snapshot-then-read cycle via separate process invocations
//! using the built `arpagona` binary (resolved by Cargo's
//! `CARGO_BIN_EXE_arpagona` env var).  Proves the governed FailureInsight
//! learning loop output survives serialization, file I/O, process restart
//! and deserialization.
//!
//! This is deliberately kept in `tests/` rather than `src/` so Cargo
//! provides the stable `CARGO_BIN_EXE_arpagona` path instead of a
//! hardcoded relative path like `../../target/debug/arpagona`.

use std::path::PathBuf;
use std::process::Command;

/// Path to the built `arpagona` binary, resolved by Cargo at compile time.
const ARPAGONA_BIN: &str = env!("CARGO_BIN_EXE_arpagona");

#[test]
fn cross_invocation_demo_snapshot_proves_readback_across_process_invocations() {
    let dir = std::env::temp_dir().join("arpagona-snapshot-integration-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let snapshot_path = dir.join("snapshot.json");
    let snapshot_str = snapshot_path.to_str().expect("valid utf-8 path").to_owned();

    let bin_path = PathBuf::from(ARPAGONA_BIN);

    // Step 1: run demo with --snapshot-path to produce the snapshot file
    let output1 = Command::new(&bin_path)
        .args([
            "memory",
            "demo",
            "failure-insight",
            "--json",
            "--snapshot-path",
            &snapshot_str,
        ])
        .output()
        .expect("failed to run demo snapshot process");
    assert!(
        output1.status.success(),
        "demo snapshot command failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output1.stdout),
        String::from_utf8_lossy(&output1.stderr),
    );
    assert!(snapshot_path.exists(), "snapshot file was not created");

    // Step 2: read the snapshot back in a separate process
    let output2 = Command::new(&bin_path)
        .args(["memory", "demo", "snapshot-read", &snapshot_str, "--json"])
        .output()
        .expect("failed to run snapshot read process");
    assert!(
        output2.status.success(),
        "snapshot read command failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output2.stdout),
        String::from_utf8_lossy(&output2.stderr),
    );

    let stdout = String::from_utf8(output2.stdout).expect("valid utf-8");
    assert!(
        stdout.contains("evidence_only_token"),
        "readback should contain evidence_only_token\n{}",
        stdout
    );
    assert!(
        stdout.contains("Readback only"),
        "readback should contain evidence_only_token text\n{}",
        stdout
    );
    assert!(
        stdout.contains("functional_alpha_chain"),
        "readback should contain functional_alpha_chain\n{}",
        stdout
    );
    assert!(
        stdout.contains("demo snapshot written for cross-invocation readback proof"),
        "readback should contain the snapshot step in the chain\n{}",
        stdout
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}
