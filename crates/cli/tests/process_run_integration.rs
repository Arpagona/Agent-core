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
    let journal_dir = temp_journal_dir();
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
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

/// Test helper: create a unique temp dir for journal isolation.
/// Returns the temp directory path. The caller MUST set
/// `ARPAGONA_PROCESS_JOURNAL_DIR` env on the Command to this path.
fn temp_journal_dir() -> std::path::PathBuf {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("arpagona-journal-test-{test_id}"));
    std::fs::create_dir_all(&dir).expect("failed to create temp journal dir");
    dir
}

/// The real user-level process journal directory for ARPAGONA.
/// Used by the regression test to verify that integration tests do NOT pollute it.
const REAL_PROCESS_JOURNAL_DIR: &str = "/home/thibaud/.arpagona/process-journal";

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
    let journal_dir = temp_journal_dir();
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
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
    let journal_dir = temp_journal_dir();
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process run daily-validation --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract the run_id from the JSON output (it appears in the plan phase)
    let run_id_marker = "\"run_id\": \"daily-validation-";
    if let Some(start) = stdout.find(run_id_marker) {
        let after_prefix = &stdout[start + run_id_marker.len()..];
        if let Some(end) = after_prefix.find('"') {
            let run_id = format!("daily-validation-{}", &after_prefix[..end]);

            // Resolve the journal path — must match process_journal_dir_resolved()
            // in main.rs. Also check ARPAGONA_PROCESS_JOURNAL_DIR first.
            let journal_path = journal_dir.join(format!("{}.json", run_id));

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
    let journal_dir = temp_journal_dir();
    // First, run process to ensure at least one journal exists
    let _run = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process run daily-validation --json");

    // Now read back the last status
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "status", "--last", "--json"])
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
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

// ---------------------------------------------------------------------------
// process list integration tests
// ---------------------------------------------------------------------------

/// process list with empty journal dir shows graceful message.
#[test]
fn process_list_empty_journal_is_graceful() {
    // Use isolated HOME so we have zero pre-existing journals
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home_dir = std::env::temp_dir().join(format!("arpagona-test-{test_id}"));

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list"])
        .env("HOME", &home_dir)
        .output()
        .expect("failed to run process list");

    let _ = std::fs::remove_dir_all(&home_dir);

    assert!(
        output.status.success(),
        "process list with empty journal should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No process runs found"),
        "Empty journal should show 'No process runs found'.\nstdout:\n{}",
        stdout
    );
}

/// process list --json with empty journal returns well-formed JSON.
#[test]
fn process_list_empty_journal_json_is_well_formed() {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home_dir = std::env::temp_dir().join(format!("arpagona-test-{test_id}"));

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list", "--json"])
        .env("HOME", &home_dir)
        .output()
        .expect("failed to run process list --json");

    let _ = std::fs::remove_dir_all(&home_dir);

    assert!(
        output.status.success(),
        "process list --json should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("process list --json output should be valid JSON");

    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("process_list"),
        "JSON should have command=process_list"
    );
    assert_eq!(
        parsed.get("total").and_then(|v| v.as_u64()),
        Some(0),
        "JSON should have total=0 for empty journal"
    );
    assert!(
        parsed.get("runs").and_then(|v| v.as_array()).is_some(),
        "JSON should have 'runs' array"
    );
    assert_eq!(
        parsed.get("runs").and_then(|v| v.as_array()).unwrap().len(),
        0,
        "runs array should be empty"
    );
}

/// process list shows journal entries after a process run.
#[test]
fn process_list_shows_journal_entries() {
    let journal_dir = temp_journal_dir();
    // First, run process to create at least one journal
    let _run = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process run daily-validation --json");

    // Now list the journals
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list"])
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process list");

    assert!(
        output.status.success(),
        "process list should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ARPAGONA process run journals"),
        "Should show header 'ARPAGONA process run journals'.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Total:"),
        "Should show total run count.\nstdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("daily-validation-"),
        "Should show the daily-validation run ID.\nstdout:\n{}",
        stdout
    );
}

/// process list --json returns well-formed JSON with runs.
#[test]
fn process_list_json_is_well_formed() {
    let journal_dir = temp_journal_dir();
    // Run process to create journals
    let _run = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process run daily-validation --json");

    // Now list with JSON
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list", "--json"])
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process list --json");

    assert!(
        output.status.success(),
        "process list --json should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("process list --json output should be valid JSON");

    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("process_list"),
        "JSON should have command=process_list"
    );
    assert!(
        parsed.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "JSON should have total >= 1 run"
    );

    let runs = parsed
        .get("runs")
        .and_then(|v| v.as_array())
        .expect("JSON should have 'runs' array");
    assert!(!runs.is_empty(), "runs array should not be empty");

    // Each run must have required fields
    for run in runs {
        assert!(
            run.get("run_id").and_then(|v| v.as_str()).is_some(),
            "Each run should have a run_id. Got: {:?}",
            run
        );
        assert!(
            run.get("overall_status").and_then(|v| v.as_str()).is_some(),
            "Each run should have overall_status. Got: {:?}",
            run
        );
    }

    // Runs should be sorted newest first (human output)
    assert!(
        stdout.contains("daily-validation-"),
        "JSON output should contain run IDs.\nstdout:\n{}",
        stdout
    );
}

/// process list handles a corrupt journal file without panicking.
#[test]
fn process_list_corrupt_journal_does_not_panic() {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home_dir = std::env::temp_dir().join(format!("arpagona-test-{test_id}"));
    let journal_dir = home_dir.join(".arpagona").join("process-journal");
    std::fs::create_dir_all(&journal_dir).expect("failed to create journal dir");

    // Write a corrupt "journal" file (invalid JSON)
    let corrupt_path = journal_dir.join("corrupt-run.json");
    std::fs::write(&corrupt_path, "This is not valid JSON {{")
        .expect("failed to write corrupt file");

    // Write a legitimate journal file
    let valid_path = journal_dir.join("valid-run.json");
    let valid_journal = serde_json::json!({
        "run_id": "valid-run",
        "process": "daily-validation",
        "started_at": "2026-05-31T12:00:00Z",
        "ended_at": "2026-05-31T12:05:00Z",
        "planned_steps": ["doctor"],
        "step_results": [{"step": 1, "name": "doctor", "status": "PASSED"}],
        "overall_status": "PASSED",
        "next_action": "none"
    });
    std::fs::write(&valid_path, valid_journal.to_string()).expect("failed to write valid file");

    // Run process list --json (read-only, no side effects)
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list", "--json"])
        .env("HOME", &home_dir)
        .output()
        .expect("failed to run process list --json with corrupt journal");

    let _ = std::fs::remove_dir_all(&home_dir);

    assert!(
        output.status.success(),
        "process list should not panic with corrupt journals.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("process list --json output should be valid JSON");

    // Should have both the valid and corrupt entry in total
    let total = parsed.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(
        total >= 2,
        "JSON should list all journal files (valid + corrupt). Got total={}",
        total
    );
    assert!(
        stdout.contains("CORRUPT") || stdout.contains("corrupt_entries"),
        "Output should indicate corruption. stdout:\n{}",
        stdout
    );
}

/// process list does NOT create the journal directory — read-only means
/// no state creation, not even the parent directory.
#[test]
fn process_list_does_not_create_journal_dir() {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home_dir = std::env::temp_dir().join(format!("arpagona-test-{test_id}"));

    // Run `process list` against a pristine HOME (no .arpagona exists)
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list"])
        .env("HOME", &home_dir)
        .output()
        .expect("failed to run process list");

    // Must exit successfully
    assert!(
        output.status.success(),
        "process list on empty HOME should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Must NOT have created the journal directory
    let journal_dir = home_dir.join(".arpagona").join("process-journal");
    assert!(
        !journal_dir.exists(),
        "process list must not create the journal directory. Found: {}",
        journal_dir.display()
    );

    let _ = std::fs::remove_dir_all(&home_dir);
}

/// process list --json does NOT create the journal directory either.
#[test]
fn process_list_json_does_not_create_journal_dir() {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home_dir = std::env::temp_dir().join(format!("arpagona-test-{test_id}"));

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "list", "--json"])
        .env("HOME", &home_dir)
        .output()
        .expect("failed to run process list --json");

    assert!(
        output.status.success(),
        "process list --json on empty HOME should exit successfully.\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Must NOT have created the journal directory
    let journal_dir = home_dir.join(".arpagona").join("process-journal");
    assert!(
        !journal_dir.exists(),
        "process list --json must not create the journal directory. Found: {}",
        journal_dir.display()
    );

    // Also verify the JSON output is valid and shows total=0
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("process list --json output should be valid JSON");
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("process_list"),
        "JSON should have command=process_list"
    );
    assert_eq!(
        parsed.get("total").and_then(|v| v.as_u64()),
        Some(0),
        "JSON should have total=0 for empty journal"
    );

    let _ = std::fs::remove_dir_all(&home_dir);
}

// ---------------------------------------------------------------------------
// Regression: process run must NOT pollute the real user journal directory
// ---------------------------------------------------------------------------

/// Verifies that running `process run` with ARPAGONA_PROCESS_JOURNAL_DIR
/// isolation does NOT create or modify files under the real journal dir.
#[test]
fn process_run_does_not_pollute_real_journal_dir() {
    // Snapshot the real journal directory before and after.
    let real_dir = std::path::Path::new(REAL_PROCESS_JOURNAL_DIR);
    let before: std::collections::BTreeSet<String> = if real_dir.exists() {
        std::fs::read_dir(real_dir)
            .expect("should read real journal dir")
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    } else {
        std::collections::BTreeSet::new()
    };
    let before_count = before.len();

    // Run process with isolated journal dir
    let journal_dir = temp_journal_dir();
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["process", "run", "daily-validation", "--json"])
        .env("OLLAMA_ENDPOINT", "http://127.0.0.1:9")
        .env("ARPAGONA_PROCESS_JOURNAL_DIR", &journal_dir)
        .output()
        .expect("failed to run process run daily-validation --json");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify the process run itself behaves correctly (BLOCKED at step 1)
    assert!(
        stdout.contains("\"overall_status\": \"BLOCKED\""),
        "process run should be BLOCKED. stdout:\n{}",
        stdout
    );

    // Verify the journal was written to the isolated dir, NOT the real dir
    let after: std::collections::BTreeSet<String> = if real_dir.exists() {
        std::fs::read_dir(real_dir)
            .expect("should read real journal dir")
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    } else {
        std::collections::BTreeSet::new()
    };
    let after_count = after.len();

    assert_eq!(
        before_count,
        after_count,
        "Real journal dir was polluted! Before: {} files, after: {} files.\n\
         New files: {:?}",
        before_count,
        after_count,
        after.difference(&before).collect::<Vec<_>>()
    );

    // Verify the isolated dir contains the journal
    let isolated_files: Vec<_> = std::fs::read_dir(&journal_dir)
        .expect("should read isolated journal dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .collect();
    assert!(
        !isolated_files.is_empty(),
        "Isolated journal dir should contain at least one journal file"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&journal_dir);
}
