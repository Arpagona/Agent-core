# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

Start each run checking the last handoff. If last was Track A, pick Track B next (and vice versa). P1 (open PRs) takes priority over alternation.

## Next action

Next pass should: Track A Phase 2 — Add DecisionGate governance layer to the MCP server's `tools/call` handler. Wrap each tool call through `evaluate_proposed_action` before execution. Return structured governance errors when blocked.

Why: PR #103 (holographic-memory CLI, Track B) and PR #106 (demo-full-governed-loop) are both merged. Alternation says Track A next. Phase 2 governance is the natural next MCP brick — it makes the server safe for multi-agent use.

Proof to seek: `cargo test --workspace` green. A new PR `feat/mcp-phase2-governance` exists with:
- A `GovernanceLayer::evaluate_tool_call()` that creates `ProposedAction { action_type: ProposeToolUse, ... }` and runs it through the DecisionGate
- `tools/call` handler checks governance before ToolRuntime execution
- Tests: approved tool calls execute; missing permission blocks; governance summary is readable in the error response

Do not: add HTTP/SSE transport, resources, prompts, or list_changed notifications. Do not modify existing Tool Runtime, Tool Registry, or cognitive loop behavior. Do not add LLM calls.
