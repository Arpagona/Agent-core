# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Step 1: Merge PR #110** (`feat/mcp-a2-persistent-governance-audit`) if CI is green. This is Track A Phase 2 refinement — persistent governance audit store for MCP DecisionGate decisions.

**Step 2: Track B Step B2 — Recursive memory graph.** After PR #110 is merged, switch to Track B. Step B2 is about building a recursive memory graph that follows `linked_memory_ids` in depth, enabling Holographic Memory to traverse related traces.

Why: The alternation was Track B (B1, done via PR #109), then Track A (A2 refinement, done via PR #110). The next step in the alternation is Track B Step B2.

Proof to seek: `cargo test --workspace` green. A new PR exists with:
- BFS/DFS traversal of HolographicTrace linked_memory_ids
- Configurable depth limit to prevent infinite loops
- CLI command to explore trace chains
- Tests proving correct traversal, cycle detection, and depth limiting

Do not: add vector databases, embeddings, LLM calls, persistence changes, Decision Gate bypasses, or execution capabilities.
