# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Execution attempt audit hardening / executor policy alignment.

Why: the executor registry, capability registry, policy engine, and dry-run layer form a complete pipeline. However, audit events for execution attempts currently reuse the generic `AuditEventType::DecisionCreated` variant. The next step is to add dedicated `AuditEventType` variants (`ExecutionBlocked`, `ExecutionDisabled`, `ExecutionRequested`) and ensure every pipeline step (policy check, executor resolution, dry-run, execution attempt) produces a rich, queryable audit trail with full metadata.

Proof to seek: `AuditEventType` has variants `ExecutionBlocked` and `ExecutionDisabled`; an `arpagona audit list` for an execute attempt shows dedicated event types with `executor_id`, `policy_decision`, and `capability` metadata.

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. This is purely audit hardening.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
