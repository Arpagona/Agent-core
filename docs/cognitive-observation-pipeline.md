# Cognitive Observation Pipeline

## Purpose

The Cognitive Observation Pipeline bridges the gap between raw tool execution results and structured cognitive processing. Every tool produces a `ToolExecutionResult`; the pipeline transforms it into a `CognitiveObservation`, then evaluates it through an `ObservationAssessment`, and optionally flags it as a `FailureInsightCandidate`.

```
ToolExecutionResult
    ↓  CognitiveObservation::from_tool_execution()
CognitiveObservation
    ↓  assess_observation()
ObservationAssessment
    ↓  (candidate detection)
Option<FailureInsightCandidate>
```

## Why This Matters

Before this pipeline, tool results were structured but **not cognitive**: the system knew *what* a tool returned but not *how useful it was*, *whether it was complete*, or *whether it should trigger learning*. Now, the cognitive loop can answer:

- "I searched for `fn main` and found 4 results — this is useful and complete."
- "I tried to read `/etc/passwd` and was blocked — this is a positive safety signal."
- "I searched for `zzz_nonexistent` and found 0 results — this might be a candidate for FailureInsight."
- "I read `Cargo.toml` (29 lines) but the preview was truncated at 500 chars — partial observation."

## Architecture

### Types (pure domain, no I/O, no authorization)

All types live in `crates/core/src/observation.rs`.

| Type | Role |
|------|------|
| `CognitiveObservation` | Core observation record: status, usefulness, risk, candidate flag |
| `ObservationSource` | Where the observation came from (ToolExecution, HumanInput, etc.) |
| `ObservationKind` | What was observed (FileContent, SearchResult, SecurityBoundary, etc.) |
| `ObservationStatus` | Success, Empty, Partial, Truncated, Blocked, Failed, Warning |
| `ObservationUsefulness` | None, Low, Medium, High, DirectlyActionable |
| `ObservationRisk` | None, Low, Medium, High, Security |
| `ObservationAssessment` | Evaluated observation with candidate metadata |
| `FailureInsightCandidate` | Lightweight marker for FailureInsight creation |
| `FailureInsightCandidateKind` | 10 variant kinds (TruncatedResult, EmptySearchResult, etc.) |

### Key Functions

- **`CognitiveObservation::from_tool_execution(result)`** — explicit conversion (not a trait, not auto-executing)
- **`assess_observation(observation)`** — pure classification function, no side effects

### Safety Invariants

1. **No auto-execution**: `from_tool_execution` is an explicit method call, not a `From`/`Into` trait
2. **No authorization**: types contain no approval/authorize/execute fields
3. **No persistence**: assessment creates `FailureInsightCandidate` markers, not full `FailureInsight` records
4. **Positive security signals**: a blocked access is classified as `SafetyBoundaryTriggered` (positive), not as a failure
5. **Domain purity**: no I/O, no system access, no shell execution

## CLI Usage

```bash
# Run the full pipeline on a tool execution
arpagona tool demo observe read_file '{"path":"Cargo.toml"}'
arpagona tool demo observe read_file '{"path":".env"}'
arpagona tool demo observe search_text '{"query":"fn main","path":"."}'

# JSON output
arpagona tool demo observe read_file '{"path":"Cargo.toml"}' --json
```

The CLI shows three steps:
1. **Tool Execution** — raw result from the runtime
2. **Cognitive Observation** — interpreted observation with status, usefulness, risk
3. **Assessment** — evaluation with FailureInsight candidate detection

## Example Outputs

### Successful file read (with capped preview)
```
Observation: FileContent / Truncated / Medium / None
        29 lines, preview truncated at 500 chars
        ↳ FailureInsight candidate: TruncatedResult (partial observation)
```

### Security boundary block
```
Observation: SecurityBoundary / Blocked / Low / Security
        File access blocked: .env
        ↳ FailureInsight candidate: SafetyBoundaryTriggered 🟢 positive signal
```

### Empty search result
```
Observation: SearchResult / Empty / Low / None
        0 results for 'nonexistent_function'
        ↳ FailureInsight candidate: EmptySearchResult
```

## Next Steps

The pipeline stops at candidate detection. The next safe handoff is **governed learning from observations**: a cognitive loop that evaluates candidates and decides which ones to promote to full FailureInsight records through the Decision Gate.

## Files Changed

| File | Change |
|------|--------|
| `crates/core/src/observation.rs` | **New** — all types + conversions + assessment + tests (21 tests) |
| `crates/core/src/lib.rs` | Added `pub mod observation` + `pub use observation::*` |
| `crates/cli/src/main.rs` | Added `tool demo observe` command |
| `docs/cognitive-observation-pipeline.md` | **This file** |
