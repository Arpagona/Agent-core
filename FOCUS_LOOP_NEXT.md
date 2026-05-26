# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Complete execution audit query coverage.

Why: the execution pipeline now emits dedicated execution-related audit events, but every execution, sandbox, and dry-run request/completion/blocking event must be recognized by audit trace summaries and query helpers before adding executor readiness states.

Proof to seek: `cargo test --workspace` passes and tests prove every execution-related `AuditEventType` is recognized by `AuditTraceSummary::has_execution_event`, sandbox/dry-run events appear in operator trace summaries, blocked policy events appear in execution audit queries, and generic `DecisionCreated` is no longer required to detect execution activity.

Do not: add real execution, add LLM calls, add endpoints unless strictly required for readback, or change PolicyEngine, Decision Gate, Executor, or ExecutorRegistry behavior.

## Requirements

- Verify all execution-related variants are included in `AuditTraceSummary::has_execution_event`.
- Add missing variants if needed, especially:
  - `ExecutionRequested`
  - `ExecutionStarted`, if present or needed
  - `ExecutionBlocked`
  - `ExecutionDisabled`
  - `DryRunRequested`, if present or needed
  - `DryRunCompleted`, if present or needed
  - `DryRunBlocked`, if present or needed
- Add tests proving:
  - each execution-related audit variant is recognized by `has_execution_event`;
  - sandbox/dry-run events appear in operator trace summaries;
  - blocked policy events appear in execution audit queries;
  - generic `DecisionCreated` is no longer required to detect execution activity.
- Keep changes limited to audit event recognition, tests, and documentation.
- Update `PROJECT_STATUS.md`.
- Update this file with the next step:
  - executor readiness states / disabled-by-default executor slots.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
