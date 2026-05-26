# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement human review queue / proposal lifecycle states for the CLI.

Why: proposals currently flow through the cognitive pipeline (candidate → proposed → decision gate → audit) but there is no persistent UI to review, approve, block, or escalate proposals. Adding a CLI surface to view pending proposals, change their states (PendingDecision → Approved → Blocked → NeedsHumanApproval), and track which human reviewed them creates the first human-in-the-loop supervision path.

Proof to seek: `arpagona action review --list --status pending_decision` lists proposals, `arpagona action review --approve <id>` changes status and creates audit event, `arpagona action review --block <id> --reason "..."` blocks with audit trail.

Do not: skip the Decision Gate, auto-execute approved actions, add LLM calls, or modify core domain types. All state transitions must go through the existing API server endpoints and produce audit events.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
