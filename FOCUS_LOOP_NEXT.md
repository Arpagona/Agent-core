# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement permission model / policy checks before any real executor.

Why: the execution capability registry now provides deterministic metadata for every action type. The next step is a `PolicyEngine` that consumes registry data (risk level, required permissions, resource kinds) plus context (agent role, workspace scope, user intent) to gate dry-run eligibility and future execution. This is the last missing layer before any executor can be designed.

Proof to seek: `arpagona action capability list` shows available executors; a `policy check` command can evaluate "can agent-X dry-run action-type-Y with risk-level-Z on workspace-W".

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. The policy engine is purely declarative.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
