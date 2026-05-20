# ARPAGONA Agent Core — Agent Context

This is the compact context file that agents should read at the beginning of every non-trivial loop.

It preserves the short-form project direction without requiring every loop to reread all long-form documents.

## 1. Current direction

Build ARPAGONA Agent Core aggressively but safely toward a functional Hermes-like alpha.

The near-term product surface is the CLI: the first local Mission Control surface.

The core must remain Rust-first, local-first, graph-native, compute-aware, auditable and governed.

## 2. Three foundational inspirations

### Reservoir computing — consciousness by propagation

ARPAGONA should explore a practical technical analogue of cognitive continuity.

The goal is not to copy the brain literally, but to preserve a useful form of propagation across cycles: short-term echoes, activation, decay, context carryover, and continuity between tasks.

In the current architecture:

- `Reservoir Echo` is short-term volatile continuity;
- `Graph Memory` is durable structured memory;
- `Compute Reservoir` is resource routing and capability allocation.

Do not confuse these three layers.

### Hermes-like agent ergonomics

Hermes is an inspiration for practical agent-product ergonomics.

Useful patterns to keep in mind:

- clear CLI entrypoints;
- local developer workflow;
- explicit commands;
- scheduled focus loops;
- readable reports;
- inspectable state;
- operational simplicity.

Do not blindly copy Hermes. ARPAGONA must preserve its own governed architecture.

### Rippletide-inspired architecture

The key Rippletide-inspired idea is runtime enforcement.

Agents should not act directly. They produce structured intent. The runtime checks context, policies, permissions and risk before anything can affect the outside world.

The non-negotiable path is:

```text
ProposedAction -> DecisionGate -> Decision -> Audit -> Graph Memory readback
```

Readback is not approval. Graph Memory is not authorization. Tool Registry lookup is not approval. CLI output is not execution state.

## 3. Current development bias

Prefer useful read-only supervision increments over repeated test-only stabilization.

Default near-term priority:

1. improve CLI audit readback;
2. add task-level and workspace-level CLI summaries;
3. make local supervision useful before expanding Mission Control Web;
4. use tests to protect real uncovered risks, not as the default next step;
5. defer execution/autonomy until the governed path is mature.

## 4. Always allowed, when bounded

- Rust-first domain/API/CLI/readback improvements;
- read-only CLI supervision;
- audit and graph readback improvements;
- documentation alignment;
- tests that protect newly exposed behavior;
- LOCO/Ollama first-pass analysis for low-risk repository reading.

## 5. Still blocked

- direct tool execution;
- shell access;
- browser automation;
- MCP expansion;
- scheduler autonomy;
- credential/secret handling changes;
- destructive data operations;
- Decision Gate bypass;
- treating readback as authorization;
- Mission Control Web expansion before CLI patterns are useful.

## 6. Loop reporting requirement

Every non-trivial loop should report:

- guidance files read;
- delegated to LOCO/Ollama: yes/no;
- local task summary;
- Hermes final decision;
- why a CLI supervision increment was or was not chosen;
- files changed;
- tests run;
- risks;
- deliberately not changed.

## 7. Files to read

Always read this file first.

Then read:

- `PROJECT_STATUS.md` for current implementation status;
- `docs/operating-doctrine.md` for work rules;
- `docs/development-acceleration.md` for acceleration direction.

Read longer documents only when needed:

- `PROJECT_OBJECTIVES.md` for architectural or philosophical decisions;
- `docs/architecture.md` for component boundaries;
- `docs/roadmap.md` for sequencing;
- `docs/causal-trace.md` for audit/readback changes.
