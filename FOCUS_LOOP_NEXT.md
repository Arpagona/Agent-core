# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Track A Phase 4 — MCP Resources + Prompts.** PR #114 documenting Track A Phase 4 is being created this run.

The alternation was Track A (this run), so the next step is Track B.

**After PR #114 merges, advance Track B Step B4 — SQLite persistence for holographic memory.** This adds a durable SQLite storage backend for the holographic memory store, enabling traces to survive server restarts. Follow `crates/holographic-memory` crate patterns.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- SQLite-backed `HolographicMemoryStore` implementation using `rusqlite`
- Schema creation (traces table with id, project_id, signature_json, created_at columns)
- CRUD operations (store, retrieve by resonance, delete by project)
- Existing in-memory store kept as default; SQLite store enabled via constructor parameter
- Tests proving persistence across store drop/reopen cycles
- No changes to existing resonance or signature logic

Do not: add real execution, shell access, LLM calls, browser automation, email sending, or SurrealDB persistence.
