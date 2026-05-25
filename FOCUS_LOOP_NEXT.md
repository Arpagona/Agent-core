# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Review and merge PR for P3 Cognitive Observation to Governed Learning if checks are green.

Why: the P3 bridge was recovered from the interrupted run and must be merged before starting another major runtime brick.

Proof to seek: PR exists, cargo fmt/check/test pass, --assess JSON includes failure_insight_candidates.

Do not: start scripts/demo-full-loop.sh or P4 Working Memory integration before P3 is merged.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
