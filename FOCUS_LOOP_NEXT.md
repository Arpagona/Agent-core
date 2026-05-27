# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

Start each run checking the last handoff. If last was Track A, pick Track B next (and vice versa). P1 (open PRs) takes priority over alternation.

## Next action

Next pass should: Track A — Merge PR #105 (native MCP server Phase 1), then start Phase 2: add governance layer wrapping tools/call through DecisionGate before tool execution.

Why: MCP Phase 1 crate and CLI exist, PR #105 is open and mergeable; Phase 2 governance is the next critical brick. After this, alternate to Track B (Holographic Memory integration with conversation-memory).

Proof to seek: after merge, `cargo run --bin arpagona -- mcp-server` starts and responds to `initialize` + `tools/list`. Then a new branch `feat/mcp-phase2-governance` exists with a GovernanceLayer struct that intercepts `tools/call`, creates a ProposedAction, runs it through `evaluate_proposed_action`, and only executes if the decision is approved.

Do not: add HTTP transport, resources, prompts, or list_changed notifications. Keep governance on stdio transport only. Do not modify existing Tool Runtime or Tool Registry behaviour.
