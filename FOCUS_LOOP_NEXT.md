# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Step 1: Merge PR #108** (`fix/docs-executor-mcp-server`) if CI is green. This is a documentation-only fix for DV-2026-05-27-004 (CLI docs coverage).

**Step 2: Track B Step B1** — Integrate Holographic Memory with `conversation-memory`. After PR #108 is merged, switch to `main`, pull, and proceed with Track B Step B1: create a bridge that encodes conversation turns as `HolographicTrace` objects using the `arpagona-holographic-memory` crate's store/resonance API.

Why: Track A Phase 2 (MCP DecisionGate governance) was the last feature run. Alternation says Track B next. Step B1 is the natural next Holographic Memory brick.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- Bridge logic in `crates/conversation-memory/` that converts conversation turns (user messages, assistant responses, tool results) into `HolographicTrace` objects using the holographic-memory distributed-signature crate
- Integration with the holographic-memory store: after each turn, the bridge adds a trace and optionally finds-similar traces
- Tests: conversation turns produce valid traces; resonance search across multiple turns finds related patterns; no mutation of existing conversation-memory or holographic-memory APIs
- CLI: extend `memory holographic` with a `from-conversation` subcommand accepting turn data via file or stdin, or a documented integration entry point

Do not: add LLM calls, embeddings, vector databases, external network calls, or persistence beyond the existing in-memory holographic store. Do not modify existing holographic-memory core kernel types. Do not add Decision Gate bypasses for memory writes.
