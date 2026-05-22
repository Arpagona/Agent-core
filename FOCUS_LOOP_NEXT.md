# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add the smallest persisted-memory inspection follow-up that lets an operator inspect an approved FailureInsight artifact by id after a configured local persistence run.

Why: the demo now proves and documents how to rerun the governed learning loop, but operator inspection still stops at the self-contained demo output rather than a reusable persisted-artifact readback command.

Proof to seek: `cargo run -q --bin arpagona -- memory demo failure-insight --json` plus a new readback command or test proving the approved FailureInsight id can be inspected without broad mutation.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, MCP/browser automation, or readback-as-authorization behavior.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
