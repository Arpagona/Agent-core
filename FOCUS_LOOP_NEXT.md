# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-29 GONA arbitration — Phase 3 selected)

**main is green:** ✅ PR #167 was merged after both GitHub CI checks passed.

**Phase 2 delivery status:** All C1-C5, D1-D3+D5, E1-E5, H1 milestones confirmed complete ✅.

**DV backlog:** All DV-2026-05-28-* entries closed ✅.

**Open PRs:** None known after PR #167 merge, unless a newer PR appears before the next run.

**Phase 3 GONA decision:** Phase 3 starts with a bounded Neutral Orchestrator track. `docs/phase3-roadmap.md` defines the queue. Do not integrate `compressed-cognitive-attention` into the runtime loop until memory/context semantics are designed.

## Next action

**P3-1 — Neutral Orchestrator V0 domain contract.**

Create one bounded implementation PR that adds the smallest pure domain contract for orchestrated work cycles.

Expected shape:

```text
ObjectiveInput
  -> OrchestratorContextRequest
  -> ContextBundle(advisory)
  -> ComputeRouteRequest
  -> ProposalRequest
  -> ProposedAction or ToolCallIntent
  -> Decision Gate
  -> Audit-linked OrchestratorOutcome
```

Acceptance criteria:
- Add pure serializable domain types first; prefer a dedicated crate or clearly bounded module only if that fits the current crate layout.
- No execution, provider calls, scheduler behavior, approval semantics, browser, shell, email, secrets, unrestricted writes, or hidden autonomy.
- Include explicit IDs linking objective, context bundle, compute route, proposal, decision and audit event.
- Tests must prove context, memory recall and compute route are advisory only and cannot authorize actions.
- Update `PROJECT_STATUS.md` after the change.
- Run required verification for code changes: `cargo fmt -- --check`, `cargo check`, `cargo test --workspace`.
