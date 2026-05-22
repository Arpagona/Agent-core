# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: replace the in-memory-only FailureInsight demo inspection with an explicitly configured local SurrealDB persistence/readback path if the adapter can support a safe file-backed configuration.

Why: `--inspect-id` now proves artifact-by-id inspection inside the governed local demo, but persistence remains scoped to the in-memory demo store rather than a reusable configured local database.

Proof to seek: `cargo run -q --bin arpagona -- memory demo failure-insight --json --inspect-id insight-demo-governed-learning-loop` plus a new configured local persistence command or test showing the same id can be inspected after a separate readback step.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, MCP/browser automation, personal/sensitive memory, or readback-as-authorization behavior.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
