# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 1-5 complete)
- **Track B** — Holographic Memory (Steps B1-B6 complete)

P1 (open PRs) takes priority over alternation.

## Next action

**Track B Step B5 — Periodic consolidation + redundant trace merging.**

Both Track A Phase 5 (PR #117) and Track B Step B6 (governance via DecisionGate) were completed in this run.

Step B5 adds periodic consolidation of redundant holographic memory traces. Requires:
- A `consolidate(project_id)` method that finds similar traces (by resonance above a configurable threshold) and merges redundant ones into a single consolidated trace with combined `linked_memory_ids` and aggregated metadata
- Tests proving consolidation produces correct merged traces and preserves linked context
- No automatic scheduling — consolidation is an explicit operation gated by explicit invocation

Do not: add real execution, shell access, LLM calls, browser automation, email sending, or SurrealDB persistence beyond existing usage.
