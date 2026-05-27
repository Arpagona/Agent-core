# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Step 1: Merge PR #109** (`feat/b1-conversation-holographic-bridge`) if CI is green. This is Track B Step B1 — the holographic conversation bridge that encodes conversation turns as distributed-signature traces.

**Step 2: Track A Phase 2 refinement — Add governance audit persistence to MCP Phase 2.**
After PR #109 is merged, switch to Track A. Phase 2 (DecisionGate governance for MCP tools/call) was delivered but the governance decision is evaluated per call without persistent audit storage. Add audit event persistence so each governed MCP tool call produces a durable audit event.

Why: Track B B1 was just delivered. Alternation says Track A next. Phase 2 was the last Track A milestone but the audit trail is in-memory only — adding persistence makes the governance observable.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- Audit event persistence for MCP governance decisions (using the existing audit event types and graph-memory store)
- CLI or server endpoint that lists recent MCP governance decisions
- Tests proving audit events survive restart (via file-based persistence)

Do not: add new tools, change MCP transport, add LLM calls, modify existing holographic-memory or conversation-memory APIs, add Decision Gate bypasses, or expand execution capabilities.
