# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — P3-4f delivered as PR #190)

**main is green:** ✅ Full workspace tests pass.

**PR #187** (`feat/c2-llm-governed-tool-call-wiring`) — **OPEN**, **MERGEABLE**, CI green. Awaits GONA merge.

**PR #189** (`feat/p3-10-compute-aware-delegation`) — **OPEN**, **MERGEABLE**, CI green. Awaits GONA merge.

**PR #190** (`feat/p3-4f-memory-aware-context-routing`) — **OPEN**, new. P3-4f delivered (stacked on #189's branch). Propagates compute route routing advice into the context assembly pipeline: `MemoryQueryRequest` now carries `compute_route_label` and `local_preferred` hints; `SimulatedContextAssembler` includes compute route in explanations; `run_cycle()` computes route before context assembly.

**DAILY_VALIDATION_BACKLOG.md:** 0 open entries.

## Next action

1. **GONA: merge PR #187** (C2 — LLM-governed tool-call wiring, CI green, based on latest main).
2. **Then GONA: merge PR #189** (P3-10 — Compute-aware delegation, CI green).
3. **Then GONA: merge PR #190** (P3-4f — Memory-aware context routing via Compute Reservoir, stacked on #189).
4. **Then — P3-13: Real adapter context assembly pipeline.** With compute-aware routing now embedded in `MemoryQueryRequest`, the existing real adapters (GraphMemoryAdapter, HolographicMemoryAdapter, ReservoirEchoAdapter, ToolRuntimeAdapter) should be updated to use the compute route hints to prioritize/filter context items. This makes context assembly genuinely compute-aware beyond the simulated prefix.
