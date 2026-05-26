# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement executor readiness states / disabled-by-default executor slots.

Why: the audit trail now has dedicated event types for every execution activity. The next step is to define states like `Disabled`, `Ready`, `Blocked` for executor instances, and add a slot-based system where multiple executors can be registered but remain disabled by default. This prepares for the eventual enabling of specific executors without global risk.

Proof to seek: `ExecutorState` enum exists; `ExecutorRegistry::register()` accepts an optional state; `resolve()` filters disabled executors; tests prove disabled executors cannot execute and blocked executors produce `ExecutionBlocked`.

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. Executors remain disabled by default.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
