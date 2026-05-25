# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The priority queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: perform P0/P1 hygiene around the `--description` propagation chain.

Why: several PRs/branches have recently touched the same `--description` / FailureInsight readback topic; the focus loop must avoid duplicate work and finish the single active consolidated PR before opening anything new.

Proof to seek: #77 is either merged safely into `main`, or explicitly reported as blocked; older superseded `--description` PRs/branches are identified as superseded or left untouched with a reason; `main` remains green.

Do not: create a new `--description` branch, start Tool Runtime Observation work, or add a new feature before the open PR/branch hygiene is resolved.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
