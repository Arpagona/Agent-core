# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: expose executor state management through API server endpoints.
Why: the `ExecutorRegistry` now supports `set_state()` and `get_state()` for managing executor readiness (Disabled → Ready → Blocked), but operators have no runtime API surface to query or change executor states. Adding `POST /executors/{id}/state` and `GET /executors` endpoints will allow manual promotion/demotion of executor readiness at runtime.
Proof to seek: `curl -X POST localhost:3000/executors/noop-executor/state -H 'Content-Type: application/json' -d '{"state":"ready"}'` returns 200 and `executor_state` is `"ready"` in the response; `GET /executors` lists all registered executors with their current state.
Do not: add real execution, modify NoopExecutor behavior, add autonomous state transitions, or introduce API endpoints that could bypass the policy engine or Decision Gate.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
