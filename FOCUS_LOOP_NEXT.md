# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: add CLI read-only supervision surface for executor registry state.
Why: the `ExecutorRegistry` now supports `set_state()`/`get_state()` and has API endpoints, but operators have no CLI command to inspect executor readiness. Adding `arpagona executor list [--json]` and `arpagona executor inspect <id> [--json]` will allow runtime executor state inspection without running the API server.
Proof to seek: `cargo run --bin arpagona -- executor list --json` returns structured JSON with executor_id, executor_state, and supported_action_types; `cargo run --bin arpagona -- executor inspect noop-executor` shows slot details.
Do not: add executor state mutation (no `executor set-state` command), add real execution, modify NoopExecutor behavior, or bypass the Decision Gate.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
