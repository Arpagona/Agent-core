# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: adopt the demo snapshot command as the standard cross-invocation readback proof and add a CLI integration test that runs the snapshot-then-read cycle end-to-end.

Why: the snapshot path now proves FailureInsight readback survives across separate process runs (pure Rust JSON persistence, no native deps). The next step is to automate the proof as a `cargo test` or CLI integration test so every CI run verifies cross-invocation readback without manual invocation.

Proof to seek: a passing test that calls `memory demo failure-insight --snapshot-path` then `memory demo snapshot-read <path>` and asserts the readback JSON matches expected fields.

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
