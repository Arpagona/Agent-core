# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — PR #187 awaiting GONA merge, Phase 3 P3-0–P3-9 delivered)

**main is green:** ✅ ~847 tests, 0 failures across full workspace.

**PR #187** (`feat/c2-llm-governed-tool-call-wiring`) — **OPEN**, **MERGEABLE**, CI green (2/2 checks pass). Directly based on latest `origin/main` (parent commit e9a5414). Awaits GONA merge.

**Phase 3 deliveries (all merged on main):**

| Milestone | Status |
|-----------|--------|
| P3-0 — Roadmap and contract definition | ✅ |
| P3-1 — Neutral Orchestrator V0 domain contract | ✅ |
| P3-2 — Deterministic loop skeleton | ✅ |
| P3-3 — CLI readback for orchestration state | ✅ |
| P3-4 — Memory-aware context integration design | ✅ |
| P3-4a — GraphMemoryAdapter | ✅ |
| P3-4b — CompressedCognitiveAttentionAdapter | ✅ |
| P3-4c — ReservoirEchoAdapter | ✅ |
| P3-4d — HolographicMemoryAdapter | ✅ |
| P3-4e — ToolRuntimeAdapter | ✅ |
| P3-5 — Cycle Trace V0 | ✅ |
| P3-6 — MultiAdapterContextAssembler | ✅ |
| P3-7 — Proposal routing + Decision Gate integration | ✅ |
| P3-8 — Proposal routing CLI surface (`--proposal-generator`) | ✅ |
| P3-9 — Orchestrator demo loop with documentation | ✅ |
| **Phase 3 total: 15 PRs merged** | ✅ |

**DAILY_VALIDATION_BACKLOG.md:** 0 open entries. All DV items closed/fixed.

## Next action

1. **GONA: merge PR #187** (C2 — LLM-governed tool-call wiring, 331 additions, 3 files, CI green, based on latest main).
2. **Then — P3-10: Compute-aware delegation.** Integrate the Compute Reservoir crate into the orchestrator cycle. The orchestrator currently uses a deterministic mock `ComputeRouteResult`. The real `arpagona-compute-reservoir` crate has types (`ComputeNodeId`, `ComputeCapability`, `ComputeResourceKind`) but is **never called** from the orchestrator. Deliverable: the orchestrator asks Compute Reservoir for routing advice with explainable cost/latency/privacy trade-offs, records the explanation in the cycle trace, and never treats compute allocation as action authorization.
