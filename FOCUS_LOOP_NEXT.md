# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Track B Step B4 — SQLite persistence for holographic memory.** PR #115 created this run implementing `SqliteHolographicMemoryStore`.

Wait for CI on PR #115. If CI passes green, auto-merge. Then advance to **Track B Step B5 — periodic consolidation + redundant trace merging.**

B5 consolidates redundant traces in the holographic memory store by merging traces with overlapping signatures within a configurable time window, reducing memory bloat from repeated similar observations.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- `consolidate_traces(window_minutes, similarity_threshold)` method on `HolographicMemoryStore` trait
- Default no-op implementation on `InMemoryHolographicMemoryStore`
- SQLite implementation that queries for trace pairs with similar signatures within a time window, merges keywords/concepts/entities, updates activation counts, removes duplicates
- CLI command `memory holographic consolidate [--window 60] [--threshold 0.7]`
- Tests proving merged traces retain key content and redundant traces are removed

Do not: add real execution, shell access, LLM calls, browser automation, email sending, or SurrealDB persistence.
