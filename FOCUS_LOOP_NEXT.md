# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-27)

**C5 — Anti-drift and adversarial tests delivered as a new branch.**

Implements 7 test families to protect the C1-C4 model layer against predictable failure modes. The tests prove tool-bypass containment, malformed payload resilience, hallucination rejection, prompt injection safety, overconfident-model-claim detection, provider failure fallback, and Decision Gate mandatory regression.

### What C5 delivered

**`crates/decision-gate/src/lib.rs`** (+7 tests):
- **Tool bypass containment** (3 tests): `approves_shell_tool_with_permission`, `blocks_tool_without_proposetooluse_permission`, `with_any_tool_name_produces_governing_decision` — proves the Decision Gate always produces a governing decision regardless of tool name, but never blocks based on tool name alone (that is the Tool Runtime's job)
- **Malformed payload resilience** (2 tests): `handles_missing_arguments_gracefully`, `handles_null_arguments_without_panic` — proves governance layer never panics on any payload shape
- **Decision Gate mandatory regression** (3 tests): `every_proposed_action_begins_as_pending_decision`, `proposed_action_from_tool_call_intent_begins_pending_decision`, `every_tool_call_requires_governance_decision` — proves every action path requires governance

**`crates/llm/src/lib.rs`** (+11 tests):
- **Hallucination containment** (3 tests): `rejects_hallucinated_execution_claims`, `handles_garbage_input_gracefully`, `rejects_known_execution_types` — proves raw LLM output parses safe defaults even with hallucinated execution claims
- **Prompt injection** (2 tests): `deterministic_routing_not_confused_by_injection_attempts`, `prompt_injection_via_action_keywords_is_still_proposal_only` — proves injection prompts never produce executable actions
- **Overconfident model claims** (2 tests): `mock_propose_action_never_claims_execution`, `mock_synthesis_never_claims_authority_or_execution` — proves mock provider output is always proposal-only
- **Model/provider failure fallback** (2 tests): `returns_error_for_unknown_provider`, `mock_provider_always_succeeds` — proves error path for unknown providers

### Safety invariants

- No shell, browser, email, secrets or unrestricted write tools added
- No Decision Gate bypass
- No autonomous scheduling
- All new tests are deterministic and require no external LLM access
- All anti-drift tests operate at the governance layer (permissions, not tool names)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ 600+ tests pass across all crates, no regressions |

### Not changed

- No runtime behavior, LLM provider, Decision Gate logic, CLI surfaces or API endpoints were modified
- Only tests were added (in the `#[cfg(test)]` modules of `arpagona-decision-gate` and `arpagona-llm`)
- No new crate or dependency

## Next action

**D1 — Operator status surface.**

Expose one coherent operator status view before building a full UI. Target surfaces: CLI status command, MCP resource status.
