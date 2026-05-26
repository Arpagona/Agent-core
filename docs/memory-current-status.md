# Memory Systems — Current Status

This document records the current alpha status of ARPAGONA Agent Core memory systems after the latest validation run.

## Verified Memory Stack

### Holographic Memory

Status: alpha functional.

Current capabilities:

- symbolic resonance over keywords, concepts, entities and linked decisions;
- deterministic distributed signatures;
- project-scoped retrieval;
- source-linked reconstructed context;
- activation-count tracking;
- JSON save/load roundtrip for cross-session continuity.

Validation evidence:

- `cognitive run --resonate` returns a `holographic_resonance` block;
- observed resonance hints include cognitive-cycle patterns, compute-routing patterns and stop-with-report patterns;
- `--assess --allocate --resonate` runs end-to-end from assessment to local compute allocation to resonance readback.

Boundary:

- resonance is recall evidence only;
- it is not authorization;
- it does not replace Graph Memory or the Decision Gate.

### Conversation Memory

Status: alpha functional.

Current capabilities:

- conversation traces;
- deterministic scoring;
- project-scoped retrieval;
- keyword, concept and entity resonance;
- linked decision readback;
- JSON persistence roundtrip.

Validation evidence:

- `arpagona-conversation-memory` test suite passes: 27 tests, 0 failures.

### Graph Memory / FailureInsight

Status: alpha functional for the governed demo path.

Current capabilities:

- governed FailureInsight demo path;
- proposal to Decision Gate;
- approved decision;
- audit event creation;
- persistence/readback proof;
- cross-session snapshot readback.

Validation evidence:

- `memory demo failure-insight` produces an approved governed learning-loop insight;
- readback finds the persisted FailureInsight with relations and audit trace;
- snapshot JSON can be written and read across sessions.

### Cognitive Loop Resonance Bridge

Status: alpha functional.

Current capabilities:

- WorkingMemory to assessment;
- assessment to Compute Reservoir allocation;
- local compute allocation selection;
- resonance hints through the holographic bridge;
- JSON readback through CLI.

Validation evidence:

- `cognitive run --assess --allocate --resonate` works end-to-end.

## Current Limits

- Memory retrieval remains non-authorizing.
- No real tool execution is authorized by memory results.
- No scheduler autonomy is introduced by memory results.
- No Mission Control Web integration yet.
- API server still has pre-existing warnings in `apps/api-server/src/main.rs` around unused variables.
- Production persistence strategy is not finalized.
- Holographic resonance is symbolic and deterministic; it is not yet semantic embedding search.

## Canonical Guidance

```text
Conversation Memory provides dialogue continuity.
Holographic Memory provides pattern resonance.
Graph Memory provides explicit traceable memory.
FailureInsight turns governed failures into learning artifacts.
Decision Gate remains the authorization boundary.
```

```text
Memory can recall and explain.
Memory must not authorize or execute.
```
