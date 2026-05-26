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

/// End-to-end integration test for context-aware governed proposals.
///
/// Runs `cognitive run --objective "..." --assess --observe --propose --json`
/// with the API server, then asserts the JSON output contains all required
/// fields and that all proposed actions are PendingDecision.
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
