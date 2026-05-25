# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: create a self-contained shell script or Makefile target (`make demo`) that runs the complete governed FailureInsight demo path end-to-end: `failure-insight --snapshot-path` writes a snapshot, `snapshot-read` proves cross-invocation readback, `snapshot-list` proves discovery, and the script exits with status 0 only if all assertions pass.

Why: the snapshot discovery surface is now complete (write, read, list), but there is no single repeatable command that proves the full governed learning loop in one invocation without manual multi-step orchestration.

Proof to seek: `./scripts/demo-full-loop.sh` exits 0, and its output contains "ALL CHECKS PASSED" or equivalent, showing signal → proposal → decision → audit → persistence → readback → discovery.

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
