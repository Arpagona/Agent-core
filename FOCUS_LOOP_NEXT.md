# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Track A Phase 3 — HTTP/SSE transport.** PR #112 (`feat/a3-http-sse-transport`) is now open with CI running. Track A Phase 3 is done this run.

The alternation was Track A (this run), so the next step is Track B.

**After PR #112 merges, advance Track B Step B3 — optional local embeddings for semantic generalization.** This adds optional semantic embedding support (fastembed or similar) to the Holographic Memory crate, enabling concept-level generalization beyond symbolic keyword matching.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- Optional embedding dependency (e.g., fastembed as an optional cargo feature)
- Integration test proving resonance with semantic overlap (not just keyword match)
- CLI command or flag to enable/use embeddings
- Guard for when embeddings are not available (graceful fallback to symbolic-only mode)

Do not: add vector databases, LLM calls, Decision Gate bypasses, execution capabilities, or SurrealDB persistence for embeddings.
