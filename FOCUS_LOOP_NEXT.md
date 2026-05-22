# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: review the merged or open `feat/failure-insight-memory-demo` PR, then add the smallest operator-facing follow-up that makes the governed FailureInsight learning demo easier to rerun or inspect locally.

Why: the recovered branch now proves the signal -> proposal -> decision -> audit -> approved persistence -> readback chain, but the next product step is making that proof more operator-friendly without widening mutation or authorization.

Proof to seek: `cargo run -q --bin arpagona -- memory demo failure-insight --json` plus the PR status for `feat/failure-insight-memory-demo`.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, or another unrelated schema-only field.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
