//! Cross-process CLI integration test for the governed FailureInsight demo snapshot path.
//!
//! Proves that the FailureInsight demo output survives serialization, file I/O,
//! process restart and deserialization via `std::process::Command`.

/// Path to the compiled arpagona binary, resolved by Cargo at compile time.
/// Works in CI and on developer machines without hardcoded target/debug paths.
const ARPAGONA_BIN: &str = env!("CARGO_BIN_EXE_arpagona");

#[test]
fn cross_invocation_demo_snapshot_proves_readback_across_process_invocations() {
    let dir = std::env::temp_dir().join("arpagona-snapshot-integration-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let snapshot_path = dir.join("snapshot.json");
    let snapshot_str = snapshot_path.to_str().expect("valid utf-8 path").to_owned();

    // Step 1: run demo with --snapshot-path to produce the snapshot file
    let output1 = std::process::Command::new(ARPAGONA_BIN)
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

    // Verify the snapshot file was created
    assert!(snapshot_path.exists(), "snapshot file was not created");

    // Step 2: read the snapshot back in a separate process
    let output2 = std::process::Command::new(ARPAGONA_BIN)
        .args(["memory", "demo", "snapshot-read", &snapshot_str, "--json"])
        .output()
        .expect("failed to run snapshot read process");
    assert!(
        output2.status.success(),
        "snapshot read command failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output2.stdout),
        String::from_utf8_lossy(&output2.stderr),
    );

    // Verify readback contains expected fields
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
        stdout.contains("approved local Graph Memory persistence"),
        "readback should contain the persistence step\n{}",
        stdout
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cross_invocation_description_survives_snapshot_path_across_processes() {
    // Proves that operator-supplied --description text survives through the
    // demo snapshot path across separate process invocations.
    let dir = std::env::temp_dir().join("arpagona-description-snapshot-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let snapshot_path = dir.join("description-snapshot.json");
    let snapshot_str = snapshot_path.to_str().expect("valid utf-8 path").to_owned();
    let custom_desc = "cross-invocation description propagation";

    // Step 1: run demo with --description and --snapshot-path
    let output1 = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "memory",
            "demo",
            "failure-insight",
            "--description",
            custom_desc,
            "--json",
            "--snapshot-path",
            &snapshot_str,
        ])
        .output()
        .expect("failed to run demo with description and snapshot path");
    assert!(
        output1.status.success(),
        "demo with description failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output1.stdout),
        String::from_utf8_lossy(&output1.stderr),
    );
    assert!(snapshot_path.exists(), "snapshot file was not created");

    // Step 2: read the snapshot back in a separate process
    let output2 = std::process::Command::new(ARPAGONA_BIN)
        .args(["memory", "demo", "snapshot-read", &snapshot_str, "--json"])
        .output()
        .expect("failed to run snapshot read process");
    assert!(
        output2.status.success(),
        "snapshot read failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output2.stdout),
        String::from_utf8_lossy(&output2.stderr),
    );

    // Verify the custom description appears in the readback across process invocations
    let stdout = String::from_utf8(output2.stdout).expect("valid utf-8");
    assert!(
        stdout.contains(custom_desc),
        "snapshot readback should contain the custom description '{}' but stdout was:\n{}",
        custom_desc,
        stdout,
    );
    assert!(
        stdout.contains("evidence_only_token"),
        "readback should contain evidence_only_token"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_read_reports_missing_file_error() {
    let missing_path = std::env::temp_dir().join("arpagona-test-nonexistent-snapshot-read.json");

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "memory",
            "demo",
            "snapshot-read",
            &missing_path.to_string_lossy(),
        ])
        .output()
        .expect("failed to run snapshot read process");

    assert!(
        !output.status.success(),
        "snapshot-read on nonexistent file should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to read snapshot"),
        "stderr should contain error message, got: {}",
        stderr
    );
}
