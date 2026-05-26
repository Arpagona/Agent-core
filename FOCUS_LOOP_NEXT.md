# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: Add `AuditEventType::ExecutionRequested` and `AuditEventType::ExecutionStarted` to the `AuditTraceSummary::has_execution_event` in-memory audit event query so that operator-initiated sandbox runs and dry-run requests are included in audit trace summaries alongside execution events.

Why: the execution audit hardening is complete. The next gap is audit query completeness — sandbox and dry-run requests (EventType::ExecutionRequested) and in-progress real execution (ExecutionStarted) are currently excluded from has_execution_event, making summaries incomplete for operators reviewing execution traces.

Proof to seek: `cargo test --workspace` shows 319+ tests passing including `has_execution_event_includes_new_variants` for ExecutionBlocked/ExecutionDisabled/ExecutionRequested, plus a new test proving ExecutionStarted is also counted in has_execution_event.

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. This is purely audit query completeness.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
