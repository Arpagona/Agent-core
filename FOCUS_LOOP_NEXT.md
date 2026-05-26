# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement executor registry with only disabled/noop executors.

Why: the `Executor` trait and `NoopExecutor` exist. The next step is a registry that maps `executor_id` to `Box<dyn Executor>`, registers `NoopExecutor` as the only entry, and exposes a `resolve(action_type, risk_level) -> Option<&dyn Executor>` lookup. Integrate with the capability registry so `executor_id` in capability entries references registered executors.

Proof to seek: a `ExecutorRegistry` struct exists with `register()` and `resolve()` methods; `resolve(ReadMemory, Low)` returns the `NoopExecutor`; an unknown executor_id returns `None`.

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. The registry only contains `NoopExecutor`.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
