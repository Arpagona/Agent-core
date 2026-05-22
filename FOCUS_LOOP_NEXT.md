# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: recover the completed local work on branch `feat/failure-insight-memory-demo`, push it, create a PR to `main`, and only then continue implementation.

Why: the previous cron completed and locally committed the repeatable governed FailureInsight memory demo, but hit the tool-call limit before GitHub push/PR creation.

Proof to seek: `git status --short --branch`, `git log -1 --oneline`, `git push -u origin feat/failure-insight-memory-demo`, a PR URL, and the demo command `cargo run -q --bin arpagona -- memory demo failure-insight --json` passing.

Do not: reimplement the same demo from scratch, discard the local commit, or start a new feature before preserving the completed branch.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.