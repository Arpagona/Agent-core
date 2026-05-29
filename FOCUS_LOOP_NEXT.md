# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — C2 LLM-governed tool-call wiring merged)

**main is green:** ✅ ~878 tests, 0 failures across full workspace.

**C2 delivered (via PR #187, rebased & merged by DEEP):**
- Added `request_tool_call_from_llm()` in `crates/llm/src/lib.rs`: standalone async function that calls the LLM provider (mock/openai/ollama) and returns a `ToolCallIntent` for the given objective.
- Added `--govern-tool` flag to `cognitive run --json`: when set, requests a tool-call intent from the LLM provider, routes through `govern_tool_call()` → Decision Gate, executes approved calls through bounded Tool Runtime, journals the full trace (intent → decision → result → observation) in the LLM journal.
- 3 new LLM crate tests, 2 new CLI parser tests.

**Also merged this run:**
- H2: CLI security boundary verification + .git/ file block fix (PR #192)
- fix: removed unused LlmProposalGenerator fields, zero warnings (PR #193)

Target chain proven:
```text
LLM ToolCall Intent -> Decision Gate -> Tool Runtime -> Observation -> LLM Journal
```

Usage:
```bash
# Mock provider (deterministic read_file on PROJECT_STATUS.md):
cargo run -q -- cognitive run --objective "Read project status" --govern-tool --json

# With explicit LLM provider:
ARPAGONA_LLM_PROVIDER=mock cargo run -q -- cognitive run --objective "Analyse le code" --govern-tool --json
```

## Next action

**Phase 3 — Neutral Orchestrator V0 integration.** With C2 wire-up merged and all Phase 2 milestones delivered, the focus loop should now re-engage Phase 3: bounded Neutral Orchestrator integration — particularly the `--proposal-generator` integration tests and operator readback surfaces for orchestrator state.

Also: merge open Phase 3 PRs (#189 P3-10 compute-aware delegation, #190 P3-4f memory-aware context routing) after verifying they rebase cleanly and tests pass.
