# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**C3 — Prompt, response, decision and risk journaling is delivered as PR #132.**

The LLM interaction journal now captures prompt summaries, response summaries, provider/model metadata, proposed actions, tool-call intents, Decision Gate outcomes and risk levels with file-backed persistence and CLI readback.

### What C3 delivered

- **`crates/core/src/llm_journal.rs`** — `LlmJournal`, `LlmJournalEntry`, `LlmInteractionType` types with file-backed persistence
- **CLI integration** — `cognitive run --llm` automatically journals synthesis interactions
- **CLI readback** — `arpagona llm journal [--json]` displays recent LLM interaction traces
- **Cross-process persistence** — entries survive separate process invocations via `target/llm-journal.jsonl`
- **7 unit tests** in arpagona-agent-core: empty journal, add_synthesis, capacity eviction, recent entries, direct_tool_call with governance data, serialization round-trip
- **Configurable** via `ARPAGONA_LLM_JOURNAL_PATH` env var

### Safety invariants

- Journals store prompt/response _summaries_, not raw secrets
- Journaled data is evidence/debugging-only, never authorization
- No shell, browser, email, secrets or unrestricted write tools added
- No Decision Gate bypass

### Proof to seek before merging PR #132

- [x] `cargo fmt -- --check` — clean
- [x] `cargo check` — clean (only pre-existing warnings)
- [x] `cargo test --workspace` — 600+ tests pass
- [ ] CI checks pass on GitHub
- [ ] PR review approval
- [ ] Merge per repository policy

## Next action

**Track C Step C4 — Compute Reservoir model routing.**

Integrate Compute Reservoir to choose between local, cloud, small and large model strategies in the cognitive run LLM path.

### Target chain

```
Objective / Task -> ComputeRequirement -> ComputeReservoir -> ModelRoute(local/cloud/small/large) -> Explanation -> Audit context
```

### Required properties (from AGENT_FOCUS_LOOP.md)

- Model route selection must be explainable
- Cost/latency/privacy trade-offs should be represented where practical
- Local-first preference should be expressible
- Route selection does not itself authorize tool execution
- Audit/readback should show why the model strategy was chosen

### Required safety boundaries

- Do not add shell/browser/email/secrets access
- Do not treat route selection as action authorization
- Do not add autonomous scheduling
- Do not add broad product roadmap items in the implementation PR
