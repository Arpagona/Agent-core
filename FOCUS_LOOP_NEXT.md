# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add an optional cross-process integration test for the demo snapshot path — run `memory demo failure-insight --snapshot-path` in one process, then run `memory demo snapshot-read` in a separate process to assert readback fields.

Why: the snapshot write/read path is implemented and manually testable, but there is no automated CI-proof cross-invocation test proving the governed FailureInsight memory output survives process restarts.

Proof to seek: a `#[test]` function (either in `crates/cli/src/main.rs` or a dedicated `tests/` integration test) that spawns the binary, writes a snapshot, reads it back, and asserts `readback_found: true`, `readback_audit_event_count >= 1`, and `evidence_only_token` is the canonical non-authorizing token.

Do not: add broad mutation, authorization, execution, SurrealDB backend config changes, feature flags, or external side effects beyond the snapshot file path.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
