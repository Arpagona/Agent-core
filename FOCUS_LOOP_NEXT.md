# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Track B Step B4 — SQLite persistence for holographic memory.** PR #113 documenting Track B Step B3 (optional local embeddings) is created. Track B Step B3 is done this run.

The alternation was Track B (this run), so the next step is Track A.

**After PR #113 merges, advance Track A Phase 2 — DecisionGate governance before MCP tools/call.** This adds a governance layer to the MCP server that wraps every tool call through `DecisionGate` before allowing execution, following the `mcp-governance-layer-pattern.md` reference.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- Wrapped `tools/call` dispatch to evaluate via `evaluate_tool_call` helper
- GovernanceDecision enum returned from tool call (Approved/Blocked/NeedsHumanApproval)
- Audit trail entries for each governed tool call
- Tests proving tool calls are blocked without permission, approved with permission, and produce audit events
- Graceful handling of `NeedsHumanApproval` states

Do not: add real execution, shell access, LLM calls, browser automation, email sending, or SurrealDB persistence.
