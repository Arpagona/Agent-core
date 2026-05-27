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

    assert!(
        output.status.success(),
        "cognitive run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // -- Assert proposed_actions present --
    let proposed_actions = parsed
        .get("proposed_actions")
        .expect("output should have proposed_actions");
    let proposed_array = proposed_actions
        .as_array()
        .expect("proposed_actions should be an array");
    assert!(
        !proposed_array.is_empty(),
        "proposed_actions should contain at least one action"
    );

    // -- Assert decisions present --
    let decisions = parsed
        .get("decisions")
        .expect("output should have decisions");
    let decisions_array = decisions.as_array().expect("decisions should be an array");
    assert!(
        !decisions_array.is_empty(),
        "decisions should contain at least one decision"
    );

    // -- Assert audit_events present --
    let audit_events = parsed
        .get("audit_events")
        .expect("output should have audit_events");
    let audit_array = audit_events
        .as_array()
        .expect("audit_events should be an array");
    assert!(
        !audit_array.is_empty(),
        "audit_events should contain at least one event"
    );

    // -- Assert non_authorizing_warning present --
    let warning = parsed
        .get("non_authorizing_warning")
        .expect("output should have non_authorizing_warning");
    assert!(
        warning.as_str().map_or(false, |s| !s.is_empty()),
        "non_authorizing_warning should be non-empty"
    );
    assert!(
        warning
            .as_str()
            .map_or(false, |s| s.contains("PendingDecision")),
        "non_authorizing_warning should mention PendingDecision"
    );
}

#[test]
fn offline_governance_produces_decisions_and_audit_events() {
    // ── Run cognitive cycle with offline governance ────────────────────────
    // Requires --assess to generate FailureInsightCandidates, then --govern
    // to produce offline DecisionGate -> Decision -> AuditEvent.
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "cognitive",
            "run",
            "--objective",
            "Vérifier un fichier de configuration YAML dans le workspace",
            "--domain",
            "engineering",
            "--context",
            "sensitivity:low",
            "--assess",
            "--govern",
            "--json",
        ])
        .output()
        .expect("failed to run cognitive run with --assess --govern --json");

    assert!(
        output.status.success(),
        "cognitive run --assess --govern --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // -- Assert governed=true --
    assert_eq!(
        parsed.get("governed").and_then(|v| v.as_bool()),
        Some(true),
        "output should have governed=true"
    );

    // -- Assert governance_results present --
    let results = parsed
        .get("governance_results")
        .expect("output should have governance_results");
    let results_array = results
        .as_array()
        .expect("governance_results should be an array");
    assert!(
        !results_array.is_empty(),
        "governance_results should contain at least one result"
    );

    // -- Assert decision_count present --
    let decision_count = parsed
        .get("decision_count")
        .and_then(|v| v.as_u64())
        .expect("output should have decision_count");
    assert!(
        decision_count > 0,
        "there should be at least one decision (got {})",
        decision_count
    );

    // -- Assert each governance result has a decision and audit_event --
    for (i, result_entry) in results_array.iter().enumerate() {
        let decision = result_entry
            .get("decision")
            .unwrap_or_else(|| panic!("governance_results[{}] should have decision", i));
        assert!(
            decision.get("id").and_then(|v| v.as_str()).is_some(),
            "governance_results[{}].decision should have an id",
            i
        );
        let audit_event = result_entry
            .get("audit_event")
            .unwrap_or_else(|| panic!("governance_results[{}] should have audit_event", i));
        assert!(
            audit_event.get("id").and_then(|v| v.as_str()).is_some(),
            "governance_results[{}].audit_event should have an id",
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

/// End-to-end integration test for cognitive run with --llm --provider mock.
///
/// Verifies that the --llm flag calls the mock provider and produces
/// non-executing, proposal-only synthesis output.
#[test]
fn cognitive_llm_mock_provides_proposal_only_synthesis() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args([
            "cognitive",
            "run",
            "--objective",
            "Test LLM synthesis with mock provider",
            "--llm",
            "--provider",
            "mock",
            "--json",
        ])
        .output()
        .expect("failed to run cognitive run --llm --provider mock --json");

    assert!(
        output.status.success(),
        "cognitive run --llm --provider mock --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // -- Verify LLM synthesis is present --
    let synthesis = parsed
        .get("llm_synthesis")
        .expect("output should have llm_synthesis");
    assert!(
        synthesis.as_str().map_or(false, |s| !s.is_empty()),
        "llm_synthesis should be a non-empty string"
    );

    // -- Verify provider metadata --
    let provider = parsed
        .get("llm_provider")
        .expect("output should have llm_provider");
    assert_eq!(provider.as_str(), Some("mock"), "provider should be 'mock'");

    // -- Verify routing info --
    let routing = parsed
        .get("llm_routing")
        .expect("output should have llm_routing");
    assert!(
        routing.as_str().map_or(false, |s| !s.is_empty()),
        "llm_routing should be a non-empty string"
    );

    // -- Verify no execution fields --
    let json_str = serde_json::to_string(&parsed).unwrap_or_default();
    assert!(
        !json_str.contains("tool_call"),
        "output should not contain tool_call markers"
    );
    assert!(
        !json_str.contains("proposed_actions"),
        "no proposed actions by --llm alone"
    );
}
