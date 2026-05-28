# P3-4: Memory-Aware Context Integration Design

> **Status**: Design document for Phase 3 milestone P3-4.
> **Scope**: Defines how Graph Memory, Holographic Memory, Reservoir Echo, Compressed Cognitive Attention, and Tool Runtime feed advisory context into orchestrator cycles.
> **Safety**: All memory-derived context is advisory. No context item may approve, authorize or execute an action.

## 1. Problem Statement

The Neutral Orchestrator V0 (P3-2) assembles an advisory `ContextBundle` before proposing actions. However, the current `assemble_context` implementation is purely synthetic:

- It only carries the `context_hint` string from `ObjectiveInput`
- It explicitly marks `HolographicMemory` and `ReservoirEcho` as unavailable
- It never queries Graph Memory, Holographic Memory, Reservoir Echo, Compressed Cognitive Attention, or the Tool Runtime

This means the orchestrator's context bundle is empty of all real contextual memory. An objective like "Review yesterday's failure insights and propose a correction" would produce a cycle with zero recall, defeating the purpose of memory-aware orchestration.

## 2. Target Pipeline

```
ObjectiveInput
  -> MemoryQueryRequest (which sources, what query, for which objective)
  -> Per-source adapters:
       ┌─ GraphMemoryAdapter
       ├─ HolographicMemoryAdapter
       ├─ ReservoirEchoAdapter
       ├─ CompressedCognitiveAttentionAdapter
       └─ ToolRuntimeAdapter
  -> MemoryQueryResult (raw, per-source, structured replies)
  -> ContextAssembler (filters, merges, deduplicates)
  -> ContextBundle (advisory, non-authorizing, source-tagged)
  -> ProposalRequest
```

Every step is advisory. No context in this pipeline may approve, authorize, or execute an action.

## 3. Source Adapters

### 3.1 GraphMemoryAdapter

**Purpose**: Retrieve structured durable facts, entities, relations, past decisions, and FailureInsight records relevant to the objective.

**Query**: By workspace + objective text keywords. Returns facts whose labels or entity names intersect with the objective's key terms.

**Output shape**: `Vec<ContextItem>` where each item carries `key`, `value`, `source: "graph_memory"`, and an optional `confidence` score.

**Non-goals**:
- Does not grant write access to any Graph Memory store
- Does not create new facts or relations

**When unavailable**: Mark `ContextSource::GraphMemory` as unavailable. Continue with empty graph_memory_items.

### 3.2 HolographicMemoryAdapter

**Purpose**: Surface non-authorizing pattern resonance — "what does this objective resemble?" from past episodes, tasks, action chains, and failures.

**Query**: Encode the objective text as a symbolic signature and compare against stored traces. Return top-k resonance matches with scores and trace IDs.

**Output shape**: `Vec<ContextItem>` with `key` = trace_id, `value` = resonance summary, `source: "holographic_memory"`, `score` = resonance value.

**Always advisory**: Holographic memory recall is pattern resonance, not established fact. Every item carries its non-authorizing provenance.

**When unavailable**: Mark `ContextSource::HolographicMemory` as unavailable. Continue with empty holographic_resonance_items.

### 3.3 ReservoirEchoAdapter

**Purpose**: Surface short-term volatile traces from recent cycles — active objectives, recent observations, recent failure-insight candidates that have not yet decayed.

**Query**: By workspace. Return all active traces that have not fully decayed (decay count < decay limit).

**Output shape**: `Vec<ContextItem>` with `key` = trace key, `value` = trace payload, `source: "reservoir_echo"`, plus decay metadata.

**Non-goals**:
- Not durable memory — traces may decay and disappear
- Not a source of authorization

**When unavailable**: Mark `ContextSource::ReservoirEcho` as unavailable. Continue with empty reservoir_traces.

### 3.4 CompressedCognitiveAttentionAdapter

**Purpose**: Surface temporally enriched memory candidates — memories that scored highly when the current query is convolved with local temporal neighborhoods in the compressed latent space.

**Query**: Project the objective text through the deterministic compressed projection, compute local temporal convolution over the memory event stream, return top-k enriched memory candidates.

**Output shape**: `Vec<ContextItem>` with `key` = memory_event_id, `value` = enriched text, `source: "compressed_cognitive_attention"`, `score` = attention similarity, `temporal_window` = the neighborhood window used.

**Always experimental**: This adapter may return results that are structurally different from flat similarity search. Its output is always advisory and non-authorizing.

**When unavailable**: Mark `ContextSource::CompressedCognitiveAttention` as unavailable. Continue with empty items. The compressed-cognitive-attention crate exists now as a standalone library — this adapter is the bridge that makes its results available to the orchestrator.

### 3.5 ToolRuntimeAdapter

**Purpose**: Surface workspace file-system perception — read key files, list directories, search for patterns that relate to the objective.

**Query**: By workspace path + objective keywords. The adapter uses bounded, read-only Tool Runtime calls (read_file, list_files, search_text) to gather workspace context.

**Output shape**: `Vec<ContextItem>` where each item captures an observation: file path, matched text, file listing result, search snippet.

**Safety guarantees**:
- Inherits all Tool Runtime boundaries (no absolute paths, no parent traversal, no sensitive files, size limits)
- Every observation is evidence-only, not authorization
- Tool Runtime calls are read-only by design

**When unavailable**: Mark `ContextSource::ToolRuntime` as unavailable. Continue with empty items.

## 4. MemoryQueryRequest Type

The orchestrator sends a `MemoryQueryRequest` to the context assembly pipeline:

```rust
pub struct MemoryQueryRequest {
    /// The orchestrator cycle ID.
    pub cycle_id: OrchestratorCycleId,
    /// The objective being processed.
    pub objective_id: ObjectiveId,
    /// The objective text (used as query for all adapters).
    pub objective_text: String,
    /// The workspace to scope queries within.
    pub workspace_id: WorkspaceId,
    /// Which sources to query (default: all available).
    pub requested_sources: Vec<ContextSource>,
    /// Maximum items per source (prevents overstuffing).
    pub max_items_per_source: usize,
    /// Timestamp of the request.
    pub created_at: DateTime<Utc>,
}
```

## 5. MemoryQueryResponse Type

Each adapter returns a `MemoryQueryResponse`:

```rust
pub struct MemoryQueryResponse {
    /// The source this response came from.
    pub source: ContextSource,
    /// Advisory context items retrieved.
    pub items: Vec<ContextItem>,
    /// Whether the source was available.
    pub available: bool,
    /// Human-readable explanation of the query result.
    pub explanation: String,
}
```

## 6. ContextAssembler Interface

The `ContextAssembler` is a pluggable component that the orchestrator delegates context assembly to.

```rust
/// The ContextAssembler gathers advisory context from memory sources.
///
/// Implementations may query real adapters (GraphMemoryAdapter, etc.) or
/// return synthetic/no-op results for testing and simulation.
///
/// Every item returned is advisory and non-authorizing.
pub trait ContextAssembler {
    /// Assemble advisory context for the given objective.
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse>;

    /// Return the list of sources this assembler can query.
    fn supported_sources(&self) -> Vec<ContextSource>;
}
```

### 6.1 SimulatedContextAssembler (Default)

The no-op/simulated implementation is the default. It:

- Returns empty results for all sources
- Marks all sources as available (but empty)
- Requires no I/O, no LLM, no persistence, no dependencies
- Is deterministic and in-process

This ensures the orchestrator compiles and works without any memory adapter installed. Real adapters can be plugged in progressively.

### 6.2 Real adapters (Future)

Real implementations will be added in later milestones:

- P3-4a: GraphMemoryAdapter — queries `crates/graph-memory`
- P3-4b: HolographicMemoryAdapter — queries `crates/holographic-memory`
- P3-4c: ReservoirEchoAdapter — queries reservoir state from `crates/core`
- P3-4d: CompressedCognitiveAttentionAdapter — queries `crates/compressed-cognitive-attention`
- P3-4e: ToolRuntimeAdapter — queries `crates/tool-runtime`

Each adapter is gated behind a feature flag or optional dependency. The `SimulatedContextAssembler` is always available.

## 7. Integration into OrchestratorEngine

The `OrchestratorEngine` gains a `context_assembler: Box<dyn ContextAssembler>` field. The `assemble_context` method is updated:

1. Create a `MemoryQueryRequest` from the objective input
2. Call `self.context_assembler.assemble(&request)`
3. Collect responses into a `ContextBundle`, mapping each `MemoryQueryResponse.source` to the appropriate field in the bundle
4. Set `advisory_warning` — this must never be removed

```rust
fn assemble_context(&self, input: &ObjectiveInput) -> ContextBundle {
    let request = MemoryQueryRequest { ... };
    let responses = self.context_assembler.assemble(&request);

    let mut bundle = ContextBundle::new(...);

    for response in responses {
        match response.source {
            ContextSource::GraphMemory => {
                for item in response.items {
                    bundle.graph_memory_items.push(item);
                }
            }
            ContextSource::HolographicMemory => {
                for item in response.items {
                    bundle.holographic_resonance_items.push(item);
                }
            }
            ContextSource::ReservoirEcho => {
                for item in response.items {
                    bundle.reservoir_traces.push(item);
                }
            }
            // CompressedCognitiveAttention and ToolRuntime 
            // items are added to the most semantically matching field
            // or stored in a generic items bucket
            _ => {
                // Future: store in a unified advisory_items field
            }
        }

        if !response.available {
            bundle.unavailable_sources.push(response.source);
        }
    }

    bundle
}
```

### 7.1 Backward Compatibility

The existing tests must pass without modification because:
- `OrchestratorEngine::new()` creates a `SimulatedContextAssembler` by default
- `SimulatedContextAssembler` returns empty responses for all sources (no context_hint)
- The existing `with_context()` integration uses Objectives that have a `context_hint`, which is handled separately before the assembler is called
- The simulator does not require any dependencies, I/O, or additional crates

To preserve the existing `context_hint` → `graph_memory_items` behavior:
- The `context_hint` field from `ObjectiveInput` is special-cased: it is always added as a `graph_memory_item` after the assembler runs, regardless of whether GraphMemoryAdapter was available
- This means existing tests like `test_deterministic_cycle_with_context_hint` continue to pass

## 8. MemoryQueryRequest → ContextSource Extensions

The existing `ContextSource` enum needs two new variants:

```rust
pub enum ContextSource {
    GraphMemory,
    HolographicMemory,
    ReservoirEcho,
    ToolRuntime,
    WorkingMemory,
    /// New: Compressed Cognitive Attention memory retrieval.
    CompressedCognitiveAttention,
}
```

## 9. Safety Invariants

| Invariant | Enforced by |
|---|---|
| All context is advisory | `CONTEXT_BUNDLE_ADVISORY_WARNING` in every bundle |
| No context authorizes actions | Private builder methods, no public `non_authorizing = false` path |
| No I/O by default | `SimulatedContextAssembler` is the default |
| No hidden memory access | Each adapter is explicitly registered, never auto-resolved |
| Size limits per source | `max_items_per_source` in `MemoryQueryRequest` |
| No persistence side-effects | Adapters are read-only by design |
| Empty sources are documented | `unavailable_sources` field tracks reachable/configured sources |

## 10. What This Milestone Delivers

### P3-4 delivery boundary:

1. **Design document** (this file) — ✅
2. **Domain types** in `crates/core/src/orchestrator.rs`:
   - `MemoryQueryRequest` struct
   - `MemoryQueryResponse` struct
   - New `ContextSource::CompressedCognitiveAttention` variant
3. **ContextAssembler trait** in `crates/neutral-orchestrator/src/`:
   - `ContextAssembler` trait (assemble, supported_sources)
   - `SimulatedContextAssembler` struct (default, no-op)
4. **OrchestratorEngine integration**:
   - `context_assembler: Box<dyn ContextAssembler>` field
   - Constructor accepts optional assembler (defaults to SimulatedContextAssembler)
   - `assemble_context()` uses the assembler
5. **Tests**:
   - SimulatedContextAssembler returns empty responses for all sources
   - SimulatedContextAssembler reports all sources as available
   - Orchestrator cycle with simulated assembler produces correct ContextBundle
   - Context hint is still preserved as graph_memory_item

### Not delivered by this milestone:

- No real GraphMemoryAdapter implementation
- No real HolographicMemoryAdapter implementation
- No real ReservoirEchoAdapter implementation
- No real CompressedCognitiveAttentionAdapter implementation
- No real ToolRuntimeAdapter implementation
- No I/O, no persistence, no LLM calls
- No authorization, execution, or scheduler behavior added

## 11. Next Steps (Post-P3-4)

| Step | Description |
|---|---|
| P3-4a | Implement GraphMemoryAdapter using `crates/graph-memory` |
| P3-4b | Implement HolographicMemoryAdapter using `crates/holographic-memory` |
| P3-4c | Implement ReservoirEchoAdapter using reservoir primitives |
| P3-4d | Implement CompressedCognitiveAttentionAdapter using `crates/compressed-cognitive-attention` |
| P3-4e | Implement ToolRuntimeAdapter using `crates/tool-runtime` |
| P3-5 | Cycle Trace V0 — orchestrator records causal traces with real context assembly metadata |
