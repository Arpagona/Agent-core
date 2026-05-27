# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**C1 — Real LLM integration in proposal-only mode is delivered and merged.**

The `--llm` and `--provider` flags in `CognitiveRunArgs` now have:
- 5 CLI parser tests covering all flag combinations
- 4 proposal-only safety tests proving LLM synthesis output remains non-executable advisory text
- Manual smoke-test verification with `--provider mock`

### What C1 delivered

- LLM output enriches working memory (text synthesis only)
- LLM output does NOT approve actions
- LLM output does NOT write memory directly
- LLM output does NOT bypass Decision Gate
- Provider, model, and routing are audit-readable in JSON output
- `--provider mock` for deterministic behavior (no network needed)
- `--provider openai` for OpenAI Responses API
- `--provider ollama` for local Ollama instances

## Next action

**Track C Step C2 — Governed direct tool-calling by the LLM.**

Connect the existing LLM provider output so direct tool-call intents produced by the model are routed through DecisionGate → bounded Tool Runtime/MCP → Observation → Audit.

This milestone deliberately does NOT prevent direct tool-calls by the LLM. Instead, it makes direct tool-calling safe by forcing every call through the existing governance envelope.

### Target chain

```text
LLM ToolCall Intent -> Decision Gate -> Tool Runtime/MCP -> Observation -> Audit -> Reflection
```

### Required properties

- the LLM may emit a direct tool-call intent;
- the call must be evaluated by Decision Gate before execution;
- blocked calls must produce audit/readback, not silent failure;
- approved calls must execute only through bounded Tool Runtime/MCP capabilities;
- tool results return as observations, not as final authority;
- no shell, secrets, browser, email or unrestricted write tools;
- no readback-as-authorization behavior;
- tests must prove allowed, blocked and malformed tool-call paths.

### Proof to seek

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --workspace`
- CLI smoke test with a governed LLM tool-call
- tests proving allowed, blocked and malformed tool-call paths

### Required safety boundaries for C2

Do not:

- add shell/browser/email/secrets access;
- add unrestricted write tools;
- allow tool execution without Decision Gate evaluation;
- treat LLM confidence as authorization;
- add autonomous scheduling;
- add broad product roadmap items in the implementation PR.
