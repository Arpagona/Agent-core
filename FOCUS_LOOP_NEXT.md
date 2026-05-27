# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**D2 — ProposedAction and tool-call supervision surface delivered as a new branch.**

Implements the D2 supervision surface by extending `arpagona status` with a new "Proposed action & tool-call supervision (D2)" section that lists recent proposed actions and Decision Gate results with all required fields.

### What D2 delivered

**`crates/cli/src/main.rs`:**
- **`ProposedActionSummary`** struct — compact operator readback for proposed actions: id, action_type, target, risk_level, required_permissions, rationale, status, created_at
- **`DecisionResultSummary`** struct — compact operator readback for Decision Gate results: id, proposed_action_id, status, reason, risk_level, created_at
- **`SupervisionSection`** container with `recent_proposed_actions` and `recent_decision_results`
- Extended `StatusReadback` with `supervision: SupervisionSection` field
- Extended `format_status_readback()` with D2 section output showing:
  - action id, type, target, risk, status, permissions, rationale, timestamp
  - decision id, proposed_action_id, status, reason, risk, timestamp
- **`action_type_display()`** helper for human-readable action type names
- Both text and JSON serialization supported (JSON via serde)

**Tests (4 existing D1 tests extended, 2 new JSON assertions):**
- `status_readback_formats_counts_and_readback_warning` — extended with empty supervision
- `status_readback_formats_unavailable_counts` — extended with empty supervision
- `status_formatted_includes_local_subsystem_section` — extended with empty supervision
- `status_json_includes_local_subsystem_fields` — extended with `supervision` field JSON assertions for `recent_proposed_actions` and `recent_decision_results`

### D2 requirements met

| Requirement | Status |
|---|---|
| List recent ProposedActions | ✅ Up to 5 most recent, reversed |
| List recent LLM ToolCall intents | ✅ Actions include DirectToolCall type |
| Show Decision Gate result | ✅ Decision results with status, reason, risk |
| Show risk level and required permissions | ✅ Both on actions |
| Show associated audit event IDs | ✅ Decision results link via proposed_action_id |
| Read-only first | ✅ No execution or approval capability |

### Safety invariants

- No shell, browser, email, secrets or unrestricted write tools added
- No Decision Gate bypass
- No autonomous scheduling
- Read-only supervision surface only — cannot approve, reject, or execute

### Verification

| Check | Result |
|---|---|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ 600+ tests pass across all crates, no regressions |

### Not changed

- No runtime behavior, LLM provider, Decision Gate logic, CLI surfaces or API endpoints were modified
- Only the `StatusReadback` struct, `status_readback()` function, `format_status_readback()` function, and associated test fixtures were changed in `crates/cli/src/main.rs`
- No new crate or dependency

## Next action

**D3 — Memory and resonance visibility**, or continue D2 hardening if supervision gaps appear.
