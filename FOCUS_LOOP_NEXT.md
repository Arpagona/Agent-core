# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: add an `--offline` flag to `executor inspect` command (mirroring the pattern just added for `executor list`) and add an end-to-end integration test for the offline executor commands that constructs an `ExecutorRegistry`, registers a test executor in Ready state, and verifies both list and inspect produce correct output without requiring an API server.
Why: the offline executor inspection support is complete for list but the inspect path hasn't been verified end-to-end in a replicable test, and the `inspect --offline` flag is consistent but the existing `inspect --offline` tests only cover parser parsing, not runtime behavior.
Proof to seek: `cargo test --workspace` passes with a new integration test proving both `executor list --offline` and `executor inspect --offline` produce correct executor metadata without an API server running.
Do not: add real execution to any executor, bypass the Decision Gate, or modify any executor behavior in the core crate.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
