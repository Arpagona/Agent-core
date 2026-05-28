# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-30 DEEP implementation — P3-1 merged)

**main is green:** ✅ PR #169 (P3-1 Neutral Orchestrator V0 domain contract) was merged after all checks passed.

**Open PRs:** None known after PR #169 merge.

**Phase 3 progress:**
- P3-0 (Roadmap definition): ✅ Completed in #168
- **P3-1 (Domain contract): ✅ Completed in #169**
- P3-2 (Deterministic loop skeleton): 🔜

## Next action

**P3-2 — Neutral Orchestrator V0 deterministic loop skeleton.**

Implement a deterministic in-process skeleton in a dedicated crate (e.g. `crates/neutral-orchestrator`) that wires the existing bricks:

- accepts a bounded `ObjectiveInput`;
- assembles a synthetic/advisory `ContextBundle`;
- requests or simulates compute route advice via `ComputeRouteRequest`/`ComputeRouteResult`;
- creates a `ProposalRequest`;
- sends any proposed action/tool-call intent through the Decision Gate;
- records an `AuditEvent`-linked `OrchestratorOutcome`;
- exposes readback data for CLI/MCP later;
- tests prove blocked, allowed-simulation and malformed paths.

The skeleton must remain deterministic and in-process — no external effects, no scheduler, no approval semantics.
