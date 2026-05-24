# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: document the cross-invocation demo snapshot readback procedure in `docs/failure-to-insight.md` and make it the standard operator demo recipe.

Why: the snapshot path now has passing unit tests and a CLI integration test. The next operator-facing step is to document the full verification procedure — `cargo run memory demo failure-insight --snapshot-path target/demo.json && cargo run memory demo snapshot-read target/demo.json` — as the standard way to prove the governed learning loop output survives serialization, file I/O, process restart and deserialization.

Proof to seek: a `docs/failure-to-insight.md` section titled "Cross-invocation readback verification" describing the exact one-liner command sequence and what to look for in the readback output.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, MCP/browser automation, personal/sensitive memory, readback-as-authorization behavior, or always-on native/unstable backend requirements.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
