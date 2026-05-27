# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**C2 — Governed direct tool-calling by the LLM is delivered as PR #131.**

The `govern_and_execute_tool_call()` bridge in `crates/runtime/src/governed_tool_executor.rs` now connects:
- ToolCallIntent → Decision Gate → Tool Runtime → Observation → Audit

### What C2 delivered

- `govern_and_execute_tool_call()` — full governed tool-call chain
- `GovernedToolCallResult` — structured output carrying Decision + ToolExecutionResult
- 9 tests proving allowed, blocked, malformed, safety and non-authorizing paths
- Inherited foundation: `ToolCallIntent`, `ActionType::DirectToolCall`, `govern_tool_call()`
- All safety invariants from the C2 specification verified

### Proof to seek before merging

- [ ] `cargo fmt -- --check`
- [ ] `cargo check`
- [ ] `cargo test --workspace`
- [ ] PR review approval
- [ ] Merge per repository policy

## Next action

**Track C Step C3 — Prompt, response, decision and risk journaling.**

After C2 (governed tool-calling) is merged, the next step is to make LLM interactions auditable after the fact.

### Target properties

- Journal prompt summaries (what was sent to the LLM)
- Journal response summaries (what the LLM returned)
- Journal provider/model metadata (which model, which provider)
- Journal proposed actions, direct tool-call intents, Decision Gate outcomes and risk levels
- Preserve enough information for debugging without leaking secrets
- Support CLI or MCP readback for recent LLM interaction traces

### Required safety boundaries

- Do not add shell/browser/email/secrets access
- Do not treat journaled data as authorization
- Do not add autonomous scheduling
- Do not add broad product roadmap items in the implementation PR
