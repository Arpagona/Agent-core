//! Integration tests for the `arpagona process run` command.
//!
//! Proves that doctor fail-severity checks correctly block the process
//! at step 1, while warn-only checks are non-blocking.
//!
//! Also verifies process journal creation and status readback.

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

/// Process run JSON output must include a `run_id` field in plan and summary phases.
#[test]
fn process_run_json_includes_run_id() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .output()
        .expect("failed to run process run daily-validation --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plan phase must include run_id
    assert!(
        stdout.contains("\"run_id\": \"daily-validation-"),
        "JSON output should contain a run_id in the plan phase.\nstdout:\n{}",
        stdout
    );

    // Summary phase must include run_id
    assert!(
        stdout.contains("\"run_id\""),
        "JSON output should contain run_id in the summary phase.\nstdout:\n{}",
        stdout
    );
}

/// Process run creates a durable journal file for inspection.
#[test]
fn process_run_creates_journal_file() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .output()
        .expect("failed to run process run daily-validation --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract the run_id from the JSON output (it appears in the plan phase)
    let run_id_marker = "\"run_id\": \"daily-validation-";
    if let Some(start) = stdout.find(run_id_marker) {
        let after_prefix = &stdout[start + run_id_marker.len()..];
        if let Some(end) = after_prefix.find('"') {
            let run_id = format!("daily-validation-{}", &after_prefix[..end]);

            // Check that the journal file exists
            let home =
                std::env::home_dir().expect("home_dir should be available in test environment");
            let journal_path = home
                .join(".arpagona")
                .join("process-journal")
                .join(format!("{}.json", run_id));

            assert!(
                journal_path.exists(),
                "Journal file should exist at: {:?}\nrun_id: {}",
                journal_path,
                run_id
            );

            // Read and verify the journal content
            let content = std::fs::read_to_string(&journal_path).expect("should read journal file");
            assert!(
                content.contains(&run_id),
                "Journal should contain the run_id"
            );
            assert!(
                content.contains("\"overall_status\": \"BLOCKED\""),
                "Journal should contain overall_status BLOCKED"
            );
            assert!(
                content.contains("\"next_action\""),
                "Journal should contain next_action"
            );
        }
    }
}

/// Process status --last reads back the most recent journal.
#[test]
fn process_status_last_reads_back_journal() {
    // First, run process to ensure at least one journal exists
    let _run = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .output()
        .expect("failed to run process run daily-validation --json");

    // Now read back the last status
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "status", "--last", "--json"])
        .output()
        .expect("failed to run process status --last --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must be valid JSON with expected fields
    assert!(
        output.status.success(),
        "process status --last should exit successfully.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"run_id\""),
        "Status output should contain a run_id.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"overall_status\""),
        "Status output should contain overall_status.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"next_action\""),
        "Status output should contain next_action.\nstdout:\n{}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// process plan integration tests
// ---------------------------------------------------------------------------

/// process plan daily-validation (human-readable) shows planned steps
/// and does NOT create any filesystem state.
#[test]
fn process_plan_daily_validation_shows_planned_steps() {
    // Use an isolated HOME so we can verify zero side effects
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home_dir = std::env::temp_dir().join(format!("arpagona-test-{test_id}"));
    std::fs::create_dir_all(&home_dir).expect("failed to create isolated HOME for test");

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "plan", "daily-validation"])
        .env("HOME", &home_dir)
        .output()
        .expect("failed to run process plan daily-validation");

    // Validate that process plan created NO .arpagona state before cleanup
    assert!(
        !home_dir.join(".arpagona").exists(),
        "process plan must not create .arpagona state directory"
    );

    let _ = std::fs::remove_dir_all(&home_dir);

    assert!(
        output.status.success(),
        "process plan should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must show the plan header
    assert!(
        stdout.contains("ARPAGONA process plan"),
        "Output should contain process plan header.\nstdout:\n{}",
        stdout
    );

    // Must list the 4 expected steps
    assert!(
        stdout.contains("doctor"),
        "Plan should list doctor step.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("cargo fmt"),
        "Plan should list cargo fmt step.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("cargo check"),
        "Plan should list cargo check step.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("cargo test"),
        "Plan should list cargo test step.\nstdout:\n{}",
        stdout
    );

    // Must show read-only mode indicator
    assert!(
        stdout.contains("read-only"),
        "Plan should indicate read-only mode.\nstdout:\n{}",
        stdout
    );

    // Must NOT start execution
    assert!(
        !stdout.contains("Starting execution"),
        "Plan must NOT start execution.\nstdout:\n{}",
        stdout
    );
}

/// process plan daily-validation --json outputs structured JSON.
#[test]
fn process_plan_daily_validation_json_is_well_formed() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "plan", "daily-validation", "--json"])
        .output()
        .expect("failed to run process plan daily-validation --json");

    assert!(
        output.status.success(),
        "process plan --json should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("process plan --json output should be valid JSON");

    // Must contain expected fields
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("process_plan"),
        "JSON should have command=process_plan"
    );
    assert_eq!(
        parsed.get("process").and_then(|v| v.as_str()),
        Some("daily-validation"),
        "JSON should have process=daily-validation"
    );
    assert_eq!(
        parsed.get("total_steps").and_then(|v| v.as_u64()),
        Some(4),
        "JSON should have total_steps=4"
    );

    // Steps should be an array of 4 strings
    let steps = parsed
        .get("steps")
        .and_then(|v| v.as_array())
        .expect("should have steps array");
    assert_eq!(steps.len(), 4, "should have 4 steps");
    assert!(
        steps
            .iter()
            .any(|s| s.as_str().map_or(false, |s| s.contains("doctor"))),
        "steps should include doctor"
    );

    // Must indicate read-only no-execution mode
    let description = parsed
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        description.contains("read-only") || description.contains("no execution"),
        "description should indicate read-only mode. Got: {}",
        description
    );
}

/// process plan with unknown process name should error.
#[test]
fn process_plan_unknown_process_reports_error() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "plan", "nonexistent"])
        .output()
        .expect("failed to run process plan nonexistent");

    assert!(
        !output.status.success(),
        "process plan with unknown process should exit with non-zero status"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown process") && stderr.contains("V0 supports only"),
        "stderr should report unknown process error.\nstderr:\n{}",
        stderr
    );
}
