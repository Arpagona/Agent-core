# ARPAGONA Agent Core — Project Status

This document is the canonical operational status file for ARPAGONA Agent Core.

It describes the current implementation state, stability level, architectural risks, explicit stop-list, and the recommended next sequence of work.

Every future contributor or agent must read this file together with `PROJECT_OBJECTIVES.md`, `docs/operating-doctrine.md`, `docs/development-acceleration.md` and `docs/failure-to-insight.md` before modifying the repository.

## 1. Current State

The repository currently contains a fast-moving alpha foundation with several experimental building blocks already present.

Current observed state:
- `PROJECT_OBJECTIVES.md` exists and defines the canonical vision of the project.
- `PROJECT_STATUS.md` exists and defines the canonical operational status of the repository.
- `docs/operating-doctrine.md` defines the current working doctrine: controlled fast iteration, Rust-first development, LOCO/Ollama delegation and CLI supervision first.
- `docs/development-acceleration.md` defines the current acceleration direction: Hermes-like alpha ergonomics, Rippletide-inspired runtime enforcement and CLI-as-local-Mission-Control.
- `docs/failure-to-insight.md` defines the canonical doctrine for turning failures, blocked decisions, bad proposals, missing context, policy gaps and human corrections into durable, non-authorizing insights.
- `docs/graph-memory-local-persistence.md` records the current local SurrealDB persistence backend findings: `kv-surrealkv` requires the unstable SurrealDB cfg flag, while `kv-rocksdb`/`File` introduces native RocksDB/zstd build assumptions that failed local scheduled-run verification.
- `README.md` points contributors and agents to the canonical project files before any modification.
- `docs/roadmap.md` distinguishes the target architectural order from experimental work already prototyped out of order.
- `docs/architecture.md` includes explicit architectural re-centering guidance.
- `docs/compute-reservoir.md` frames the alpha minimal Compute Reservoir crate and its non-goals.
- `docs/tool-registry.md` frames the alpha minimal Tool Registry crate, its declarative role and its explicit non-goals.
- `docs/causal-trace.md` documents alpha conventions for linking proposed actions, tasks, decisions and audit events.
- `crates/core` exists and contains the core domain vocabulary: agents, workspaces, tasks, goals, proposed actions, decisions, policies, permissions, risks, graph primitives, audit events, memory concepts, cognitive primitives and the minimal Failure-to-Insight vocabulary.
- `Decision Gate` exists as alpha governance logic inside `crates/decision-gate`.
- `crates/compute-reservoir` exists as an alpha minimal pure Rust crate with compute inventory/allocation types and a deterministic `allocate_compute` function.
- `crates/tool-registry` exists as an alpha minimal declarative catalogue for tool definitions, capabilities, schemas, permissions, risk levels and enabled/disabled states, without execution.
- `Reservoir Echo` currently exists inside the Cognitive Runtime primitives as short-term volatile cognitive continuity.
- `crates/graph-memory` exists as an experimental SurrealDB adapter for Graph Memory persistence, alpha audit trace lookup by workspace, task, proposed action and decision, governed approved memory fact and FailureInsight persistence/readback helpers with non-authorizing trace proof readback, an in-memory demo/test store helper, and schema-backed CLI status readback.
- `crates/llm` exists as an experimental provider abstraction that can produce `ProposedAction` objects with `PendingDecision`, without executing tools.
- `crates/runtime` exists as an experimental cognitive runtime loop that stops at action proposal.
- `apps/api-server` exists as an alpha Axum API server.
- `crates/cli` exists as an alpha terminal interface and provides read-only local supervision surfaces for decision-scoped audit readback, Failure-to-Insight vocabulary, Graph Memory alpha status, governed memory-write proposal readback and a local `memory demo failure-insight` loop for proposal → Decision Gate → audit → in-memory persistence → readback proof.
- Governed memory-write readbacks now expose the optional proposed target value in both memory proposal summaries and decision/audit readbacks, while preserving compatibility with older payloads that omit the value.
- `apps/mission-control` exists only as a placeholder and must remain deferred until the CLI supervision path proves useful.
- `workers/python-ingestion` exists only as a placeholder and must remain deferred.

The implementation already demonstrates the founding rule:

```text
Agent -> ProposedAction -> DecisionGate -> Decision -> Audit
```

The current product direction is no longer abstract stabilization only. The near-term priority is to move toward a functional Hermes-like alpha through read-only, Rust-first, local supervision surfaces, especially the CLI, while preserving Rippletide-inspired runtime enforcement and the non-negotiable governed action path.

## 2. Stability Matrix

| Component | Status | Role | Notes |
|---|---|---|---|
| `PROJECT_OBJECTIVES.md` | Stable foundation | Canonical project vision | Must be read before every significant change. |
| `PROJECT_STATUS.md` | Stable foundation | Canonical operational status | Must be updated after every significant change. |
| `docs/operating-doctrine.md` | Stable foundation | Current work doctrine | Defines controlled fast iteration, Rust-first work, LOCO/Ollama delegation and CLI supervision first. |
| `docs/development-acceleration.md` | Stable foundation | Current acceleration direction | Defines Hermes-like alpha ergonomics, CLI supervision first and Rippletide-inspired runtime enforcement. |
| `docs/failure-to-insight.md` | Stable foundation | Failure-to-Insight doctrine | Defines how failures and corrections become durable learning without becoming authorization, execution or self-modification. |
| `README.md` | Stable foundation | Entry point for contributors | Points to canonical objective/status/doctrine/acceleration files. |
| `docs/roadmap.md` | Stable foundation | Architectural implementation order | Must reflect controlled acceleration without allowing unsafe execution. |
| `docs/architecture.md` | Stable foundation | Target architecture and boundaries | Includes Architectural Re-Centering section. |
| `docs/compute-reservoir.md` | Stable foundation | Compute Reservoir framing | Documents the alpha minimal crate and the boundary with Decision Gate, Graph Memory and Tool Registry. |
| `docs/tool-registry.md` | Stable foundation | Tool Registry framing | Documents the declarative registry boundary, explicit non-goals and alpha surface. |
| `docs/causal-trace.md` | Alpha foundation | Causal trace conventions | Documents current links and alpha audit trace queries for tasks, proposed actions, decisions and audit events without adding execution. |
| `crates/core` | Stable foundation | Domain vocabulary and pure types | Must not become a catch-all crate. Governance logic should stay in dedicated crates. |
| Core domain types | Stable foundation | Shared typed language | Includes minimal Failure-to-Insight vocabulary; remains pure, serializable and dependency-light. |
| Decision Gate | Alpha | Pre-execution governance | Extracted into `crates/decision-gate`; `crates/core` no longer reexports the Decision Gate logic. |
| Reservoir Echo | Alpha | Short-term cognitive continuity | Volatile traces only. Not persistent memory. Not model routing. Not Compute Reservoir. |
| Holographic Memory | Alpha domain vocabulary | Pattern resonance layer | Types exist in `crates/core`; no runtime behavior, vector DB, persistence, authorization or similarity execution yet. |
| Compute Reservoir | Alpha minimal | Compute/model/resource routing | `crates/compute-reservoir` provides serializable types and pure allocation only; no model calls, execution, I/O, persistence or Decision Gate replacement. |
| Tool Registry | Alpha minimal | Declarative catalogue of tools and permissions | `crates/tool-registry` declares tools, capabilities, schemas, governance notes and lookup/status changes only; no execution path. |
| `crates/graph-memory` | Experimental | SurrealDB Graph Memory adapter | Adds alpha audit-event queries by task, proposed action and decision plus governed FailureInsight memory trace proof readback, an in-memory demo/test helper and schema-backed CLI status readback; broader persistence conventions and graph schema still need stabilization. |
| Graph Memory domain port | Alpha | Memory contract | Useful foundation, but persistence and audit coupling are not final. |
| Audit System | Alpha | Trace important events and decisions | Has usable decision-scoped readback summaries; must remain non-authorizing. |
| `crates/llm` | Experimental | LLM provider abstraction | Must remain limited to proposals. No tool execution by provider. |
| `crates/runtime` | Experimental | Cognitive runtime loop | Must remain proposal-only until governance layers are ready for controlled integration. |
| `apps/api-server` | Alpha | REST access to alpha objects | Must not take business governance responsibility. |
| `crates/cli` | Alpha supervision surface | Local Mission Control precursor | Provides read-only audit, Failure-to-Insight, Graph Memory status, governed memory-write proposal supervision and a local FailureInsight memory demo loop. Must not become an execution bypass. |
| Neutral Orchestrator | Not implemented | Coordination layer | Deferred until governance, compute and tool layers are coherent enough for controlled integration. |
| Mission Control Web | Deferred | Human supervision UI | Do not expand yet. CLI supervision comes first. |
| Scheduler / autonomous loops | Deferred | Controlled recurring work | Must wait for Decision Gate, Tool Registry, Audit and human approval path. |
| MCP integration | Deferred | External tool ecosystem | Must wait for Tool Registry and security hardening. |
| Browser automation | Deferred | Controlled web interaction | Must wait for governance, audit and security hardening. |
| Security hardening | Deferred | Production-grade protection | Final V0 hardening stage, not a reason to bypass governance now.

## 3. What Is Stable

Stable foundations:

- the founding principle: no direct execution by agents;
- the canonical objective document;
- the canonical operational status document;
- the current operating doctrine and acceleration direction;
- the monorepo direction;
- Rust as backend foundation;
- local-first, graph-native, compute-aware, auditable and human-governed architecture;
- `ProposedAction -> DecisionGate -> Decision -> Audit` as the mandatory control path;
- separation between domain vocabulary and adapters as an architectural rule;
- documentation-level separation between Reservoir Echo and Compute Reservoir;
- the CLI as the preferred near-term local supervision surface;
- Failure-to-Insight as a stable documentary doctrine for turning failures and corrections into durable, non-authorizing learning artifacts;
- minimal `FailureInsight` domain vocabulary in `crates/core`, limited to pure serializable types and optional trace links.

## 4. What Is Experimental

Experimental areas:

- SurrealDB persistence details in `crates/graph-memory`;
- LLM provider behavior in `crates/llm`;
- runtime loop behavior in `crates/runtime`;
- API shape in `apps/api-server`;
- terminal UX in `crates/cli`;
- Reservoir Echo tuning and lifecycle;
- Holographic Memory adapter crate, vector similarity, persistence and runtime integration (domain vocabulary exists in `crates/core`);
- Compute Reservoir allocation heuristics and telemetry shape;
- audit persistence and causal trace design;
- future Failure-to-Insight audit conventions, CLI readback and broader Graph Memory integration;
- exact crate boundaries for remaining governance layers.

Experimental means: useful for learning, local supervision and integration tests, but not stable enough to justify external-effect execution around it.

## 5. What Must Not Be Implemented Yet

Do not implement yet:

- real tool execution;
- shell access;
- file deletion;
- email sending;
- scheduler autonomy;
- Mission Control UI;
- MCP integration;
- browser automation;
- multi-agent autonomy;
- self-modification;
- secrets access by LLM.

These capabilities are explicitly blocked until Decision Gate, Compute Reservoir, Tool Registry, Graph Memory persistence and Audit are stabilized enough for controlled integration.

Read-only CLI supervision work is allowed and encouraged, provided it does not approve, reject, execute, schedule, mutate external state, bypass the Decision Gate or treat readback as authorization.

## 6. Current Architectural Risks

Main risks:

- `crates/core` may become a catch-all crate.
- API, CLI, LLM and runtime layers are advancing before Tool Registry and before Compute Reservoir is stabilized beyond alpha minimal.
- Decision Gate is now a dedicated crate; downstream imports must keep using `arpagona-decision-gate` instead of reintroducing governance logic into `crates/core`.
- Reservoir Echo must not be confused with Compute Reservoir.
- No tool execution must be introduced before Tool Registry + Decision Gate + Audit are stable; the current Tool Registry is declarative only.
- API server and CLI could accidentally become privileged orchestration layers if responsibilities are not constrained.
- LLM provider abstraction could drift toward tool-calling unless explicitly kept proposal-only.
- Runtime loops could drift toward autonomy before human-governed control paths exist.
- Graph Memory and Audit could diverge unless important decisions produce durable, queryable traces.
- Development could drift back into endless test-only stabilization instead of shipping small read-only supervision surfaces.

## 7. Next Recommended Work

Recommended sequence from the current state:

1. Keep the Failure-to-Insight doctrine and minimal domain vocabulary visible in canonical contributor and focus-loop context.
2. In a later bounded implementation PR, add the smallest audit conventions needed to extract or reference `FailureInsight`, without adding execution, autonomy or authorization.
3. Prefer read-only CLI supervision increments that make the existing audit/task/action state inspectable.
4. Add more Graph Memory or Audit guards only when they protect a concrete uncovered regression risk or unblock a supervision feature.
5. Keep `crates/tool-registry` as a declarative catalogue only and harden it if gaps appear.
6. Stabilize `crates/compute-reservoir` only as needed for future governed integration and local/cloud delegation.
7. Expand API/Runtime only when the change remains read-only, clearly governed, or directly supports the CLI supervision path.

The Decision Gate extraction is complete, the Compute Reservoir exists as alpha minimal, and the Tool Registry now exists as alpha minimal declarative catalogue. Keep `crates/core` limited to domain vocabulary, keep governance logic in `crates/decision-gate`, and do not treat compute allocation, readback or tool lookup as action approval.

## 8. Target Architectural Order

The target consolidation order is now interpreted as controlled acceleration, not paralysis:

1. Core Domain Types
2. Decision Gate separated
3. Compute Reservoir minimal
4. Tool Registry minimal
5. Graph Memory + SurrealDB stabilized enough for readback
6. Audit System stabilized enough for readback
7. Failure-to-Insight vocabulary present; next conventions remain bounded and non-executing
8. CLI supervision surface
9. Neutral Orchestrator
10. API Server Axum
11. Mission Control Web
12. Scheduler / controlled autonomous loops
13. LLM Provider abstraction stabilized
14. End-to-end demo
15. Security hardening

Some components already exist experimentally outside this order. They must not be treated as permission to expand unsafe features. They may be grown when the growth is read-only, observable, reversible and aligned with CLI supervision or governed integration.

## 9. Explicit Stop-List for Unsafe Feature Expansion

Stop unsafe feature expansion until the governance layers are stabilized.

Do not add:

- executable tools;
- scheduler behavior;
- autonomous loops;
- Mission Control screens;
- MCP support;
- browser automation;
- unrestricted file access;
- shell integration;
- operational secrets management;
- agent self-modification;
- multi-agent autonomous execution;
- any CLI/API path that acts as approval, authorization, orchestration or execution state.

Allowed work during the current acceleration phase:

- read-only CLI supervision;
- documentation cleanup;
- crate boundary clarification;
- tests that protect newly exposed behavior or concrete uncovered risks;
- Compute Reservoir design and local/cloud delegation improvements;
- Tool Registry declarative design improvements;
- audit and graph persistence stabilization work that supports readback and does not introduce execution.

## 10. Session Update Rule

Every future agent must update `PROJECT_STATUS.md` after every significant modification.

A significant modification includes:

- adding, removing or renaming a crate;
- changing the responsibility of a crate;
- adding a new API or CLI surface;
- changing Decision Gate behavior;
- changing Graph Memory or Audit persistence/readback semantics;
- adding a provider, runtime loop, worker or interface;
- changing security assumptions;
- changing the project roadmap or implementation order.

The update must clearly state whether the change is stable, alpha, experimental, deferred or not implemented.

## 11. Latest Session Update

This session added structured JSON output to the read-only local supervision status CLI.
This session added a cross-invocation demo snapshot path that proves the governed FailureInsight learning loop output survives across separate process invocations.

Changed:
- added `crates/graph-memory/src/demo_snapshot.rs` with `FailureInsightDemoSnapshot` struct, `write_to_file`/`read_from_file` methods, `SnapshotError` type and 4 unit tests (round-trip, bare filename, missing file error, invalid JSON error);
- added `pub mod demo_snapshot;` to `crates/graph-memory/src/lib.rs`;
- extended `arpagona memory demo failure-insight` with an optional `--snapshot-path <path>` flag: when provided, the demo writes the readback state as a JSON snapshot file to disk after the in-memory demo succeeds;
- added `arpagona memory demo snapshot-read <path> [--json]` subcommand that reads and displays a previously written snapshot file, proving cross-invocation readback;
- all new code uses only `serde` + `serde_json` + `std::fs` — no native SurrealDB backend dependencies, no feature flags, no build-time gates.
- all verification passes: `cargo fmt -- --check`, `cargo check`, `cargo test` (132+4 new tests all passing).

- changed `arpagona status` to accept `--json` in `crates/cli`;
- reused the existing `StatusReadback` shape for human and JSON output;
- documented `arpagona status --json` in `docs/cli.md`;
- added CLI parser coverage for the new flag.

Stability level: alpha CLI supervision surface.

Limits:
- no endpoint was added;
- no server-side state was modified;
- no Graph Memory schema, query or mutation was added;
- no audit event creation or extraction behavior was added;
- no runtime behavior was added;
- no real tool execution was introduced;
- no destructive capability was added;
- no approval, rejection or authorization behavior was added;
- no Decision Gate behavior was changed;
- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;

Stability level: alpha CLI demo/readback. This change adds the missing `approved persistence -> cross-invocation readback -> repeatable demo` link to the functional-alpha chain. It remains a simulated/internal Graph Memory proof and does not add durable user memory, approval, authorization, execution or external side effects.

Limits:
- no broad CLI mutation command was added;
- no API endpoint was added;
- no new Graph Memory persistence helper was added;
- no Decision Gate behavior was changed;
- no durable/file-backed SurrealDB configuration was added;
- no LLM/provider/runtime direct memory mutation was added;
- no broad autonomous memory writing was added;
- no personal or sensitive memory path was added;
- no database migration runner was added;
- no broad semantic search or embeddings pipeline was added;
- no hidden context injection into LLM prompts was added;
- readback remains evidence only and must not be treated as authorization.

- low. The change adds read-only artifact inspection inside an already bounded local demo path. The main risk is operator confusion if in-memory demo readback is mistaken for durable memory; the readback warnings and handoff explicitly preserve the evidence-only, local-demo boundary.

Recommended next step: replace the in-memory-only FailureInsight demo inspection with an explicitly configured local SurrealDB persistence/readback path if the adapter can support a safe file-backed configuration, without adding broad mutation or authorization paths.

## Latest Session Update (2026-05-24 — Holographic Memory domain vocabulary)

This session added the minimal pure-domain vocabulary for Holographic Memory — an experimental cognitive resonance layer that stores distributed pattern signatures of cognitive experience. This aligns with the recentred project philosophy: **Cognitive ambition first. Governance as the immune system.**

Changed:
- Created `crates/core/src/holographic.rs` with pure serialisable types:
  - `HolographicTraceKind` enum (TaskPattern, ActionChainPattern, FailurePattern, SuccessPattern, ConversationPattern, ComputeRoutingPattern, DecisionPattern, ToolUsePattern, CognitiveCyclePattern)
  - `HolographicPatternKind` enum (FailurePrototype, SuccessPrototype, RoutingPrototype, ConversationDriftPrototype, DecisionBoundaryPrototype, ToolUsePrototype, CognitiveCyclePrototype)
  - `HolographicTrace` struct with workspace_id, source_episode_id, vector (Vec<f32> placeholder), labels, strength, decay
  - `HolographicPattern` struct with workspace_id, prototype_vector (Vec<f32> placeholder), support_count, confidence, labels
  - `HolographicQuery` struct (workspace_id, query_vector, top_k, min_similarity)
  - `HolographicMatch` struct (trace_id, similarity, matched_labels, linked_episode_id)
- Added `HolographicTraceId` and `HolographicPatternId` to `crates/core/src/ids.rs`
- Added `HolographicMemory` variant to `CognitiveLayer` enum in `crates/core/src/cognitive.rs`
- Exported the new module in `crates/core/src/lib.rs`

Stability level: stable domain vocabulary. Pure types, serialisable, zero-dependency beyond chrono + serde, no execution logic, no vector database, no persistence adapter, no runtime integration, no Decision Gate bypass, no authorisation of any kind. The `Vec<f32>` fields are placeholders — not computed, persisted or queried.

Tests: 8 tests pass including `holographic_memory_is_non_authorizing_by_design` which explicitly verifies no governance fields exist.

Recommended next step: add a read-only CLI `arpagona memory holographic status --json` command (Option B) or create a `HolographicStore` trait analogous to `GraphMemoryStore`.

## Latest Session Update (2026-05-24 — Demo snapshot path for cross-invocation readback)

This session added a cross-invocation demo snapshot path for the governed FailureInsight learning loop, proving readback across separate process invocations without unstable SurrealDB cfg flags or native RocksDB/zstd dependencies.

Changed:
- added `crates/graph-memory/src/demo_snapshot.rs` with a `FailureInsightDemoSnapshot` struct, JSON serialize/deserialize, `write_to_file()` and `read_failure_insight_demo_snapshot()` public API;
- extended `arpagona memory demo failure-insight` with `--snapshot-path <path>` flag that persists the demo readback as a pure-Rust JSON snapshot file after the in-memory demo succeeds;
- added `arpagona memory demo snapshot-read <path>` subcommand that reads and formats a demo snapshot JSON file from disk;
- updated the functional-alpha chain to include the snapshot step: `demo snapshot written for cross-invocation readback proof`;
- updated the repeatable demo recipe with snapshot-path and snapshot-read steps.

Stability level: alpha CLI demo persistence. This change uses pure Rust stdlib file I/O and serde for cross-invocation snapshot persistence. It requires no native toolchain dependencies, no unstable cfg flags, and leaves `cargo fmt -- --check && cargo check && cargo test` green by default. The snapshot mechanism is a development-only proof, not a production persistence mechanism.

Limits:
- no Cargo feature flag was added (the snapshot path is pure Rust stdlib/serde, no build-time gate needed);
- no SurrealDB backend change was made (`kv-mem` remains the default);
- no migration runner was added;
- no broad CLI mutation command was added;
- no API endpoint was added;
- no Decision Gate behavior was changed;
- no LLM/provider/runtime direct memory mutation was added;
- no broad autonomous memory writing was added;
- no personal or sensitive memory path was added;
- no real tool execution was introduced;
- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;
- readback remains evidence only and must not be treated as authorization.

Architectural risk:
- low. The snapshot path is pure Rust JSON I/O behind an optional CLI flag. The main risk is operator confusion between demo snapshot files and real persistent memory; the evidence-only token and readback warnings explicitly preserve the demo-only, non-authorizing boundary.

- low for alpha use. The change is bounded to read-only CLI status readback formatting and documentation, preserving the separation between supervision output, governance and execution.

Recommended next step: add the next small read-only CLI supervision increment that helps operators inspect pending actions, decisions or failure insights without expanding API/runtime execution.

## Latest Session Update (2026-05-24 — CLI integration test for cross-invocation snapshot readback)

This session added a CLI-level integration test that runs the full demo snapshot-then-read cycle end-to-end, proving the governed FailureInsight learning demo persists and readably survives a separate process invocation.

Changed:
- added `cross_invocation_demo_snapshot_proves_readback_across_process_invocations` test in `crates/cli/src/main.rs` that:
  1. invokes the built `arpagona` binary with `memory demo failure-insight --json --snapshot-path <path>`;
  2. verifies the snapshot file was created;
  3. invokes the built `arpagona` binary with `memory demo snapshot-read <path> --json` in a separate process;
  4. asserts the readback JSON contains the `evidence_only_token`, `functional_alpha_chain` and the cross-invocation snapshot chain step.

Stability level: alpha CLI integration test. The test uses `std::process::Command` to run the built binary, proving the governed learning loop output survives serialization, file I/O, process restart and deserialization. It requires no SurrealDB backend, no unstable cfg flags, no native dependencies, and runs in ~0.03s.
- no Cargo feature flag was added;
- no SurrealDB backend change was made (`kv-mem` remains the default);
- no migration runner was added;
- no broad CLI mutation command was added;
- no API endpoint was added;
- no Decision Gate behavior was changed;
- no LLM/provider/runtime direct memory mutation was added;
- no broad autonomous memory writing was added;
- no personal or sensitive memory path was added;
- no real tool execution was introduced;
- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;
- readback remains evidence only and must not be treated as authorization.

Architectural risk:
- low. The test uses `CARGO_BIN_EXE_arpagona`, a stable Cargo-provided environment variable available to integration tests in the same package that declares the `arpagona` binary target. This is the canonical way to reference companion binaries in cross-process integration tests.

Recommended next step: consider making the demo snapshot path the standard persistence proof for operator demos and add a section to `docs/failure-to-insight.md` documenting the cross-invocation readback verification procedure.

## Latest Session Update (2026-05-24 — Cherry-pick and deliver demo snapshot PR)

This session cherry-picked 4 existing commits from the previous `feat/cli-integration-test-snapshot` branch onto a fresh branch (`feat/demo-snapshot-persistence-v2`) based on the latest `main`, then ran full verification and pushed/PR'd the branch.

The 4 commits (in order):
1. `feat: add demo snapshot path for cross-invocation FailureInsight readback proof`
2. `Harden demo snapshot persistence path`
3. `feat: add CLI integration test for cross-invocation demo snapshot readback`
4. `docs: add cross-invocation readback verification section to failure-to-insight`

Changed:
- cherry-picked `crates/graph-memory/src/demo_snapshot.rs` (pure-Rust JSON snapshot persistence)
- cherry-picked `crates/cli/src/main.rs` extensions (--snapshot-path flag, snapshot-read subcommand, cross-invocation integration test)
- cherry-picked `docs/failure-to-insight.md` cross-invocation verification section
- updated `FOCUS_LOOP_NEXT.md` with next handoff

Verification:
- `cargo fmt -- --check`: clean
- `cargo check`: clean
- `cargo test`: 133 tests pass, including `cross_invocation_demo_snapshot_proves_readback_across_process_invocations`
- Manual CLI demo: snapshot written and read back across separate process invocation

Stability level: alpha CLI demo persistence. No new code was added by this session; existing work was delivered as a PR.

- readback remains evidence only and must not be treated as authorization.

Architectural risk:
- low. The snapshot path uses only serde_json + std::fs with no native database dependencies. The `evidence_only_token` prevents readback-as-authorization drift. All snapshot operations are separate from the demo execution path and do not affect correctness.

Recommended next step: extend the demo snapshot path to include an optional cross-process integration test that runs the `failure-insight --snapshot-path` demo in one process, then runs `snapshot-read` in a separate process to assert readback fields — providing a fully automated CI-proof cross-invocation governed memory persistence test.

## 12. Latest Session Update

This session resolved merge conflicts in the repository (demo_snapshot.rs, snapshot_integration.rs, lib.rs) and added operator-supplied input to the governed FailureInsight learning demo.

Changed:
- **Merge conflict fixes**:
  - Restored `crates/graph-memory/src/demo_snapshot.rs` to the clean version from commit `3db91e4` (origin/main PR #64), removing HEAD/origin/main conflict markers
  - Cleaned `crates/cli/tests/snapshot_integration.rs` — removed conflict markers, kept both integration tests (cross-invocation readback + missing file error)
  - Removed duplicate `pub mod demo_snapshot;` from `crates/graph-memory/src/lib.rs`
  - Removed dead duplicate snapshot persistence block from `memory_demo_failure_insight` (was calling old one-arg `FailureInsightDemoSnapshot::new()` API)
  - Fixed `memory_demo_snapshot_read` to use `FailureInsightDemoSnapshot::read_from_file()` instead of the removed standalone `read_failure_insight_demo_snapshot()` function
  - Added missing `--json` flag to `MemoryDemoFailureInsightArgs`
- **Operator-supplied input** (`--description` flag):
  - Added `--description <text>` flag to `arpagona memory demo failure-insight`
  - When provided, the FailureInsight is constructed from the operator's custom description instead of the hardcoded default
  - Updates all FailureInsight fields (summary, impact, root cause, recommended action) with operator text
  - Added parser test: `cli_parses_memory_demo_failure_insight_with_description`
- Fixed `FOCUS_LOOP_NEXT.md` which had unresolved conflict markers (`>>>>>>> origin/main`)

Stability level: alpha CLI demo/readback. The `--description` flag is a read-only input to an already-bounded local demo; it does not add mutation, authorization, persistence, or external effects.

Limits:
- no broad CLI mutation command was added;
- no API endpoint was added;
- no new Graph Memory persistence helper was added;
- no Decision Gate behavior was changed;
- no SurrealDB backend change was made;
- no LLM/provider/runtime direct memory mutation was added;
- no real tool execution was introduced;
- no destructive capability was added;
- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;
- readback remains evidence only and must not be treated as authorization.

Architectural risk:
- low. The `--description` flag is pure CLI input parsing. The demo uses `FailureInsight::new()` which is a pure domain constructor with no side effects. All existing tests continue to pass without modification (the `None` default preserves backward compatibility).

Recommended next step: add an end-to-end integration test proving that `--description` text flows through the full governed path (signal → proposal → decision → audit → persistence → readback) and appears in the readback output.

## 13. Cognitive Tool Runtime — Alpha Read-Only Foundation

This session created the first operational bridge between ARPAGONA's cognitive vocabulary and real filesystem perception.

### What was added

**`crates/tool-runtime`** — new crate providing an alpha read-only tool runtime with 3 tools:

- **`read_file`** — read a file within the workspace (blocked: absolute paths, parent traversal, sensitive files, large files)
- **`list_files`** — list directory entries (skips `.git`, `target`, `node_modules`, `.env`, `.ssh`)
- **`search_text`** — search for text patterns in workspace files (bounded results, bounded file sizes)

All tools are:
- read-only
- locally scoped to the workspace
- size-limited and count-limited
- returning structured `ToolExecutionResult` with observation, error, audit hint and failure-insight-candidate flags

**`crates/core`** — new cognitive vocabulary types:

- `ToolIntent` — full tool intention with rationale, purpose, risk, fallback
- `ToolExecutionRequest` — concrete execution request
- `ToolExecutionResult` — structured result with status, observation, error
- `ToolExecutionStatus` — Success, Warning, Failed, Blocked, Skipped
- `ToolExecutionError` — typed error with security flag
- `ToolObservation` — observation with actionable/failure-insight-candidate markers
- `ToolUseRationale` — justification, expected observation, downstream use, risk assessment
- `ToolExecutionMode` — Simulate, Execute, RequireHumanConfirmation
- `ToolRiskLevel` — None, Low, Medium, High, Critical
- `CognitivePurpose` — Perception, Recall, Inspection, Transformation, Validation, Execution, Communication, Reflection
- `FallbackStrategy` — Retry, UseAlternative, ReportOnly, EscalateToHuman

**`crates/tool-registry`** — extended with cognitive concepts:

- `ToolCognitiveRole` — Perception, Recall, Inspection, Transformation, Validation, Execution, Communication, Reflection
- `ToolRiskProfile` — risk profile for tool declarations
- Extended `ToolCapability` with ReadFile, ListFiles, SearchText, ShellAccess, EmailSend, BrowserAutomation, MCPAccess
- `is_safe_for_read_only()` and `is_non_executable()` methods on roles
- `ToolCognitiveRole::Transformation`, `Execution`, `Communication` are marked non-executable

**CLI** — new `arpagona tool` commands:

- `arpagona tool list` — list available tools
- `arpagona tool inspect <name>` — show tool details
- `arpagona tool demo read-file <path>` — execute read-only read_file
- `arpagona tool demo list-files [path]` — execute read-only list_files
- `arpagona tool demo search-text <query> [path]` — execute read-only search_text
- All commands support `--json` for structured output

**Documentation**:

- `docs/cognitive-tool-runtime.md` — comprehensive design document explaining why tools are necessary for cognition, why execution must be controlled, the architecture overview, each tool's cognitive role, how Hermes inspired the selection, why read-only first, what remains non-executable, and what comes next

### What was NOT added

- No scheduler, browser, shell-free, write, email, MCP, secrets, API endpoint, self-modification, autonomy, multi-agent runtime, Holographic Memory vector store, SurrealDB persistence, or LLM integration
- No Decision Gate bypass — the runtime is a local demo layer; governance integration is architectural preparation only
- The `ToolExecutionResult` carries `failure_insight_candidate` flags but does not auto-generate FailureInsights

### Stability level

Alpha experimental. All 3 tools are proven by unit tests (13 tests in tool-runtime). The core vocabulary has 7 new tests, the tool-registry has 5 new tests. The CLI commands compile and dispatch correctly.

### Test count

- `arpagona-agent-core`: 42 tests (35 existing + 7 new)
- `arpagona-tool-registry`: 11 tests (6 existing + 5 new)
- `arpagona-tool-runtime`: 13 tests (all new)
- **Total**: 66 tests across the 3 crates

### Recommended next step

Connect `ToolExecutionResult` to the Audit system and Failure-to-Insight pipeline, so that failed observations automatically produce candidate `FailureInsight` records. Then add the `search_text` and `list_files` results to the Working Memory / Reservoir vocabulary for context-grounded agent behaviour.

## 14. Latest Session Update

This session added an end-to-end integration test proving that `--description` text propagates through the full governed FailureInsight path (signal → proposal → decision → audit → persistence → readback → inspection).

Changed:
- Added `description_propagates_through_governed_failure_insight_path` test to `crates/cli/src/main.rs` that:
  1. Calls `memory_demo_failure_insight_readback` with a custom inspect_id and custom description
  2. Verifies the governed path is intact (proposal type, decision status, readback found, audit events, relations)
  3. Asserts the custom description appears in the FailureInsight inspection summary
  4. Asserts the custom description appears in the formatted readback text output
  5. Verifies the no-authorization invariant (warning, evidence-only next step)

Stability level: alpha CLI integration test. Pure in-memory, no external effects, no persistence, no SurrealDB.

Limits:
- no broad CLI mutation command was added;
- no API endpoint was added;
- no new Graph Memory persistence helper was added;
- no Decision Gate behavior was changed;
- no SurrealDB backend change was made;
- no LLM/provider/runtime direct memory mutation was added;
- no real tool execution was introduced;
- no destructive capability was added;
- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;
- readback remains evidence only and must not be treated as authorization.

Architectural risk:
- low. The test is pure in-memory async with no external side effects. It reuses the existing `memory_demo_failure_insight_readback` function with existing `--description` support.

Recommended next step: add a CLI end-to-end demo recipe or a documented operator workflow that shows how to run `arpagona memory demo failure-insight --description "..."` locally and inspect the governed learning loop output. The test proves it works; a recipe makes it usable.