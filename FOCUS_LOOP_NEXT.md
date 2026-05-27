# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Track A Phase 3 — HTTP/SSE transport.** PR #111 (`feat/b2-recursive-memory-graph`) is Track B Step B2 (done this run). The alternation was Track B (this run), so the next step is Track A.

Phase 3 adds native HTTP transport and Server-Sent Events (SSE) support via an Axum endpoint `/mcp`. This enables remote MCP clients (not just stdio) to connect to the ARPAGONA MCP server.

Why: Phase A2 (persistent governance audit) is merged via PR #110. Phase B2 (recursive memory graph) is done via PR #111. The next step in the alternation is Track A Phase 3.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- Axum route at `/mcp` accepting JSON-RPC messages over HTTP POST
- SSE endpoint for server-to-client notifications (tools/list_changed)
- MCP protocol compatibility (JSON-RPC 2.0, methods: tools/list, tools/call)
- Tests for HTTP request/response roundtrip

Do not: add new tools, broaden execution, add LLM calls, bypass Decision Gate, or modify existing MCP tool governance.
