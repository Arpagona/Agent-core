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
