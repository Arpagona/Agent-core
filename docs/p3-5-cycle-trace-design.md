# P3-5: Cycle Trace V0 — Structured Causal Trace with Context Assembly Metadata

> **Status**: Delivered as part of Phase 3 milestone P3-5.
> **Scope**: Structured, serializable cycle trace with per-source context assembly metadata, enriched human-readable `causal_trace()`, and CLI `--trace` flag.
> **Safety**: All trace data is non-authorizing. No field may be interpreted as approval, authorization, or execution permission.

## 1. Problem Statement

The Neutral Orchestrator produced a `causal_trace()` string that showed the core causal chain (cycle → objective → compute → action → decision → audit → outcome) but omitted per-source context assembly metadata:

- Which memory sources contributed context items?
- How many items per source?
- What sample items were retrieved?
- Which sources were unavailable?

Without this metadata, operators could see that a cycle "has context" but could not inspect *which cognitive sources* were active, *how much* each contributed, or whether any sources were missing.

## 2. What Was Built

### 2.1 Domain Types (in `crates/core/src/orchestrator.rs`)

**`ContextSourceSummary`** — per-source context assembly metadata:
- `source`: source name (`"graph_memory"`, `"holographic_memory"`, `"reservoir_echo"`)
- `item_count`: number of items contributed
- `available`: whether the source was reachable
- `sample_key` / `sample_value_preview` (optional, truncated to 120 chars)

**`CycleTrace`** — structured, serializable causal trace:
- `cycle_id`, `objective_text`, `objective_domain`
- `context_source_summaries: Vec<ContextSourceSummary>`
- `total_context_items`, `unavailable_sources`
- `compute_route_label`, `compute_route_justification`
- `action_type`, `decision_status`
- `audit_event_count`, `gate_was_applied`, `cycle_status`, `summary`
- `non_authorizing: true` (invariant)
- `format()` method for human-readable output
- `from_context_bundle()` helper to extract per-source summaries from a `ContextBundle`

### 2.2 OrchestratorCycle Extension (in `crates/neutral-orchestrator/src/lib.rs`)

**`OrchestratorCycle::to_cycle_trace()`** — converts a cycle into a structured `CycleTrace`:
- Extracts context assembly metadata from the `ContextBundle`
- Maps objective, compute route, action, decision, audit fields
- Preserves `non_authorizing` invariant

**`OrchestratorCycle::causal_trace()`** — now delegates to `to_cycle_trace().format()`:
- Richer output includes per-source item count breakdown with sample items
- Shows unavailable sources
- Human-readable tree format with `├─` branch indicators

### 2.3 CLI Enhancement (in `crates/cli/src/main.rs`)

**New `--trace` flag** on `arpagona orchestrator run`:
- No flag: backward-compatible causal trace output
- `--trace`: enriched trace with context assembly metadata
- `--json --trace`: full `CycleTrace` JSON with all context assembly metadata
- `--json` (no trace): backward-compatible `OrchestratorOutcome` JSON

### 2.4 Tests

**Core crate** (5 new tests in `crates/core/src/orchestrator.rs`):
- `cycle_trace_new_is_non_authorizing`
- `cycle_trace_serializes_and_deserializes`
- `cycle_trace_format_shows_context_sources`
- `context_source_summary_with_sample_truncates_long_values`
- `from_context_bundle_maps_all_sources`

**Neutral orchestrator crate** (4 new/updated tests in `crates/neutral-orchestrator/src/lib.rs`):
- `test_deterministic_cycle_causal_trace_format` (updated — checks per-source metadata)
- `test_orchestrator_cycle_to_cycle_trace_is_non_authorizing`
- `test_cycle_trace_serialization_round_trip`
- `test_cycle_trace_with_context_hint_shows_sample`

## 3. Safety Invariants

| Invariant | Enforced by |
|---|---|
| `non_authorizing: true` at construction | `CycleTrace::new()` sets it, no setter exposed |
| No approval/authorization/execution fields | Structural: no fields named `approved`, `authorized`, `executed` |
| All trace data is advisory | `format()` includes "Advisory — context assembly metadata is non-authorizing" warning |
| No side effects from trace creation | `to_cycle_trace()` is pure, in-process, no I/O |
| Backward compatibility | `--json` without `--trace` returns `OrchestratorOutcome` as before |

## 4. Files Changed

| File | Change |
|---|---|
| `crates/core/src/orchestrator.rs` | Added `ContextSourceSummary`, `CycleTrace` types + 5 tests |
| `crates/neutral-orchestrator/src/lib.rs` | Added `to_cycle_trace()`, updated `causal_trace()`, 4 cycle trace tests |
| `crates/cli/src/main.rs` | Added `--trace` flag to orchestrator run, enriched CLI output |
| `docs/p3-5-cycle-trace-design.md` | **NEW** — this design document |
| `FOCUS_LOOP_NEXT.md` | Updated handoff |
| `PROJECT_STATUS.md` | Updated with P3-5 delivery record |

## 5. What Was NOT Added

- No real memory adapters — `SimulatedContextAssembler` remains the default
- No persistence — `CycleTrace` is serializable but not automatically saved
- No new CLI subcommands — only a `--trace` flag
- No Decision Gate bypass, scheduler, autonomy, shell, browser, email, or secrets access
- No LLM calls, model endpoints, or provider integration
- No changes to existing orchestrator behavior or outcome types

## 6. Next Step

P3-5 is delivered. The next Phase 3 candidate is **P3-4a through P3-4e** — real memory adapters:
- P3-4a: GraphMemoryAdapter
- P3-4b: HolographicMemoryAdapter
- P3-4c: ReservoirEchoAdapter
- P3-4d: CompressedCognitiveAttentionAdapter
- P3-4e: ToolRuntimeAdapter

Or, if GONA prefers, proceed to **Neutral Orchestrator integration into the CLI/API runtime loop**.
