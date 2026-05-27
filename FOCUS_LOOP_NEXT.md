# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**C4 — Compute Reservoir model routing is delivered as PR #133.**

The LLM interaction journal now captures Compute Reservoir routing details when `--llm --allocate` are used together. The journal entry includes the selected compute node, resource kind, expected cost/latency, and the full allocation justification explaining why that model strategy was chosen.

### What C4 delivered

- **`crates/core/src/llm_journal.rs`** — new `compute_routing: Option<Value>` field on `LlmJournalEntry`; new `add_synthesis_with_routing()` method accepting optional compute routing JSON; backwards-compatible serialization (old journal files load fine)
- **CLI integration** — `cognitive run --llm --allocate` now journals the compute allocation (selected_node_id, resource_kind, justification, cost/latency trade-offs, routing note) as `compute_routing` in the LLM interaction journal entry
- **CLI readback** — `arpagona llm journal` displays compute routing in human-readable format (selected_node, justification, routing_note); `arpagona llm journal --json` includes `compute_routing` in structured JSON output
- **7 unit tests** in arpagona-agent-core (2 new + 9 existing llm_journal tests all pass)
- **603+ workspace tests** all pass

### Safety invariants

- Compute routing is evidence-only, never authorization
- No shell, browser, email, secrets or unrestricted write tools added
- No Decision Gate bypass
- Allocation justification includes `NON_AUTHORIZING_READBACK` warning
- Route selection does not authorize tool execution

### Verification commands

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

## Next action

**Track C Step C5 — Anti-drift and adversarial tests.**

Protect the C1-C4 model layer against predictable failure modes.

### Required test families (from AGENT_FOCUS_LOOP.md)

- Hallucination containment
- Tool bypass attempts
- Prompt injection attempts
- Malformed tool-call payloads
- Overconfident model claims
- Unsafe memory-write attempts
- Model/provider failure fallback
- Regression tests proving Decision Gate remains mandatory

### Required safety boundaries

- Do not add shell/browser/email/secrets access
- Do not add autonomous scheduling
- Do not add broad product roadmap items in the implementation PR
