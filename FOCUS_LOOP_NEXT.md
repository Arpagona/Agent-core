# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**Phase 1 alpha foundation is delivered and merged into `main`.**

### Completed Phase 1 queue
| Milestone | Status |
|-----------|--------|
| P1 — Open PR cleanup | ✅ No open PRs |
| P2 — Holographic Memory persistence | ✅ Merged |
| P3 — Governed MCP observability | ✅ Merged |
| P4 — General Cognitive Work Loop V0 | ✅ Merged |
| P5 — Cognitive Observation to Governed Learning | ✅ Merged |
| P6 — Working Memory integration | ✅ Merged |
| P7 — Compute Reservoir integration | ✅ Merged |
| P8 — End-to-end governed alpha demo | ✅ Verified working |

### Completed Track A — MCP Server
| Step | Status |
|------|--------|
| A1 — stdio transport + tools/list + tools/call | ✅ |
| A2 — DecisionGate governance | ✅ |
| A3 — HTTP/SSE transport | ✅ |
| A4 — Resources + Prompts | ✅ |
| A5 — notifications/list_changed + protocol hardening | ✅ |

### Completed Track B — Holographic Memory
| Step | Status |
|------|--------|
| B1 — Conversation-memory bridge | ✅ |
| B2 — Recursive graph traversal | ✅ |
| B3 — Local embeddings / semantic generalization | ✅ |
| B4 — SQLite persistence | ✅ |
| B5 — Consolidation and duplicate trace fusion | ✅ |
| B6 — Governed writes via DecisionGate | ✅ |
| B7 — Cognitive-loop recall hints from resonance matches | ✅ |

### Daily validation backlog
All items resolved or closed.

## Next action

**Track C Step C1 — Real LLM integration in proposal-only mode.**

Connect `arpagona cognitive run --llm` to the existing LLM/provider abstraction so the model can enrich WorkingMemory, observations, plans and ProposedActions, but cannot approve actions, write memory directly or bypass Decision Gate.

Direct LLM tool-calls are intentionally **not required in C1**. They are planned for **C2 — Governed direct tool-calling by the LLM**, where direct tool-call intents are allowed only through Decision Gate, bounded Tool Runtime/MCP execution, observation readback and audit.

## Proof to seek

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --workspace`
- CLI smoke test with `--llm`
- tests proving C1 LLM output remains proposal-only
- audit/readback includes provider, model, prompt summary and decision path where practical

## Required safety boundaries for C1

Do not:

- add direct LLM approval;
- add direct LLM memory writes;
- bypass Decision Gate;
- add shell/browser/email/secrets access;
- add autonomous scheduling;
- treat LLM confidence as authorization;
- add broad product roadmap items in the implementation PR.

## Required C2 interpretation

C2 must **not** prohibit direct tool-calls by the LLM. C2 must make them safe:

```text
LLM ToolCall Intent → DecisionGate → bounded Tool Runtime/MCP → Observation → Audit
```

## Expected outcome

A new PR exists for C1 with one bounded implementation increment proving that real model integration can participate in the cognitive loop while preserving the mandatory governed action path:

```text
Objective → WorkingMemory → LLM-assisted reasoning → ProposedAction → DecisionGate → Audit
```
