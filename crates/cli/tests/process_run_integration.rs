//! Integration tests for the `arpagona process run` command.
//!
//! Proves that doctor fail-severity checks correctly block the process
//! at step 1, while warn-only checks are non-blocking.

/// Path to the compiled arpagona binary, resolved by Cargo at compile time.
const ARPAGONA_BIN: &str = env!("CARGO_BIN_EXE_arpagona");

/// Set OLLAMA_ENDPOINT to a dead address (port 9) to force fail-severity
/// checks (ollama, qwen3.5:9b_model) and verify that `process run`
/// blocks at step 1 with overall_status=BLOCKED — does NOT continue to
/// cargo_fmt, cargo_check, or cargo_test.
#[test]
fn process_run_blocks_at_step_1_when_doctor_has_blocking_failures() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .output()
        .expect("failed to run process run daily-validation --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must contain the BLOCKED summary — not PASSED
    assert!(
        stdout.contains("\"overall_status\": \"BLOCKED\""),
        "process run should end with BLOCKED when doctor has blocking failures.\nstdout:\n{}",
        stdout
    );

    // Must block at step 1 (doctor/preflight)
    assert!(
        stdout.contains("\"blocked_at_step\": 1"),
        "process run should block at step 1 (doctor), not later.\nstdout:\n{}",
        stdout
    );

    // Must NOT contain a PASSED summary
    assert!(
        !stdout.contains("\"overall_status\": \"PASSED\""),
        "process run must NOT report PASSED when blocking failures exist.\nstdout:\n{}",
        stdout
    );

    // The doctor step itself must have status FAILED
    assert!(
        stdout.contains("\"status\": \"FAILED\"") && stdout.contains("\"step\": 1"),
        "doctor step should be reported as FAILED.\nstdout:\n{}",
        stdout
    );
}

/// Run doctor in --json mode with a dead Ollama endpoint.
/// Verifies that doctor exits with non-zero exit code and emits
/// `all_pass: false` when fail-severity checks fail.
#[test]
fn doctor_returns_error_on_blocking_failures() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["doctor", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .output()
        .expect("failed to run doctor --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Doctor must exit with non-zero status when fail-severity checks fail
    assert!(
        !output.status.success(),
        "doctor should exit with non-zero status when blocking failures exist.\nstdout:\n{}",
        stdout
    );

    // JSON output must show all_pass: false
    assert!(
        stdout.contains("\"all_pass\": false"),
        "doctor JSON should show all_pass: false when blockers exist.\nstdout:\n{}",
        stdout
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("blocker") || stderr.contains("Doctor found blocker"),
        "doctor stderr should contain blocker message.\nstderr:\n{}",
        stderr
    );
}

/// Run doctor pointed at dead port 1 of localhost (connection refused,
/// not timeout) so the fail-severity ollama check fires immediately
/// without waiting for a 5-second timeout.
#[test]
fn doctor_returns_blocking_failures_via_connection_refused() {
    // Port 1 is privileged and not in use — connection refused is instant.
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["doctor", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:1")
        .output()
        .expect("failed to run doctor --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must complete (not hang) and produce valid JSON
    assert!(
        stdout.contains("\"all_pass\""),
        "doctor should complete with valid JSON output even when Ollama is unreachable.\nstdout:\n{}",
        stdout
    );

    // all_pass should be false (ollama/qwen checks fail)
    assert!(
        stdout.contains("\"all_pass\": false"),
        "all_pass should be false when Ollama is unreachable.\nstdout:\n{}",
        stdout
    );

    // The ollama check should show reachable: NO
    assert!(
        stdout.contains("reachable: NO"),
        "doctor should report Ollama as unreachable.\nstdout:\n{}",
        stdout
    );

    // Should exit with non-zero status
    assert!(
        !output.status.success(),
        "doctor should exit with non-zero status when Ollama is unreachable.\nstdout:\n{}",
        stdout
    );
}
