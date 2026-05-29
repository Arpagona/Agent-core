# Steroid Hermes — Vertical Cognitive Loop Action Plan

> *Make ARPAGONA become a Hermes-like local-first cognitive runtime with stronger memory, continuity, compute routing, governance, reflection and operator readback. Prefer vertical cycle progress over isolated subsystem expansion.*

## 1. Strategic Objective

ARPAGONA Agent Core targets a Hermes-like ergonomic surface with deeper cognitive layers:

- **Hermes surface**: CLI entrypoints, local developer workflow, explicit commands, scheduled focus loops, readable reports, inspectable state, operational simplicity.
- **ARPAGONA depth**: Reservoir Echo, Holographic Memory, Compressed Convolutional Memory Retrieval, Graph Memory, Compute Reservoir, governed Tool Runtime, Failure-to-Insight reflection, Neutral Orchestrator, Decision Gate.

The Steroid Hermes plan is to accelerate the vertical cognitive loop while preserving ARPAGONA's governed architecture.

## 2. Target Runtime Chain (Two Modes)

**Proposal mode:**
```text
Objective -> Working Memory -> Observations -> Plan -> Assessment -> ProposedAction -> Decision Gate -> Audit -> Reflection -> Governed Learning
```

**Governed tool-call mode:**
```text
Objective -> Working Memory -> LLM ToolCall Intent -> Decision Gate -> Tool Runtime/MCP -> Observation -> Audit -> Reflection -> Governed Learning
```

## 3. Phase 3 Milestones (Active)

### P3 Milestones — Neutral Orchestrator Phase

| # | Milestone | Status |
|---|-----------|--------|
| P3-4f | Memory-aware context routing via Compute Reservoir | ✅ Merged |
| P3-10 | Compute-aware delegation from orchestrator to Compute Reservoir | ✅ Merged |
| P3-13 | Compute-aware context assembly for all 5 real adapters | ✅ Merged |
| P3-14 | Cycle Trace → FailureInsightCandidate bridge | ✅ Merged (PR #201) |
| P3-15 | CycleTrace to Failure-to-Insight candidates (`--insights` flag) | 🔜 PR #197 (CI ✅) |
| P3-16 | Compute efficiency feedback via `analyze_compute_efficiency()` | 🔜 PR #198 (CI ✅) |
| P3-17 | Efficiency feedback into context assembly | 🔜 PR #199 (CI ✅) |
| P3-18+ | Wire efficiency signal explanations into CycleTrace output; advance toward P4 | Planned |

### C Series — LLM and Tool-Call Integration

| # | Milestone | Status |
|---|-----------|--------|
| C1 | Real LLM integration in proposal-only mode | ✅ Complete (Phase 2) |
| C2 | Governed direct tool-calling by the LLM | ✅ Merged |
| C3 | Prompt, response, decision and risk journaling | Deferred |
| C4 | Compute Reservoir model routing | Deferred |
| C5 | Anti-drift and adversarial tests | Deferred |

### D Series — Operator Surfaces

| # | Milestone | Status |
|---|-----------|--------|
| D1 | Operator status surface | ✅ Merged |
| D2 | ProposedAction and tool-call supervision surface | ✅ Merged |
| D3 | Memory and resonance visibility | ✅ Merged |
| D4 | Minimal Web Mission Control skeleton | Deferred (after D1-D3) |
| D5 | Operator approval design study | Deferred |

### E Series — Product Demos

| # | Milestone | Status |
|---|-----------|--------|
| E1-E5 | SME demo, business workflow, demo pack, README demo, positioning | Deferred |

### H Series — Hardening

| # | Milestone | Status |
|---|-----------|--------|
| H1a | Clippy hygiene pass | ✅ Merged |
| H1b | Edge-case tests | ✅ Merged |
| H2 | CLI security boundary verification | ✅ Merged |

## 4. Default Work Order

When selecting a milestone for the next DEEP cycle, prefer in this order:

1. **P0 hygiene/truth alignment** — if validation or docs contradict.
2. **Cycle Trace V0** — operator inspectability of the runtime cycle.
3. **Orchestrated `cycle run` V0** — bounded coordinator for existing bricks.
4. **Governed perception inside the cycle** — read-only tools under governance.
5. **Failure-to-Insight from real observations** — turn cycle failures into durable learning.
6. **Compute Reservoir routing proof** — local/cloud routing with explanation.
7. **Memory/resonance recall integration** — advisory memory sources for context.
8. **Approval/mutation design** — before any mutation implementation.

## 5. Safety Boundaries (Non-Negotiable)

- No unrestricted shell, arbitrary command execution, file deletion, unrestricted write.
- No secrets access, browser automation, email sending, scheduler autonomy expansion.
- No hidden prompt/context injection, broad user-memory ingestion, readback-as-authorization.
- No Decision Gate bypass, self-modification without governed proposal, direct LLM approval of actions.
- MCP itself is permitted; unsafe MCP capabilities are forbidden.
- LLM direct tool-calling is permitted; ungoverned direct tool-calling is forbidden.

## 6. Verification Requirement

For any code change:
```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI changes: run the affected commands manually.
For MCP changes: include protocol-level tests.
For LLM changes: include proposal-only behavior tests.
For Tool Runtime changes: include safety-boundary tests.

## 7. Key Architecture Invariants

- `ProposedAction -> DecisionGate -> Decision -> Audit` is mandatory for all sensitive paths.
- All memory reads are advisory; readback ≠ authorization.
- All FailureInsight candidates are born `status: Proposed` — never auto-applied.
- Compute route hints are advisory signals, not authorization tokens.
- Context assembly items are advisory; none authorize actions.
- Tool execution results return as observations, not as final authority.
