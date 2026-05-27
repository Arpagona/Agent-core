# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

Start each run checking the last handoff. If last was Track A, pick Track B next (and vice versa). P1 (open PRs) takes priority over alternation.

## Next action

Next pass should: Track B Step B1 — Integrate Holographic Memory with `conversation-memory`. Encode conversation turns as `HolographicTrace` objects using the existing `arpagona-holographic-memory` crate's store/resonance API.

Why: This run delivered Track A Phase 2 (MCP DecisionGate governance, PR merged). Alternation says Track B next. Step B1 is the natural next Holographic Memory brick — it connects the pattern-resonance kernel to real conversation streams so traces can be stored and matched across actual cognitive cycles.

Proof to seek: `cargo test --workspace` green. A new PR `feat/holographic-memory-conversation-memory` exists with:
- Bridge logic in `crates/conversation-memory/` that converts conversation turns (user messages, assistant responses, tool results) into `HolographicTrace` objects
- Integration with the store: after each turn, the bridge adds a trace and optionally finds-similar traces
- Tests: conversation turns produce valid traces; resonance search across multiple turns finds related patterns; no mutation of existing conversation-memory or holographic-memory APIs
- Docs/CLI: at minimum, a new `arpagona holographic-memory from-conversation <session-id>` command or documented integration entry point

Do not: add LLM calls, embeddings, vector databases, external network calls, or persistence beyond the existing in-memory holographic store. Do not modify existing holographic-memory core kernel types. Do not add Decision Gate bypasses for memory writes.
