# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: integrate the executor list/inspect commands into the CLI's offline mode by allowing direct ExecutorRegistry construction from the core crate, so operators can inspect executor readiness without running the API server.
Why: the CLI executor commands added in this session work via HTTP to the API server, but the handoff goal was offline inspection. After that, advance the P2 General Cognitive Work Loop by adding the `--observe` flag support to generate cognitive observations from the work cycle output.
Proof to seek: `cargo run --bin arpagona -- executor list --json` works without the API server; `cargo run --bin arpagona -- cognitive run --objective "test" --observe --json` returns observations.
Do not: add real execution, modify NoopExecutor behavior, bypass the Decision Gate, or add executor state mutation.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
