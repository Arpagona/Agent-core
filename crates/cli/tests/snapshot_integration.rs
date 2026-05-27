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

/// End-to-end integration test for offline executor commands.
///
/// Verifies that `executor list --offline` and `executor inspect --offline`
/// produce correct executor metadata directly from the core crate without
/// requiring an API server, and that the output clearly indicates offline mode.
#[test]
fn offline_executor_commands_produce_correct_output() {
    let arpagona = ARPAGONA_BIN;

    // ── executor list --offline (human-readable) ──────────────────────────
    let list_out = std::process::Command::new(arpagona)
        .args(["executor", "list", "--offline"])
        .output()
        .expect("failed to run executor list --offline");
    assert!(
        list_out.status.success(),
        "executor list --offline failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&list_out.stdout),
        String::from_utf8_lossy(&list_out.stderr),
    );
    let list_stdout = String::from_utf8(list_out.stdout).expect("valid utf-8");
    assert!(
        list_stdout.contains("[offline mode"),
        "executor list --offline should indicate offline mode\n{}",
        list_stdout
    );
    assert!(
        list_stdout.contains("noop-executor"),
        "executor list --offline should contain 'noop-executor'\n{}",
        list_stdout
    );
    assert!(
        list_stdout.contains("state=disabled"),
        "executor list --offline should show state=disabled\n{}",
        list_stdout
    );

    // ── executor list --offline --json (parsed) ──────────────────────────
    let list_json_out = std::process::Command::new(arpagona)
        .args(["executor", "list", "--offline", "--json"])
        .output()
        .expect("failed to run executor list --offline --json");
    assert!(
        list_json_out.status.success(),
        "executor list --offline --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&list_json_out.stdout),
        String::from_utf8_lossy(&list_json_out.stderr),
    );
    let list_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8(list_json_out.stdout).expect("valid utf-8"))
            .expect("executor list --offline --json output should be valid JSON");
    // Top-level wrapper with mode field
    assert_eq!(
        list_json.get("mode").and_then(|v| v.as_str()),
        Some("offline"),
        "JSON output should have mode=offline"
    );
    let executors = list_json
        .get("executors")
        .and_then(|v| v.as_array())
        .expect("JSON output should have an 'executors' array");
    assert!(
        !executors.is_empty(),
        "executor list should have at least one executor"
    );

    let noop = executors
        .iter()
        .find(|e| e.get("executor_id").and_then(|v| v.as_str()) == Some("noop-executor"))
        .expect("noop-executor should be in the list");
    assert_eq!(
        noop.get("executor_state").and_then(|v| v.as_str()),
        Some("disabled"),
        "noop-executor should have state 'disabled', got: {:?}",
        noop.get("executor_state")
    );
    let action_types = noop
        .get("supported_action_types")
        .and_then(|v| v.as_array())
        .expect("executor should have supported_action_types");
    assert!(
        !action_types.is_empty(),
        "noop-executor should support action types"
    );

    // ── executor inspect noop-executor --offline (human-readable) ────────
    let inspect_out = std::process::Command::new(arpagona)
        .args(["executor", "inspect", "noop-executor", "--offline"])
        .output()
        .expect("failed to run executor inspect noop-executor --offline");
    assert!(
        inspect_out.status.success(),
        "executor inspect --offline failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&inspect_out.stdout),
        String::from_utf8_lossy(&inspect_out.stderr),
    );
    let inspect_stdout = String::from_utf8(inspect_out.stdout).expect("valid utf-8");
    assert!(
        inspect_stdout.contains("[offline mode"),
        "executor inspect --offline should indicate offline mode\n{}",
        inspect_stdout
    );
    assert!(
        inspect_stdout.contains("Executor: noop-executor"),
        "executor inspect should contain 'Executor: noop-executor'\n{}",
        inspect_stdout
    );
    assert!(
        inspect_stdout.contains("State: "),
        "executor inspect should contain 'State:'\n{}",
        inspect_stdout
    );

    // ── executor inspect noop-executor --offline --json (parsed) ─────────
    let inspect_json_out = std::process::Command::new(arpagona)
        .args([
            "executor",
            "inspect",
            "noop-executor",
            "--offline",
            "--json",
        ])
        .output()
        .expect("failed to run executor inspect noop-executor --offline --json");
    assert!(
        inspect_json_out.status.success(),
        "executor inspect --offline --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&inspect_json_out.stdout),
        String::from_utf8_lossy(&inspect_json_out.stderr),
    );
    let inspect_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8(inspect_json_out.stdout).expect("valid utf-8"))
            .expect("executor inspect --offline --json output should be valid JSON");
    assert_eq!(
        inspect_json.get("mode").and_then(|v| v.as_str()),
        Some("offline"),
        "inspect JSON should have mode=offline"
    );
    let exec = inspect_json
        .get("executor")
        .expect("inspect JSON should have an 'executor' field");
    assert_eq!(
        exec.get("executor_id").and_then(|v| v.as_str()),
        Some("noop-executor"),
        "inspect executor should show executor_id=noop-executor"
    );
    assert_eq!(
        exec.get("executor_state").and_then(|v| v.as_str()),
        Some("disabled"),
        "inspect executor should show state=disabled"
    );
    let inspect_action_types = exec
        .get("supported_action_types")
        .and_then(|v| v.as_array())
        .expect("inspect executor should have supported_action_types");
    assert!(
        !inspect_action_types.is_empty(),
        "inspect executor should show supported action types"
    );

    // ── executor inspect nonexistent --offline ───────────────────────────
    let miss_out = std::process::Command::new(arpagona)
        .args(["executor", "inspect", "nonexistent-executor", "--offline"])
        .output()
        .expect("failed to run executor inspect nonexistent --offline");
    let miss_stdout = String::from_utf8_lossy(&miss_out.stdout);
    assert!(
        miss_stdout.contains("[offline mode"),
        "nonexistent executor should still indicate offline mode\n{}",
        miss_stdout
    );
    assert!(
        miss_stdout.contains("not found"),
        "missing executor should report 'not found'\n{}",
        miss_stdout
    );
}

/// End-to-end integration test for offline executor commands with --state-file.
///
/// Verifies that --state-file correctly applies persisted executor state
/// transitions on top of the default registry.
#[test]
fn offline_executor_state_file_produces_ready_executor() {
    let arpagona = ARPAGONA_BIN;
    let dir = std::env::temp_dir().join("arpagona-executor-state-file-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let state_path = dir.join("executor_states.json");
    let state_str = state_path.to_str().expect("valid utf-8 path").to_owned();

    // Write a state file that promotes noop-executor to Ready
    let state_content = r#"{"noop-executor": "ready"}"#;
    std::fs::write(&state_path, state_content).expect("write state file");

    // ── executor list --offline --state-file --json ──────────────────────
    let list_out = std::process::Command::new(arpagona)
        .args([
            "executor",
            "list",
            "--offline",
            "--state-file",
            &state_str,
            "--json",
        ])
        .output()
        .expect("failed to run executor list --offline --state-file --json");
    assert!(
        list_out.status.success(),
        "executor list --offline --state-file --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&list_out.stdout),
        String::from_utf8_lossy(&list_out.stderr),
    );
    let list_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8(list_out.stdout).expect("valid utf-8"))
            .expect("output should be valid JSON");
    assert_eq!(
        list_json.get("mode").and_then(|v| v.as_str()),
        Some("offline"),
        "should indicate offline mode"
    );
    let executors = list_json
        .get("executors")
        .and_then(|v| v.as_array())
        .expect("should have executors array");
    let noop = executors
        .iter()
        .find(|e| e.get("executor_id").and_then(|v| v.as_str()) == Some("noop-executor"))
        .expect("noop-executor should be present");
    assert_eq!(
        noop.get("executor_state").and_then(|v| v.as_str()),
        Some("ready"),
        "noop-executor should be 'ready' after --state-file load, got: {:?}",
        noop.get("executor_state")
    );

    // ── executor inspect noop-executor --offline --state-file --json ─────
    let inspect_out = std::process::Command::new(arpagona)
        .args([
            "executor",
            "inspect",
            "noop-executor",
            "--offline",
            "--state-file",
            &state_str,
            "--json",
        ])
        .output()
        .expect("failed to run executor inspect --offline --state-file --json");
    assert!(
        inspect_out.status.success(),
        "executor inspect --offline --state-file --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&inspect_out.stdout),
        String::from_utf8_lossy(&inspect_out.stderr),
    );
    let inspect_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8(inspect_out.stdout).expect("valid utf-8"))
            .expect("output should be valid JSON");
    assert_eq!(
        inspect_json.get("mode").and_then(|v| v.as_str()),
        Some("offline"),
        "should indicate offline mode"
    );
    let exec = inspect_json
        .get("executor")
        .expect("should have executor field");
    assert_eq!(
        exec.get("executor_state").and_then(|v| v.as_str()),
        Some("ready"),
        "inspected noop-executor should be 'ready', got: {:?}",
        exec.get("executor_state")
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cognitive_propose_pipeline_produces_governed_proposals() {
    // ── Start API server ──────────────────────────────────────────────────
    let api_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_owned()))
        .map(|p| {
            // The test binary lives in target/debug/deps/; the API server
            // binary is one level up at target/debug/arpagona-api-server
            p.parent().expect("deps parent").join("arpagona-api-server")
        })
        .expect("api server path");

    let mut server = std::process::Command::new(&api_bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start API server");

    // Wait for server to be ready using a simple TCP connection check
    use std::net::TcpStream;
    use std::time::Duration;
    let max_retries = 30;
    let mut ready = false;
    for _ in 0..max_retries {
        std::thread::sleep(Duration::from_millis(200));
        if TcpStream::connect_timeout(
            &"127.0.0.1:3000".parse().expect("valid addr"),
            Duration::from_millis(100),
        )
        .is_ok()
        {
            ready = true;
            break;
        }
    }
    assert!(ready, "API server did not become healthy in time");

    // ── Run cognitive cycle with propose bridge ────────────────────────────
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "cognitive",
            "run",
            "--objective",
            "Analyser le fichier Cargo.toml pour vérifier les dépendances Rust",
            "--assess",
            "--observe",
            "--propose",
            "--json",
        ])
        .output()
        .expect("failed to run cognitive run");

    // Cleanup: stop the API server
    let _ = server.kill();
    let _ = server.wait();

    // ── Parse and assert JSON output ──────────────────────────────────────
    assert!(
        output.status.success(),
        "cognitive run command failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout: String = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // Working memory should contain failure_insight_candidates (from --assess)
    let wm = parsed
        .get("working_memory")
        .expect("output should have working_memory");
    let fic = wm
        .get("failure_insight_candidates")
        .expect("working_memory should have failure_insight_candidates (from --assess)");
    assert!(
        fic.as_array().map_or(false, |a| !a.is_empty()),
        "failure_insight_candidates should not be empty"
    );

    // Cognitive observations should be present (from --observe)
    let obs = wm
        .get("cognitive_observations")
        .expect("working_memory should have cognitive_observations (from --observe)");
    assert!(
        obs.as_array().map_or(false, |a| !a.is_empty()),
        "cognitive_observations should not be empty"
    );

    // Proposed actions should be present (from --propose)
    let proposed_actions = parsed
        .get("proposed_actions")
        .expect("output should have proposed_actions");
    let pa_array = proposed_actions
        .as_array()
        .expect("proposed_actions should be an array");
    assert!(!pa_array.is_empty(), "proposed_actions should not be empty");

    // All proposed actions must be PendingDecision
    for (i, pa) in pa_array.iter().enumerate() {
        let status = pa
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("missing");
        assert_eq!(
            status, "pending_decision",
            "proposed_action #{} should be pending_decision, got: {}",
            i, status
        );
    }

    // Decisions should be present
    let decisions = parsed
        .get("decisions")
        .expect("output should have decisions");
    assert!(
        decisions.as_array().map_or(false, |a| !a.is_empty()),
        "decisions should not be empty"
    );

    // Audit events should be present
    let audit_events = parsed
        .get("audit_events")
        .expect("output should have audit_events");
    assert!(
        audit_events.as_array().map_or(false, |a| !a.is_empty()),
        "audit_events should not be empty"
    );

    // Non-authorizing warning should be present
    let warning = parsed
        .get("non_authorizing_warning")
        .expect("output should have non_authorizing_warning");
    assert!(
        warning.as_str().map_or(false, |s| !s.is_empty()),
        "non_authorizing_warning should be a non-empty string"
    );

    // Proposed flag should be true
    assert_eq!(
        parsed.get("proposed").and_then(|v| v.as_bool()),
        Some(true),
        "proposed flag should be true"
    );

    // Each proposed action should carry context-aware metadata in payload
    for (i, pa) in pa_array.iter().enumerate() {
        let payload = pa
            .get("payload")
            .expect("proposed_action should have payload");
        assert!(
            payload.get("originating_objective").is_some(),
            "proposed_action #{} payload should have originating_objective",
            i
        );
        assert!(
            payload.get("source_kind").is_some(),
            "proposed_action #{} payload should have source_kind",
            i
        );
        assert!(
            payload.get("expected_benefit").is_some(),
            "proposed_action #{} payload should have expected_benefit",
            i
        );
        assert!(
            payload.get("suggested_action_type").is_some(),
            "proposed_action #{} payload should have suggested_action_type",
            i
        );
        assert!(
            payload.get("rationale").is_some(),
            "proposed_action #{} payload should have rationale",
            i
        );
        // Score and priority fields
        assert!(
            payload.get("priority_score").is_some(),
            "proposed_action #{} payload should have priority_score",
            i
        );
        let score = payload
            .get("priority_score")
            .and_then(|v| v.as_f64())
            .unwrap();
        assert!(
            score >= 0.0 && score <= 2.0,
            "proposed_action #{} priority_score should be in [0.0, 2.0], got {}",
            i,
            score
        );
        assert!(
            payload.get("priority_band").is_some(),
            "proposed_action #{} payload should have priority_band",
            i
        );
        let band = payload
            .get("priority_band")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            ["high", "medium", "low"].contains(&band),
            "proposed_action #{} priority_band should be high/medium/low, got {}",
            i,
            band
        );
        assert!(
            payload.get("implementation_cost").is_some(),
            "proposed_action #{} payload should have implementation_cost",
            i
        );
    }

    // Verify proposals are sorted by priority_score descending
    for i in 1..pa_array.len() {
        let prev_score = pa_array[i - 1]
            .get("payload")
            .and_then(|p| p.get("priority_score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        let curr_score = pa_array[i]
            .get("payload")
            .and_then(|p| p.get("priority_score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        assert!(
            prev_score >= curr_score,
            "proposed_actions should be sorted by priority_score descending: #{} ({:.2}) < #{} ({:.2})",
            i - 1,
            prev_score,
            i,
            curr_score
        );
    }

    // If any proposal is batched, verify batch metadata is consistent
    for (i, pa) in pa_array.iter().enumerate() {
        let payload = pa.get("payload").expect("payload exists");
        let is_batched = payload
            .get("batched")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_batched {
            let merged_count = payload
                .get("merged_count")
                .and_then(|v| v.as_u64())
                .expect("batched actions must have merged_count");
            assert!(
                merged_count >= 2,
                "proposed_action #{} merged_count should be >= 2, got {}",
                i,
                merged_count
            );
            let ids = payload
                .get("merged_proposal_ids")
                .and_then(|v| v.as_array())
                .expect("batched actions must have merged_proposal_ids");
            assert_eq!(
                ids.len() as u64,
                merged_count,
                "proposed_action #{} merged_proposal_ids length should match merged_count",
                i
            );
        }
    }
}

/// End-to-end integration test for `cognitive run --assess --govern --json`.
///
/// Proves the full P3 governance chain works offline without an API server:
/// CognitiveObservation -> FailureInsightCandidate -> ProposedAction ->
/// DecisionGate -> Decision -> AuditEvent -> structured readback.
///
/// The `--govern` flag converts FailureInsightCandidates from `--assess`
/// into local ProposedActions, runs them through `evaluate_proposed_action`
/// (DecisionGate) and `audit_event_for_decision`, and returns the full
/// governance chain as structured JSON — all without any network,
/// external process, or API server dependency.
#[test]
fn cognitive_govern_chain_produces_decisions_and_audit_events_offline() {
    // The --govern bridge runs DecisionGate + AuditEvent locally.
    // No API server needed — this is the whole point of --govern.
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "cognitive",
            "run",
            "--objective",
            "Analyser les dépendances du projet pour identifier les risques de sécurité",
            "--assess",
            "--govern",
            "--json",
        ])
        .output()
        .expect("failed to run cognitive run --assess --govern --json");

    assert!(
        output.status.success(),
        "cognitive run --assess --govern --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Parse JSON output
    let stdout: String = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // ── Assert --assess flags ────────────────────────────────────────────
    assert_eq!(
        parsed.get("assessed").and_then(|v| v.as_bool()),
        Some(true),
        "assessed flag should be true"
    );

    // Working memory should contain failure_insight_candidates from --assess
    let wm = parsed
        .get("working_memory")
        .expect("output should have working_memory");
    let fic = wm
        .get("failure_insight_candidates")
        .expect("working_memory should have failure_insight_candidates (from --assess)");
    assert!(
        fic.as_array().map_or(false, |a| !a.is_empty()),
        "failure_insight_candidates should not be empty"
    );

    // ── Assert --govern flags ────────────────────────────────────────────
    assert_eq!(
        parsed.get("governed").and_then(|v| v.as_bool()),
        Some(true),
        "governed flag should be true"
    );

    // decision_count must be present and > 0
    let decision_count = parsed
        .get("decision_count")
        .and_then(|v| v.as_u64())
        .expect("output should have decision_count (positive integer)");
    assert!(
        decision_count > 0,
        "decision_count should be > 0, got {}",
        decision_count
    );

    // audit_event_count must be present and > 0
    let audit_event_count = parsed
        .get("audit_event_count")
        .and_then(|v| v.as_u64())
        .expect("output should have audit_event_count (positive integer)");
    assert!(
        audit_event_count > 0,
        "audit_event_count should be > 0, got {}",
        audit_event_count
    );

    // ── Assert governance_results structure ───────────────────────────────
    let governance_results = parsed
        .get("governance_results")
        .expect("output should have governance_results");
    let results_array = governance_results
        .as_array()
        .expect("governance_results should be an array");
    assert!(
        !results_array.is_empty(),
        "governance_results should not be empty"
    );
    assert!(
        results_array.len() as u64 >= decision_count,
        "governance_results length ({}) should be >= decision_count ({})",
        results_array.len(),
        decision_count
    );

    // Each governance result must have proposed_action_id, decision, audit_event
    for (i, result) in results_array.iter().enumerate() {
        let proposed_action_id = result
            .get("proposed_action_id")
            .and_then(|v| v.as_str())
            .unwrap_or("missing");
        assert!(
            !proposed_action_id.is_empty(),
            "governance_result #{} should have non-empty proposed_action_id",
            i
        );

        let decision = result
            .get("decision")
            .expect(&format!("governance_result #{} should have a decision", i));
        // Decision should be a non-null object with a status field
        let decision_status = decision
            .get("status")
            .and_then(|v| v.as_str())
            .expect(&format!(
                "governance_result #{} decision should have a status field",
                i
            ));
        // Decision status should be one of the valid states
        assert!(
            ["approved", "rejected", "needs_human_review", "blocked"].contains(&decision_status),
            "governance_result #{} decision status should be a valid DecisionGate status, got: {}",
            i,
            decision_status
        );

        let audit_event = result.get("audit_event").expect(&format!(
            "governance_result #{} should have an audit_event",
            i
        ));
        let event_type = audit_event
            .get("event_type")
            .and_then(|v| v.as_str())
            .expect(&format!(
                "governance_result #{} audit_event should have event_type",
                i
            ));
        assert!(
            !event_type.is_empty(),
            "governance_result #{} audit_event event_type should be non-empty",
            i
        );
    }

    // ── Assert governance_warning present ─────────────────────────────────
    let warning = parsed
        .get("governance_warning")
        .expect("output should have governance_warning");
    assert!(
        warning.as_str().map_or(false, |s| !s.is_empty()),
        "governance_warning should be a non-empty string"
    );
    assert!(
        warning
            .as_str()
            .map_or(false, |s| s.contains("Offline governance readback")),
        "governance_warning should indicate offline governance readback"
    );
}

#[test]
fn cognitive_observe_govern_pipeline_produces_governance_results_from_tool_observations() {
    // The --observe --govern pipeline: ToolRuntime observations -> governance via
    // DecisionGate -> Decision -> AuditEvent. Proves that ToolRuntime results (read_file,
    // list_files, search_text) propagate through the full governed learning chain
    // in a single offline invocation, without requiring the API server.
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "cognitive",
            "run",
            "--objective",
            "Analyse the codebase structure and dependencies",
            "--assess",
            "--observe",
            "--govern",
            "--json",
        ])
        .output()
        .expect("failed to run cognitive run --assess --observe --govern --json");

    assert!(
        output.status.success(),
        "cognitive run --assess --observe --govern --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Parse JSON output
    let stdout: String = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // -- Assert --observe flags --
    let wm = parsed
        .get("working_memory")
        .expect("output should have working_memory");

    // cognitive_observations must be present from --observe
    let observations = wm
        .get("cognitive_observations")
        .expect("working_memory should have cognitive_observations (from --observe)");
    let obs_array = observations
        .as_array()
        .expect("cognitive_observations should be an array");
    assert!(
        !obs_array.is_empty(),
        "cognitive_observations should not be empty"
    );

    // Each observation must have tool_name, kind, status
    for (i, obs) in obs_array.iter().enumerate() {
        let tool_name = obs
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("missing");
        assert!(
            !tool_name.is_empty(),
            "observation #{} should have a non-empty tool_name",
            i
        );
        let kind = obs.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            !kind.is_empty(),
            "observation #{} should have a non-empty kind",
            i
        );
        let status = obs.get("status").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            !status.is_empty(),
            "observation #{} should have a non-empty status",
            i
        );
    }

    // observed flag must be true
    assert_eq!(
        wm.get("observed").and_then(|v| v.as_bool()),
        Some(true),
        "observed flag should be true"
    );

    // -- Assert --assess flags --
    assert_eq!(
        parsed.get("assessed").and_then(|v| v.as_bool()),
        Some(true),
        "assessed flag should be true"
    );

    // failure_insight_candidates must be present
    let fic = wm.get("failure_insight_candidates").expect(
        "working_memory should have failure_insight_candidates (from --assess + observations)",
    );
    assert!(
        fic.as_array().map_or(false, |a| !a.is_empty()),
        "failure_insight_candidates should not be empty"
    );

    // -- Assert --govern flags --
    assert_eq!(
        parsed.get("governed").and_then(|v| v.as_bool()),
        Some(true),
        "governed flag should be true"
    );

    // decision_count must be present and > 0
    let decision_count = parsed
        .get("decision_count")
        .and_then(|v| v.as_u64())
        .expect("output should have decision_count (positive integer)");
    assert!(
        decision_count > 0,
        "decision_count should be > 0, got {}",
        decision_count
    );

    // audit_event_count must be present and > 0
    let audit_event_count = parsed
        .get("audit_event_count")
        .and_then(|v| v.as_u64())
        .expect("output should have audit_event_count (positive integer)");
    assert!(
        audit_event_count > 0,
        "audit_event_count should be > 0, got {}",
        audit_event_count
    );

    // -- Assert governance_results structure --
    let governance_results = parsed
        .get("governance_results")
        .expect("output should have governance_results");
    let results_array = governance_results
        .as_array()
        .expect("governance_results should be an array");
    assert!(
        !results_array.is_empty(),
        "governance_results should not be empty"
    );
    assert!(
        results_array.len() as u64 >= decision_count,
        "governance_results length ({}) should be >= decision_count ({})",
        results_array.len(),
        decision_count
    );

    // Each governance result must have proposed_action_id, decision, audit_event
    for (i, result) in results_array.iter().enumerate() {
        let proposed_action_id = result
            .get("proposed_action_id")
            .and_then(|v| v.as_str())
            .unwrap_or("missing");
        assert!(
            !proposed_action_id.is_empty(),
            "governance_result #{} should have non-empty proposed_action_id",
            i
        );

        let decision = result
            .get("decision")
            .expect(&format!("governance_result #{} should have a decision", i));
        let decision_status = decision
            .get("status")
            .and_then(|v| v.as_str())
            .expect(&format!(
                "governance_result #{} decision should have a status field",
                i
            ));
        assert!(
            ["approved", "rejected", "needs_human_review", "blocked"].contains(&decision_status),
            "governance_result #{} decision status should be a valid DecisionGate status, got: {}",
            i,
            decision_status
        );

        let audit_event = result.get("audit_event").expect(&format!(
            "governance_result #{} should have an audit_event",
            i
        ));
        let event_type = audit_event
            .get("event_type")
            .and_then(|v| v.as_str())
            .expect(&format!(
                "governance_result #{} audit_event should have event_type",
                i
            ));
        assert!(
            !event_type.is_empty(),
            "governance_result #{} audit_event event_type should be non-empty",
            i
        );
    }

    // -- Assert governance_warning present --
    let warning = parsed
        .get("governance_warning")
        .expect("output should have governance_warning");
    assert!(
        warning.as_str().map_or(false, |s| !s.is_empty()),
        "governance_warning should be a non-empty string"
    );
    assert!(
        warning
            .as_str()
            .map_or(false, |s| s.contains("Offline governance readback")),
        "governance_warning should indicate offline governance readback"
    );
}
