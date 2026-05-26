# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement the human review queue for governed proposals.

Why: P8/P9/P10 now produce contextualized, scored, sorted, and deduplicated `ProposedAction` entries, but operators still need a deterministic CLI/API surface to inspect and explicitly accept, reject, defer, or supersede proposals without executing them.

Proof to seek: `cargo test --workspace` passes and an integration test proves this flow: generate proposals with `cognitive run --objective "..." --assess --observe --propose --json`, list them through the review CLI, show one proposal by id, transition it to `Approved` or `Rejected`, and verify that an audit event is produced while no tool execution occurs.

Do not: add autonomous execution, treat `Approved` as executed, bypass the Decision Gate, add LLM calls, weaken PendingDecision defaults, or drop priority/dedup/batch metadata during review transitions.

## Implementation hints

Preferred lifecycle states for this pass:

- `PendingDecision` — default for newly generated proposals.
- `Approved` — human accepted the proposal as valid, but it is still non-executing.
- `Rejected` — human rejected the proposal, optionally with a reason.
- `Deferred` — human postponed the proposal, optionally with a reason.
- `Superseded` — proposal was replaced by a better or batched proposal.

Suggested CLI shape, adapt to existing conventions if needed:

```text
arpagona action review --list --status pending_decision
arpagona action review --show <proposal_id>
arpagona action review --approve <proposal_id> [--reason "..."]
arpagona action review --reject <proposal_id> [--reason "..."]
arpagona action review --defer <proposal_id> [--reason "..."]
arpagona action review --supersede <proposal_id> --by <proposal_id> [--reason "..."]
```

Required invariants:

- All newly generated proposals remain `PendingDecision` by default.
- Every lifecycle transition creates an audit event.
- Invalid transitions are rejected deterministically.
- Review state changes do not execute tools.
- Batched proposals preserve `merged_count`, `merged_proposal_ids`, aggregated summaries, score, band, risk, cost, and original context metadata.

Recommended tests:

- list pending proposals;
- show proposal details by id;
- approve/reject/defer/supersede transitions;
- invalid transition is blocked;
- transition creates audit event;
- approved proposal is not executed;
- batched proposal keeps dedup/batch metadata after review.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
