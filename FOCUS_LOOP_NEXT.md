# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: add an end-to-end integration test for `cognitive run --assess --govern --json` that proves the full P3 chain (CognitiveObservation -> FailureInsightCandidate -> ProposedAction -> DecisionGate -> Decision -> AuditEvent -> readback) works offline without the API server.
Why: the `--govern` flag was just added and works at runtime, but there is no automated test proving the governance chain produces correct Decision Gate decisions and AuditEvents end-to-end from cognitive work loop output.
Proof to seek: `cargo test --workspace` passes with a new test in `crates/cli/tests/` that invokes `cognitive run --assess --govern --json` via the `CARGO_BIN_EXE_arpagona` pattern and asserts `decision_count`, decision status, and the non-authorizing governance warning.
Do not: add real execution, bypass the Decision Gate, add persistence, modify executor behavior, or add new CLI flags.

Note: `crates/holographic-memory` now exists as an alpha Rust kernel (27 tests, JSON file persistence). Future sessions can explore integrating it with governance or conversation-memory, but it is independent of the current P3 governance path.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
