//! Smoke/hardening integration tests for actor readback commands.
//!
//! Proves that actor status, memory, journal, and history readback commands
//! produce correct output (JSON and text mode) without errors, handle empty
//! journal state gracefully, and always emit NON_AUTHORIZING_READBACK warnings.
//! No mutation paths are exercised — all commands are read-only.

/// Path to the compiled arpagona binary, resolved by Cargo at compile time.
const ARPAGONA_BIN: &str = env!("CARGO_BIN_EXE_arpagona");

/// Return a unique temp file path for an isolated journal.
///
/// The file does not exist — the journal module will read it as empty.
/// Each test invocation gets its own path to avoid cross-test contamination.
fn temp_isolated_journal_path(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    dir.join(format!(
        "arpagona-test-{}-{}-journal.jsonl",
        label,
        std::process::id(),
    ))
}

/// Run `actor journal --json --limit 10` with an explicitly isolated journal
/// and verify the output is valid JSON with NON_AUTHORIZING_READBACK.
fn run_journal_json_isolated(label: &str) -> serde_json::Value {
    let path = temp_isolated_journal_path(label);
    // Ensure the file does NOT exist before the command runs
    let _ = std::fs::remove_file(&path);
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "journal", "--json", "--limit", "10"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor journal --json --limit 10");
    assert!(
        output.status.success(),
        "actor journal --json --limit 10 failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    serde_json::from_str(&stdout).expect("actor journal --json should be valid JSON")
    // Leave the temp file for diagnostics if the test fails; cleanup is best-effort.
}

// ---------------------------------------------------------------------------
// actor status -- smoke tests
// ---------------------------------------------------------------------------

#[test]
fn actor_status_json_produces_valid_readback() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "status", "--json"])
        .output()
        .expect("failed to run actor status --json");
    assert!(
        output.status.success(),
        "actor status --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor status --json should be valid JSON");

    // Top-level actor_status section
    let status = parsed
        .get("actor_status")
        .expect("JSON should contain 'actor_status'");
    assert!(
        status.get("agent_id").is_some(),
        "actor_status should contain agent_id"
    );
    assert!(
        status.get("agent_kind").is_some(),
        "actor_status should contain agent_kind"
    );
    assert!(
        status.get("workspace_id").is_some(),
        "actor_status should contain workspace_id"
    );
    assert!(
        status.get("api_url").is_some(),
        "actor_status should contain api_url"
    );

    // Journal summary section (may be 0 entries in test context — that's OK)
    let journal = parsed
        .get("journal_summary")
        .expect("JSON should contain 'journal_summary'");
    assert!(
        journal.get("total_entries").is_some(),
        "journal_summary should contain total_entries"
    );
    assert!(
        journal.get("direct_tool_calls").is_some(),
        "journal_summary should contain direct_tool_calls"
    );
    assert!(
        journal.get("governance_entries").is_some(),
        "journal_summary should contain governance_entries"
    );

    // NON_AUTHORIZING_READBACK warning must be present
    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "JSON output must contain NON_AUTHORIZING_READBACK warning, got: {warning}"
    );
}

#[test]
fn actor_status_text_produces_readback_output() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "status"])
        .output()
        .expect("failed to run actor status");
    assert!(
        output.status.success(),
        "actor status failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    assert!(
        stdout.contains("Actor Status Readback"),
        "text output should contain header"
    );
    assert!(
        stdout.contains("NON_AUTHORIZING_READBACK"),
        "text output must contain NON_AUTHORIZING_READBACK warning"
    );
    assert!(
        stdout.contains("agent_id") && stdout.contains("workspace_id"),
        "text output should show agent_id and workspace_id"
    );
    // Not JSON (text mode)
    assert!(
        !stdout.trim().starts_with('{'),
        "text mode should not emit JSON"
    );
}

// ---------------------------------------------------------------------------
// actor memory -- smoke tests
// ---------------------------------------------------------------------------

#[test]
fn actor_memory_json_produces_valid_readback() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "memory", "--json"])
        .output()
        .expect("failed to run actor memory --json");
    assert!(
        output.status.success(),
        "actor memory --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor memory --json should be valid JSON");

    // Top-level actor_memory section
    let memory = parsed
        .get("actor_memory")
        .expect("JSON should contain 'actor_memory'");
    assert!(
        memory.get("graph_memory_support_compiled").is_some(),
        "actor_memory should contain graph_memory_support_compiled"
    );
    assert!(
        memory.get("configured_backend").is_some(),
        "actor_memory should contain configured_backend"
    );
    assert!(
        memory.get("memory_active").is_some(),
        "actor_memory should contain memory_active"
    );
    let alpha_limits = memory
        .get("alpha_limits")
        .and_then(|v| v.as_array())
        .expect("actor_memory should contain alpha_limits array");
    assert!(!alpha_limits.is_empty(), "alpha_limits should not be empty");

    // Access methods
    let access_methods = parsed
        .get("access_methods")
        .and_then(|v| v.as_array())
        .expect("JSON should contain access_methods array");
    assert!(
        !access_methods.is_empty(),
        "access_methods should not be empty"
    );

    // NON_AUTHORIZING_READBACK warning
    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "JSON output must contain NON_AUTHORIZING_READBACK warning, got: {warning}"
    );
}

#[test]
fn actor_memory_text_produces_readback_output() {
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "memory"])
        .output()
        .expect("failed to run actor memory");
    assert!(
        output.status.success(),
        "actor memory failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    assert!(
        stdout.contains("Actor Memory Readback"),
        "text output should contain header"
    );
    assert!(
        stdout.contains("NON_AUTHORIZING_READBACK"),
        "text output must contain NON_AUTHORIZING_READBACK warning"
    );
    assert!(
        stdout.contains("Alpha Limits"),
        "text output should show Alpha Limits section"
    );
    assert!(
        stdout.contains("Access Methods"),
        "text output should show Access Methods section"
    );
    // Not JSON
    assert!(
        !stdout.trim().starts_with('{'),
        "text mode should not emit JSON"
    );
}

// ---------------------------------------------------------------------------
// actor journal -- smoke tests (empty journal — no prior actor runs)
// ---------------------------------------------------------------------------

#[test]
fn actor_journal_json_empty_journal_produces_valid_readback() {
    // Uses isolated temp journal to guarantee empty state.
    let parsed = run_journal_json_isolated("actor_journal_json_empty_journal");

    // Must have total_entries and displayed_entries fields
    assert!(
        parsed.get("total_entries").is_some(),
        "JSON should contain total_entries"
    );
    assert!(
        parsed.get("displayed_entries").is_some(),
        "JSON should contain displayed_entries"
    );
    assert!(
        parsed.get("entries").is_some(),
        "JSON should contain entries array"
    );
    // entries may be empty — that's fine for an empty journal
    let entries = parsed.get("entries").and_then(|v| v.as_array()).unwrap();
    assert!(
        entries.is_empty() || entries.len() <= 10,
        "entries should be at most the requested limit"
    );

    // NON_AUTHORIZING_READBACK warning
    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "JSON output must contain NON_AUTHORIZING_READBACK warning, got: {warning}"
    );
}

#[test]
fn actor_journal_text_empty_journal_does_not_panic() {
    let path = temp_isolated_journal_path("actor_journal_text_empty_journal");
    let _ = std::fs::remove_file(&path);
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "journal"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor journal");
    assert!(
        output.status.success(),
        "actor journal (text) failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    assert!(
        stdout.contains("Actor Journal Readback"),
        "text output should contain header"
    );
    assert!(
        stdout.contains("NON_AUTHORIZING_READBACK"),
        "text output must contain NON_AUTHORIZING_READBACK warning"
    );
    // Even with empty journal, should show total count
    assert!(
        stdout.contains("entries total"),
        "should report total entries count"
    );
}

// ---------------------------------------------------------------------------
// actor history -- smoke tests (empty journal — no prior actor runs)
// ---------------------------------------------------------------------------

/// GONA-requested live smoke test: actor history --limit 3 --json.
/// Verifies JSON output structure even with no prior actor runs.
#[test]
fn actor_history_json_limit_3_produces_valid_readback() {
    let path = temp_isolated_journal_path("actor_history_json_limit_3");
    let _ = std::fs::remove_file(&path);
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "history", "--limit", "3", "--json"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor history --limit 3 --json");
    assert!(
        output.status.success(),
        "actor history --limit 3 --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor history --json should be valid JSON");

    // Must have command identifier
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("actor_history"),
        "JSON should identify command as actor_history"
    );
    // Must have total_matching count (may be 0 — that's fine)
    assert!(
        parsed.get("total_matching").is_some(),
        "JSON should contain total_matching"
    );
    // Must have entries array
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("JSON should contain entries array");
    assert!(
        entries.len() <= 3,
        "entries should be at most the requested limit of 3"
    );

    // NON_AUTHORIZING_READBACK warning
    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "JSON output must contain NON_AUTHORIZING_READBACK warning, got: {warning}"
    );
}

#[test]
fn actor_history_json_limit_0_produces_valid_readback() {
    let path = temp_isolated_journal_path("actor_history_json_limit_0");
    let _ = std::fs::remove_file(&path);
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "history", "--limit", "0", "--json"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor history --limit 0 --json");
    assert!(
        output.status.success(),
        "actor history --limit 0 --json failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor history --limit 0 --json should be valid JSON");
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("actor_history"),
        "JSON should identify command as actor_history"
    );
}

#[test]
fn actor_history_text_limit_1_produces_output() {
    let path = temp_isolated_journal_path("actor_history_text_limit_1");
    let _ = std::fs::remove_file(&path);
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "history", "--limit", "1"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor history --limit 1");
    assert!(
        output.status.success(),
        "actor history --limit 1 failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    assert!(
        stdout.contains("Actor Run History"),
        "text output should contain header"
    );
    assert!(
        stdout.contains("most recent run"),
        "should indicate how many runs are shown"
    );
}

// ---------------------------------------------------------------------------
// actor journal -- non-ASCII readback regression (char-boundary safety)
// ---------------------------------------------------------------------------

/// Regression test: actor journal text-mode must not panic when
/// prompt_summary/response_summary contain non-ASCII characters at
/// the truncation boundary (~120 bytes for byte-indexed slicing).
///
/// Before the fix, byte-indexed truncation (`&s[..s.len().min(N)]`)
/// panics with "end byte index N is not a char boundary" when a
/// multi-byte UTF-8 character straddles the cut point. The safe
/// pattern (`s.chars().take(N).collect::<String>()`) avoids this.
#[test]
fn actor_journal_non_ascii_truncation_does_not_panic() {
    // Build a single journal entry with non-ASCII chars precisely
    // placed so that byte-indexed truncation at 120 would split a
    // multi-byte character.
    //
    // Use a 3-byte Unicode character (U+2728 ✨ SPARKLES). Pad with
    // ASCII 'x' chars so the boundary crosses the multi-byte char.
    let sparkle = "\u{2728}"; // ✨ — 3 bytes in UTF-8
                              // prompt_summary: 118 ASCII 'x's + ✨ → byte 118 is end of x's,
                              // byte 119 is start of ✨. 120-byte slice would try to split at
                              // byte 120 which is inside ✨'s 3-byte sequence. After fix,
                              // char-based truncation at 120 chars includes the full ✨.
    let mut prompt = "x".repeat(118);
    prompt.push_str(sparkle);
    prompt.push_str(" end marker");
    // Same pattern for response_summary
    let mut response = "y".repeat(118);
    response.push_str(sparkle);
    response.push_str(" end marker");

    // Build a minimal journal entry in JSONL format.
    // Objective must be "actor_run" for the default journal readback filter
    // (line ~13269-13277) to include it in text-mode display.
    // Non-ASCII content goes in prompt_summary and response_summary for
    // truncation boundary testing.
    let objective_for_filter = "actor_run";
    let entry = serde_json::json!({
        "id": "non-ascii-regression-001",
        "created_at": "2026-06-01T07:00:00Z",
        "interaction_type": "direct_tool_call",
        "provider": "regression-test",
        "model": null,
        "objective": objective_for_filter,
        "prompt_summary": prompt,
        "response_summary": response,
        "tool_call_intents": {"tool": "regression_test"},
        "decision_gate_outcomes": {"decision_status": "approved", "approval_state": "automatic"},
        "risk_level": "low",
    });

    let journal_line = serde_json::to_string(&entry).expect("valid json");
    let path = temp_isolated_journal_path("non_ascii_truncation");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, format!("{journal_line}\n")).expect("write journal file");

    // Run actor journal (text mode) — this would have panicked before the fix
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "journal"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor journal with non-ASCII data");

    assert!(
        output.status.success(),
        "actor journal (text) with non-ASCII must not panic:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    assert!(
        stdout.contains("NON_AUTHORIZING_READBACK"),
        "must emit NON_AUTHORIZING_READBACK warning"
    );
    assert!(
        stdout.contains(sparkle),
        "non-ASCII character must appear in readback output"
    );

    // Also run actor journal --json — just the readback command, not json output check
    let json_output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "journal", "--json", "--limit", "10"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor journal --json with non-ASCII data");
    assert!(
        json_output.status.success(),
        "actor journal --json with non-ASCII must not panic:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&json_output.stdout),
        String::from_utf8_lossy(&json_output.stderr),
    );

    // Clean up
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// llm journal + action supervise — non-ASCII readback live smoke (PR #256)
// ---------------------------------------------------------------------------

/// Live smoke test: `arpagona llm journal` and `arpagona action supervise`
/// must display non-ASCII objectives/prompts/action JSON without panic,
/// exercised through a cross-process readback from a pre-seeded journal.
///
/// Covers the 80-char, 120-char, and 200-char truncation boundaries that
/// PR #256 (#406a4c2) hardened with char-boundary-safe slicing:
///   - llm journal list: objective (80), prompt/response (120),
///     compute_routing justification (120)
///   - action supervise: objective (80), proposed_actions (200),
///     tool_call_intents (200), decision_gate_outcomes (200)
#[test]
fn llm_journal_and_action_supervise_non_ascii_readback() {
    let sparkle = "\u{2728}"; // ✨ — 3 bytes in UTF-8

    // Build an objective that crosses the 80-char truncation boundary
    // with a multi-byte character at position ~78-80.
    let mut objective = "o".repeat(78);
    objective.push_str(sparkle);
    objective.push_str(" end");

    // Build prompt/response that cross the 120-char boundary
    let mut prompt = "p".repeat(118);
    prompt.push_str(sparkle);
    prompt.push_str(" end");

    let mut response = "r".repeat(118);
    response.push_str(sparkle);
    response.push_str(" end");

    // Build a justification that crosses the 120-char boundary
    let mut justification = "j".repeat(118);
    justification.push_str(sparkle);
    justification.push_str(" end");

    // Build proposed_actions JSON that crosses the 200-char boundary
    let mut action_text = "action payload ".repeat(12); // ~168 chars
    action_text.push_str(sparkle);
    action_text.push_str(" end marker end marker"); // pushes past 200

    // Build tool_call_intents JSON that crosses the 200-char boundary
    let mut tci_text = "tool intent ".repeat(12); // ~156 chars
    tci_text.push_str(sparkle);
    tci_text.push_str(" end marker end marker end marker"); // pushes past 200

    // Build decision_gate_outcomes JSON that crosses the 200-char boundary
    let mut dg_text = "decision gate outcome ".repeat(9); // ~198 chars
    dg_text.push_str(sparkle);
    dg_text.push_str(" final"); // pushes past 200

    let entry = serde_json::json!({
        "id": "non-ascii-llm-smoke-001",
        "created_at": "2026-06-01T07:30:00Z",
        "interaction_type": "direct_tool_call",
        "provider": "live-smoke-test",
        "model": null,
        "objective": objective,
        "prompt_summary": prompt,
        "response_summary": response,
        "proposed_actions": [{"action": action_text, "rationale": "smoke proof"}],
        "tool_call_intents": [{"tool": "smoke_test", "arguments": {"input": tci_text}}],
        "decision_gate_outcomes": [{"decision": {"status": "approved"}, "outcome": dg_text}],
        "risk_level": "low",
        "compute_routing": {
            "selected_node_id": "test-node",
            "justification": justification,
            "routing_note": sparkle
        },
    });

    let journal_line = serde_json::to_string(&entry).expect("valid json");
    let path = temp_isolated_journal_path("non_ascii_llm_supervise_smoke");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, format!("{journal_line}\n")).expect("write journal file");

    // ── 1. arpagona llm journal (text mode) ─────────────────────────────
    let llm_text = std::process::Command::new(ARPAGONA_BIN)
        .args(["llm", "journal"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run llm journal (text) with non-ASCII data");

    assert!(
        llm_text.status.success(),
        "llm journal (text) with non-ASCII must not panic:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&llm_text.stdout),
        String::from_utf8_lossy(&llm_text.stderr),
    );

    let llm_stdout = String::from_utf8(llm_text.stdout).expect("valid utf-8");
    assert!(
        llm_stdout.contains(sparkle),
        "llm journal (text) must display non-ASCII sparkle in output:\n{}",
        llm_stdout
    );
    assert!(
        llm_stdout.contains("LLM Interaction Journal"),
        "llm journal (text) must show header"
    );

    // ── 2. arpagona llm journal --json ──────────────────────────────────
    let llm_json = std::process::Command::new(ARPAGONA_BIN)
        .args(["llm", "journal", "--json", "--limit", "10"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run llm journal --json with non-ASCII data");

    assert!(
        llm_json.status.success(),
        "llm journal --json with non-ASCII must not panic:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&llm_json.stdout),
        String::from_utf8_lossy(&llm_json.stderr),
    );

    let llm_json_stdout = String::from_utf8(llm_json.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&llm_json_stdout).expect("llm journal --json should be valid JSON");
    let entries = parsed.get("entries").and_then(|v| v.as_array()).unwrap();
    assert!(
        !entries.is_empty(),
        "llm journal --json should have entries"
    );
    assert!(
        parsed
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1,
        "total_entries should be >= 1"
    );

    // ── 3. arpagona action supervise (text mode) ────────────────────────
    let supervise_text = std::process::Command::new(ARPAGONA_BIN)
        .args(["action", "supervise"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run action supervise (text) with non-ASCII data");

    assert!(
        supervise_text.status.success(),
        "action supervise (text) with non-ASCII must not panic:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&supervise_text.stdout),
        String::from_utf8_lossy(&supervise_text.stderr),
    );

    let supervise_stdout = String::from_utf8(supervise_text.stdout).expect("valid utf-8");
    assert!(
        supervise_stdout.contains(sparkle),
        "action supervise (text) must display non-ASCII sparkle in output:\n{}",
        supervise_stdout
    );
    assert!(
        supervise_stdout.contains("Action Supervision Surface"),
        "action supervise (text) must show header"
    );

    // ── 4. arpagona action supervise --json ─────────────────────────────
    let supervise_json = std::process::Command::new(ARPAGONA_BIN)
        .args(["action", "supervise", "--json"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run action supervise --json with non-ASCII data");

    assert!(
        supervise_json.status.success(),
        "action supervise --json with non-ASCII must not panic:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&supervise_json.stdout),
        String::from_utf8_lossy(&supervise_json.stderr),
    );

    let supervise_json_stdout = String::from_utf8(supervise_json.stdout).expect("valid utf-8");
    let _parsed: serde_json::Value = serde_json::from_str(&supervise_json_stdout)
        .expect("action supervise --json should be valid JSON");

    // Clean up
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// actor status -- isolated journal smoke tests (PR #258)
// ---------------------------------------------------------------------------

/// actor status --json with an explicitly isolated empty journal.
/// Proves that journal_summary correctly reports zero entries when the
/// journal file is empty/missing, and that NON_AUTHORIZING_READBACK is
/// always emitted regardless of journal state.
#[test]
fn actor_status_json_isolated_empty_journal() {
    let path = temp_isolated_journal_path("actor_status_isolated_empty");
    let _ = std::fs::remove_file(&path);

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "status", "--json"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor status --json with isolated empty journal");
    assert!(
        output.status.success(),
        "actor status --json (isolated empty) failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor status --json should be valid JSON");

    // journal_summary must show 0 entries
    let journal = parsed
        .get("journal_summary")
        .expect("JSON should contain 'journal_summary'");
    assert_eq!(
        journal
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX),
        0,
        "journal_summary total_entries should be 0 with empty journal"
    );
    assert_eq!(
        journal
            .get("direct_tool_calls")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX),
        0,
        "journal_summary direct_tool_calls should be 0 with empty journal"
    );
    assert_eq!(
        journal
            .get("governance_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX),
        0,
        "journal_summary governance_entries should be 0 with empty journal"
    );

    // NON_AUTHORIZING_READBACK warning must be present
    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "isolated status --json must contain NON_AUTHORIZING_READBACK warning"
    );

    let _ = std::fs::remove_file(&path);
}

/// actor status --json with a pre-seeded journal containing 3 entries:
/// 2 DirectToolCall entries (one with decision_gate_outcomes for governance count)
/// and 1 Synthesis entry (no governance).
/// Proves journal_summary correctly counts entries from persisted state.
#[test]
fn actor_status_json_isolated_with_persisted_entries() {
    let path = temp_isolated_journal_path("actor_status_persisted");
    let _ = std::fs::remove_file(&path);

    // Entry 1: DirectToolCall without governance
    let entry1 = serde_json::json!({
        "id": "smoke-status-001",
        "created_at": "2026-06-01T08:00:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "read a file",
        "response_summary": "file content returned",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_status_journal_count_test",
        "proposed_actions": null,
        "tool_call_intents": null,
        "decision_gate_outcomes": null,
        "risk_level": "low",
        "compute_routing": null,
    });

    // Entry 2: DirectToolCall WITH decision_gate_outcomes (governance)
    let entry2 = serde_json::json!({
        "id": "smoke-status-002",
        "created_at": "2026-06-01T08:01:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "write to file",
        "response_summary": "file written successfully",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_status_journal_count_test",
        "proposed_actions": null,
        "tool_call_intents": [{"tool": "write_file", "arguments": {"path": "/tmp/test.txt"}}],
        "decision_gate_outcomes": {"decision_status": "approved", "approval_state": "automatic"},
        "risk_level": "medium",
        "compute_routing": null,
    });

    // Entry 3: Synthesis (no tool calls, no governance)
    let entry3 = serde_json::json!({
        "id": "smoke-status-003",
        "created_at": "2026-06-01T08:02:00Z",
        "interaction_type": "synthesis",
        "prompt_summary": "summarize findings",
        "response_summary": "summary complete",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_status_journal_count_test",
        "proposed_actions": null,
        "tool_call_intents": null,
        "decision_gate_outcomes": null,
        "risk_level": null,
        "compute_routing": null,
    });

    // Write all 3 entries as JSONL
    let jsonl = format!(
        "{}\n{}\n{}\n",
        serde_json::to_string(&entry1).expect("entry1 json"),
        serde_json::to_string(&entry2).expect("entry2 json"),
        serde_json::to_string(&entry3).expect("entry3 json"),
    );
    std::fs::write(&path, jsonl).expect("write journal file");

    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "status", "--json"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor status --json with persisted entries");
    assert!(
        output.status.success(),
        "actor status --json (persisted) failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor status --json should be valid JSON");

    let journal = parsed
        .get("journal_summary")
        .expect("JSON should contain 'journal_summary'");
    let total = journal
        .get("total_entries")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let direct = journal
        .get("direct_tool_calls")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let governance = journal
        .get("governance_entries")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Expected: total=3, direct=2 (entries 1 and 2 are DirectToolCall),
    //           governance=1 (only entry 2 has decision_gate_outcomes)
    assert_eq!(total, 3, "total_entries should be 3 with 3 seeded entries");
    assert_eq!(
        direct, 2,
        "direct_tool_calls should be 2 (entries 1 and 2 are DirectToolCall)"
    );
    assert_eq!(
        governance, 1,
        "governance_entries should be 1 (only entry 2 has decision_gate_outcomes)"
    );

    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "must emit NON_AUTHORIZING_READBACK warning with persisted entries"
    );

    let _ = std::fs::remove_file(&path);
}

/// actor journal --json with 3 pre-seeded entries containing distinct
/// content (prompt/response/objective) and different interaction types.
/// Proves that the readback returns all entries with correct ordering,
/// proper field values, NON_AUTHORIZING_READBACK warning, and header
/// metadata (total_entries, displayed_entries).
#[test]
fn actor_journal_json_isolated_multiple_persisted_entries() {
    let path = temp_isolated_journal_path("actor_journal_multiple_persisted");
    let _ = std::fs::remove_file(&path);

    // Three entries staggered by 1 minute, all matching the default
    // actor-journal filter (DirectToolCall + objective=actor_run).
    // They vary in content (prompt, tool calls, governance) to prove
    // correct field readback of persisted state.
    let entry1 = serde_json::json!({
        "id": "multi-001",
        "created_at": "2026-06-01T08:00:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "Find all markdown files in the project",
        "response_summary": "Found 15 .md files under docs/ and root",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_run",
        "proposed_actions": null,
        "tool_call_intents": [{"tool": "search_files", "arguments": {"pattern": "*.md"}}],
        "decision_gate_outcomes": {"decision_status": "approved", "approval_state": "automatic"},
        "risk_level": "low",
        "compute_routing": null,
    });

    let entry2 = serde_json::json!({
        "id": "multi-002",
        "created_at": "2026-06-01T08:01:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "Read the project README for setup instructions",
        "response_summary": "README shows build steps and dependencies",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_run",
        "proposed_actions": null,
        "tool_call_intents": null,
        "decision_gate_outcomes": null,
        "risk_level": null,
        "compute_routing": null,
    });

    let entry3 = serde_json::json!({
        "id": "multi-003",
        "created_at": "2026-06-01T08:02:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "Evaluate code quality of the agent-core library",
        "response_summary": "Code quality looks solid with good test coverage",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_run",
        "proposed_actions": null,
        "tool_call_intents": null,
        "decision_gate_outcomes": null,
        "risk_level": "informational",
        "compute_routing": null,
    });

    // Write as JSONL
    let jsonl = format!(
        "{}\n{}\n{}\n",
        serde_json::to_string(&entry1).expect("entry1 json"),
        serde_json::to_string(&entry2).expect("entry2 json"),
        serde_json::to_string(&entry3).expect("entry3 json"),
    );
    std::fs::write(&path, jsonl).expect("write journal file");

    // Run actor journal --json --limit 10
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "journal", "--json", "--limit", "10"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor journal --json with persisted entries");
    assert!(
        output.status.success(),
        "actor journal --json (persisted) failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("actor journal --json should be valid JSON");

    // Header metadata
    assert_eq!(
        parsed
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        3,
        "total_entries should be 3"
    );
    let displayed = parsed
        .get("displayed_entries")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(displayed, 3, "displayed_entries should be 3 (no limit hit)");

    // Entries array
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("JSON should contain entries array");
    assert_eq!(entries.len(), 3, "should have 3 entries");

    // Verify each entry's core fields
    for (i, entry) in entries.iter().enumerate() {
        assert!(entry.get("id").is_some(), "entry {} should have id", i);
        assert!(
            entry.get("interaction_type").is_some(),
            "entry {} should have interaction_type",
            i
        );
        assert!(
            entry.get("objective").is_some(),
            "entry {} should have objective",
            i
        );
    }

    // Full ordered id sequence: newer-first (multi-003, multi-002, multi-001)
    let ids: Vec<&str> = entries
        .iter()
        .filter_map(|e| e.get("id").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(
        ids,
        vec!["multi-003", "multi-002", "multi-001"],
        "entries should be newer-first ordered, got: {ids:?}"
    );

    // Verify persisted content fields survive readback for each entry.
    // The expected ordering is newer-first: multi-003 (08:02Z), multi-002
    // (08:01Z), multi-001 (08:00Z).
    let content_expectations: Vec<(&str, &str, &str)> = vec![
        // (prompt_summary, response_summary, objective)
        (
            "Evaluate code quality of the agent-core library",
            "Code quality looks solid with good test coverage",
            "actor_run",
        ),
        (
            "Read the project README for setup instructions",
            "README shows build steps and dependencies",
            "actor_run",
        ),
        (
            "Find all markdown files in the project",
            "Found 15 .md files under docs/ and root",
            "actor_run",
        ),
    ];
    for (i, (entry, (exp_prompt, exp_response, exp_obj))) in
        entries.iter().zip(content_expectations.iter()).enumerate()
    {
        let got_prompt = entry
            .get("prompt_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let got_response = entry
            .get("response_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let got_obj = entry
            .get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let entry_id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        assert_eq!(
            got_prompt, *exp_prompt,
            "entry[{i}] id={entry_id}: prompt_summary mismatch"
        );
        assert_eq!(
            got_response, *exp_response,
            "entry[{i}] id={entry_id}: response_summary mismatch"
        );
        assert_eq!(
            got_obj, *exp_obj,
            "entry[{i}] id={entry_id}: objective mismatch"
        );
    }

    // NON_AUTHORIZING_READBACK warning
    let warning = parsed
        .get("readback_warning")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        warning.contains("NON_AUTHORIZING_READBACK"),
        "must contain NON_AUTHORIZING_READBACK warning"
    );

    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// actor journal — text-mode smoke with pre-seeded persisted entries (PR #259)
// ---------------------------------------------------------------------------

/// actor journal (text mode) with 3 pre-seeded entries containing distinct
/// content and different governance states.
///
/// Proves that text-mode readback displays persisted content safely, with
/// correct header, NON_AUTHORIZING_READBACK warning, newest-first ordering
/// of IDs and content, and no contamination from default local journal state
/// (the isolated path guarantees zero contamination).
#[test]
fn actor_journal_text_isolated_persisted_entries() {
    let path = temp_isolated_journal_path("actor_journal_text_persisted");
    let _ = std::fs::remove_file(&path);

    // Three DirectToolCall entries at 1-minute intervals, all with
    // objective="actor_run" so they pass the default text-mode filter.
    // They vary in prompt/response content, tool intents, and risk level
    // so we can verify correct field readback of persisted state.
    let entry1 = serde_json::json!({
        "id": "text-persist-001",
        "created_at": "2026-06-01T08:00:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "Search for outdated dependencies in Cargo.toml",
        "response_summary": "Found 3 outdated crates: serde, tokio, reqwest",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_run",
        "proposed_actions": null,
        "tool_call_intents": [{"tool": "search_files", "arguments": {"pattern": "Cargo.toml"}}],
        "decision_gate_outcomes": {"decision_status": "approved", "approval_state": "automatic"},
        "risk_level": "low",
        "compute_routing": null,
    });

    let entry2 = serde_json::json!({
        "id": "text-persist-002",
        "created_at": "2026-06-01T08:01:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "Check CI pipeline status for recent commits",
        "response_summary": "All CI checks passed on the latest commit",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_run",
        "proposed_actions": null,
        "tool_call_intents": null,
        "decision_gate_outcomes": null,
        "risk_level": null,
        "compute_routing": null,
    });

    let entry3 = serde_json::json!({
        "id": "text-persist-003",
        "created_at": "2026-06-01T08:02:00Z",
        "interaction_type": "direct_tool_call",
        "prompt_summary": "Review the latest pull request diff for approval",
        "response_summary": "PR looks clean, minor formatting nits noted",
        "provider": "smoke-test",
        "model": null,
        "objective": "actor_run",
        "proposed_actions": null,
        "tool_call_intents": null,
        "decision_gate_outcomes": null,
        "risk_level": "informational",
        "compute_routing": null,
    });

    // Write as JSONL
    let jsonl = format!(
        "{}\n{}\n{}\n",
        serde_json::to_string(&entry1).expect("entry1 json"),
        serde_json::to_string(&entry2).expect("entry2 json"),
        serde_json::to_string(&entry3).expect("entry3 json"),
    );
    std::fs::write(&path, jsonl).expect("write journal file");

    // Run actor journal (text mode) — default limit 10 covers all 3 entries
    let output = std::process::Command::new(ARPAGONA_BIN)
        .args(["actor", "journal"])
        .env("ARPAGONA_LLM_JOURNAL_PATH", &path)
        .output()
        .expect("failed to run actor journal (text) with persisted entries");
    assert!(
        output.status.success(),
        "actor journal (text) with persisted entries failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("valid utf-8");

    // 1. Header must be present
    assert!(
        stdout.contains("Actor Journal Readback"),
        "text output should contain header"
    );
    assert!(
        stdout.contains("3 entries total"),
        "text output should report 3 entries total"
    );

    // 2. NON_AUTHORIZING_READBACK warning must be present
    assert!(
        stdout.contains("NON_AUTHORIZING_READBACK"),
        "text output must contain NON_AUTHORIZING_READBACK warning"
    );

    // 3. Newest-first ordering: "08:02:00" (newest) must appear before
    //    "08:00:00" (oldest) because the entries are rendered in
    //    newest-first order. Also the display indices #3..#1 appear
    //    in that descending order.
    //    Note: text mode does NOT render entry IDs — it uses display
    //    indices (#3 = newest shown first, #1 = oldest shown last).
    let pos_0802 = stdout.find("08:02:00").unwrap_or(usize::MAX);
    let pos_0800 = stdout.find("08:00:00").unwrap_or(usize::MAX);
    assert!(
        pos_0802 < pos_0800,
        "newest timestamp (08:02:00) should appear before oldest (08:00:00)"
    );

    // 4. Persisted content must be readable in text output
    assert!(
        stdout.contains("Search for outdated dependencies in Cargo.toml"),
        "entry 1 prompt_summary should appear"
    );
    assert!(
        stdout.contains("Found 3 outdated crates: serde, tokio, reqwest"),
        "entry 1 response_summary should appear"
    );
    assert!(
        stdout.contains("Check CI pipeline status for recent commits"),
        "entry 2 prompt_summary should appear"
    );
    assert!(
        stdout.contains("All CI checks passed on the latest commit"),
        "entry 2 response_summary should appear"
    );
    assert!(
        stdout.contains("Review the latest pull request diff for approval"),
        "entry 3 prompt_summary should appear"
    );
    assert!(
        stdout.contains("PR looks clean, minor formatting nits noted"),
        "entry 3 response_summary should appear"
    );

    // 5. No contamination from default local journal state:
    //    The temp path is unique per invocation (process-id-scoped) and we
    //    created it fresh with exactly our 3 entries. No default path or
    //    global state leaked in.
    //    Also verify decision_gate and risk_level from entry 1 are rendered
    //    (the only entry with decision_gate_outcomes) — text mode renders
    //    decision_gate outcomes and risk_level for entries that have them.
    assert!(
        stdout.contains("decision_gate"),
        "entry with decision_gate_outcomes should render 'decision_gate' in text output"
    );
    assert!(
        stdout.contains("approved"),
        "entry with decision_status=approved should render 'approved' in text output"
    );

    // Clean up
    let _ = std::fs::remove_file(&path);
}
