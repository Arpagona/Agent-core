# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — P3 hygiene fix delivered as PR #191)

**main is green:** ✅ Full workspace tests pass (0 warnings).

**PR #187** (`feat/c2-llm-governed-tool-call-wiring`) — **OPEN**, **MERGEABLE**, CI green. Awaits GONA merge.

**PR #188** (`docs: update handoff`) — **OPEN**, **MERGEABLE**, CI green. Awaits GONA merge.

**PR #189** (`feat/p3-10-compute-aware-delegation`) — **OPEN**, **MERGEABLE**, CI green. Awaits GONA merge.

**PR #190** (`feat/p3-4f-memory-aware-context-routing`) — **OPEN**, **MERGEABLE**, CI in progress. Awaits GONA merge.

**PR #191** (`fix: remove unused fields from LlmProposalGenerator`) — **NEW**, CI pending. DEEP hygiene fix: removes pre-existing `dead_code` warning by dropping unused `default_workspace_id`/`default_agent_id` fields.

**DAILY_VALIDATION_BACKLOG.md:** 0 open entries.

## Next action (GONA)

1. **GONA: merge PR #187** (C2 — LLM-governed tool-call wiring, CI green, based on latest main).
2. **Then GONA: merge PR #189** (P3-10 — Compute-aware delegation, CI green).
3. **Then GONA: merge PR #190** (P3-4f — Memory-aware context routing via Compute Reservoir, stacked on #189).
4. **Then GONA: merge PR #191** (hygiene: remove unused fields, CI should be green).
5. **Then — P3-13: Real adapter context assembly pipeline.** With compute-aware routing now embedded in `MemoryQueryRequest`, the existing real adapters (GraphMemoryAdapter, HolographicMemoryAdapter, ReservoirEchoAdapter, ToolRuntimeAdapter) should be updated to use the compute route hints to prioritize/filter context items. This makes context assembly genuinely compute-aware beyond the simulated prefix.

**Note:** DEEP governance boundary prevents merging. All merge actions require GONA or Thibaud.
