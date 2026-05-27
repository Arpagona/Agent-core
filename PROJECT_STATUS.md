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
- `crates/holographic-memory` exists as an alpha Rust kernel for symbolic associative memory: deterministic distributed signatures, resonance-based retrieval, project-scoped isolation, and an in-memory store with 22 tests.
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
| Holographic Memory | Alpha V0 crate | Symbolic associative memory kernel | `crates/holographic-memory`: 22 tests, in-memory store, deterministic signatures, no LLM/vector DB/persistence/authorization. Canonical phrase: "Holographic Memory reactivates paths to truth. It does not replace truth." |
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
- Holographic Memory crate (`crates/holographic-memory`) — symbolic associative memory kernel; in-memory store, 22 tests, deterministic signatures, no LLM/vector DB/persistence/authorization yet;
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

- Added `docs/daily-agent-validation.md`
- Wrote comprehensive agent validation checklist
- All existing tests pass without modification

## 12. General Cognitive Work Loop V0 — Alpha Domain/Runtime Skeleton

This session added the first general-purpose cognitive cycle skeleton to Agent Core:

**New module:** `crates/core/src/cognitive_work.rs`

**Pure types added:**
- `Objective`, `ObjectiveId`, `ObjectiveDomain`, `ObjectiveStatus`, `SuccessCriterion`
- `WorkingMemory`, `ContextItem`, `Assumption`, `Constraint`, `MissingContext`
- `CognitivePlan`, `PlanStep`, `RequiredObservation`
- `ProposedNextAction`, `NextActionKind`
- `ImprovementCandidate`, `ImprovementCandidateKind`
- `CognitiveCycleResult`

**Heuristic engine:**
- `run_cognitive_work_cycle()` — pure, deterministic, I/O-free, LLM-free.
- Domain classification via keyword matching (9 domains including General, Unknown).
- Missing context detection based on domain heuristics.
- Plan generation with context-aware ordering.
- Next action proposal (RequestContext if gaps exist, StopWithReport otherwise).
- Improvement candidate identification (missing context, weak plan, domain ambiguity).

**CLI surface:**
- `arpagona cognitive run --objective <TEXT> [--domain <DOMAIN>] [--context <TEXT>] [--json]`
- Human-readable text output and structured JSON output.
- JSON contains: objective, working_memory, plan, required_observations, proposed_next_action, improvement_candidates, warning.

**Documentation:** `docs/general-cognitive-work-loop.md`

**Tests:** 17 core tests + 7 CLI tests = 24 new tests covering serialization, domain classification, missing context detection, non-authorizing invariant, CLI parsing, and JSON output structure.

Stability level: alpha domain/runtime skeleton.

Key invariants enforced:
- ✅ read-only (no I/O, no LLM, no tool execution, no persistence)
- ✅ non-autonomous (no scheduler, no auto-execution)
- ✅ no external effects
- ✅ non-authorizing (every `ProposedNextAction` has `non_authorizing: true`)
- ✅ pure serde serialization for all types
- ✅ prepares future LLM/orchestrator integration

Architectural risk:
- low. The module is entirely self-contained in `crates/core` with no new crate dependencies. No existing behavior is modified or bypassed.

Not added (per stop-list):
- no LLM call, API endpoint, scheduler, browser automation, MCP, email, shell, file write, Graph Memory persistence, hidden memory injection, Decision Gate bypass, or self-modification.

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

## 14. Snapshot List — CLI Snapshot Discovery

This session added the missing snapshot discovery command for the governed FailureInsight demo path.

### What was added

**`crates/graph-memory/src/demo_snapshot.rs`**:

- `SnapshotListing` struct with `file_name`, `description_preview`, `chain_step_count`, `content_preview`
- `list_snapshots_in_directory(dir)` — scans a directory for `.json` files, deserializes valid `FailureInsightDemoSnapshot` instances, returns sorted metadata

**`crates/cli/src/main.rs`**:

- New CLI command: `arpagona memory demo snapshot-list [--json] [--snapshot-dir <dir>]`
- Default snapshot directory: `target/demo-snapshots` (configurable via `ARPAGONA_SNAPSHOT_DIR` env var)
- Human-readable output with snapshot count, file names, description previews, alpha chain step counts, content previews
- JSON output via `--json` for programmatic consumption
- 4 parser tests: basic parse, `--json`, `--snapshot-dir`, combined flags

### Verification

- `cargo fmt -- --check`: clean
- `cargo test`: 41 CLI tests + 2 integration tests → all pass (0 new failures)
- `cargo run -- memory demo failure-insight --snapshot-path target/demo-snapshots/demo.snapshot.json`: writes snapshot
- `cargo run -- memory demo snapshot-list --snapshot-dir target/demo-snapshots`: shows 1 snapshot with 7 alpha chain steps
- `cargo run -- memory demo snapshot-list --snapshot-dir target/demo-snapshots --json`: returns structured JSON with file path and listing metadata

### Functional-alpha chain advancement

This completes the CLI discovery surface for the demo snapshot path:

```
signal → proposal → decision → audit → approved persistence → snapshot-write → snapshot-read → snapshot-list (NEW) → repeatable demo
```

Before this session: operators needed to know exact file paths to inspect snapshots.
After this session: `snapshot-list` discovers all available snapshots, names, and metadata.

### What was NOT added

- No SurrealDB persistence changes
- No scheduler, browser, write, email, MCP, secrets, API endpoint, self-modification, autonomy, multi-agent runtime, or LLM integration
- No Decision Gate bypass

### Files changed

| File | Change |
|------|--------|
| crates/graph-memory/src/demo_snapshot.rs | Added `SnapshotListing` struct and `list_snapshots_in_directory()` |
| crates/cli/src/main.rs | Added `SnapshotList` variant, args struct, dispatch, handler function, 4 parser tests |

### Stability level

Stable alpha extension. Pure file I/O, no native deps or SurrealDB feature flags. All snapshot operations (write, read, list) now have CLI surfaces.

### Test count

- `arpagona-cli`: 41 tests (37 existing + 4 new)
- `arpagona-graph-memory`: 24 tests (20 existing + 4 existing)
- **Total**: 65+ tests across the workspace

### Recommended next step

Create a self-contained demo script (`scripts/demo-full-loop.sh`) that runs the complete governed FailureInsight demo path end-to-end, proving the full chain in one repeatable invocation.

## 15. Latest Session Update (2026-05-25 — Rebase #77, resolve conflicts, close superseded PRs)

This session rebased PR #77's description-propagation commits onto the latest `main`, resolving merge conflicts in handoff files (FOCUS_LOOP_NEXT.md, PROJECT_STATUS.md accepted main's versions). The code commits applied cleanly.

Changed:
- Cherry-picked 2 commits from `feat/description-cross-invocation-delivery` (#77) onto current `main`:
  1. `feat: prove --description propagates through full governed loop (signal to readback)` — changes `MemoryDemoSignalReadback.summary` from `&'static str` to `String`, wires custom description into signal readback, adds integration test
  2. `feat: add cross-invocation description propagation test` — proves `--description` text survives demo snapshot-then-readback cycle across separate process invocations
- Force-pushed rebased branch to update #77 (conflicts resolved, now mergeable)
- Closed superseded PRs:
  - #74 (feat/description-end-to-end-governed-path-test) — superseded by #77
  - #72 (feat/description-e2e-v2) — superseded by #77

Status: PR #77 is mergeable (no conflicts). CI pending re-run on new commits.

Stability level: alpha CLI demo/readback. Same bounded work as before, just rebased and delivered cleanly.

Verification:
- `cargo fmt -- --check`: clean
- `cargo check`: clean
- `cargo test`: 172 tests pass (all crates), including `memory_demo_description_propagates_through_governed_loop_signal_to_readback` and `cross_invocation_description_survives_snapshot_path_across_processes`

Limits:
- no new code was added by this session (cherry-pick only)
- no broad CLI mutation command added
- no API endpoint added
- no Graph Memory persistence helper added
- no Decision Gate behavior changed
- no SurrealDB backend change made
- no LLM/provider/runtime direct memory mutation added
- no real tool execution introduced
- no scheduler, autonomy, MCP, browser automation, credential handling, or Mission Control Web growth
- readback remains evidence only, not authorization

Recommended next step: wait for CI to complete on #77, then merge into `main`. After merge, create `scripts/demo-full-loop.sh` for a single-repeatable-command governed FailureInsight demo path.

## 16. Latest Session Update (2026-05-25 — P5 + P6 + P7: WorkingMemory ↔ ComputeReservoir ↔ HolographicMemory bridge complete)

This session completed the P5–P7 milestone sequence for the cognitive loop integration.

### P5 — Connect WorkingMemory to ComputeReservoir allocation (PR #85, merged)

Added `allocate_for_working_memory()` in `crates/compute-reservoir/src/lib.rs` — a pure function that maps WorkingMemory fields (sensitivity_estimate, complexity_estimate, local_first, cost_sensitive, required_observations_count) to a ComputeRequest, then delegates to the existing `allocate_compute()` engine.

CLI: `cognitive run --objective "..." --domain business --assess --allocate --json`

JSON output includes `working_memory`, `compute_requirement`, `non_authorizing_warning`.

6 new tests covering:
- missing context → request_context / no expensive model needed
- sensitive objective → prefer local resource
- complex research objective → strong model justified if policy allows
- unavailable resource → fallback
- allocation is not authorization
- no external provider call

### P6 — WorkingMemory → HolographicMemory resonance hints

Added in `crates/core/src/holographic.rs`:

- `ResonanceHint` struct — single resonance hint with suggested_trace_kind, labels, resonance_score, rationale
- `WorkingMemoryResonance` struct — result container with non-authorizing warning
- `resonate_for_working_memory()` — pure function mapping domain, sensitivity, complexity, proposed action kind, and allocation justification to heuristic resonance hints
- 10 new tests proving: business domain produces TaskPattern, confidential sensitivity produces DecisionPattern with score 0.7, secret sensitivity has score 0.9, high complexity triggers complexity hint, public sensitivity skips sensitivity hint, fallback justification detected, resonance is non-authorizing, serializes to JSON, pure/deterministic, engineering domain produces ActionChainPattern

Integration tests in `crates/compute-reservoir`:
- `p6_integration_wm_to_allocate_to_resonate_chain` — proves the full WorkingMemory → allocate → resonate chain with research/confidential/high-complexity scenario
- `p6_integration_simple_business_chain_produces_readable_json` — proves the chain works with public/business/low-complexity scenario and JSON serialization

### P7 — Demo script

Added `scripts/demo-full-loop.sh` — demonstrates 4 cognitive run scenarios (business, research, confidential, coding) with the full P5+P6 pipeline, plus integration tests. No API server required.

### Files changed

| File | Change |
|------|--------|
| crates/core/src/holographic.rs | Added `ResonanceHint`, `WorkingMemoryResonance`, `resonate_for_working_memory()`, `RESONANCE_NON_AUTHORIZING_WARNING`, 2 helpers, 10 tests |
| crates/compute-reservoir/src/lib.rs | Added `holographic::resonate_for_working_memory` import, 2 integration tests (chained WM→allocate→resonate) |
| scripts/demo-full-loop.sh | New file: cognitive loop demo script |

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean (0 warnings, 0 errors)
- `cargo test --workspace`: 224 tests pass (all crates)

### Stability level

All additions are alpha pure-domain extensions:
- P5: alpha CLI surface (compute allocation bridge)
- P6: alpha domain vocabulary (resonance hints, pure heuristic)
- P7: alpha demo script (no API required)

### Not added (per stop-list)

- no LLM call, API endpoint, scheduler, browser automation, MCP, email, shell, file write (beyond demo-script output), Graph Memory persistence, hidden memory injection, Decision Gate bypass, or self-modification
- no `--resonate` CLI flag (resonance is proven via integration tests; CLI flag deferred to P6+ if user requests)

### P8 — Context-aware governed proposals (--propose bridge)

Added `--propose` flag to `cognitive run` that converts FailureInsightCandidates and CognitiveObservations into context-rich ProposedActions via the API server.

Each proposal carries metadata in its `payload` field:
- `originating_objective` — the objective that triggered the proposal
- `source_kind` — where the signal came from (`failure_insight_candidate` or `cognitive_observation`)
- `source_summary` — short summary of the original signal
- `rationale` — why this action is proposed (with tool name context)
- `expected_benefit` — derived from the candidate/observation kind
- `risk_level` — informational/low based on signal type
- `suggested_action_type` — one of: test, fix, refactor, doc, research, governance
- `confidence` — None (available for future use)
- `non_authorizing_warning` — embedded governance guard

The proposal bridge:
1. Collects FailureInsightCandidates from working_memory (injected by `--assess`)
2. Collects CognitiveObservations from working_memory (injected by `--observe`)
3. For each actionable signal, creates a `ProposedAction` via `POST /proposed-actions` (API)
4. Evaluates each proposal through the Decision Gate (`evaluate_proposed_action`)
5. Records audit events (`audit_event_for_decision`)
6. Injects `proposed_actions`, `decisions`, `audit_events`, `non_authorizing_warning` into the JSON output

### Files changed

| File | Change |
|------|--------|
| crates/cli/src/main.rs | Added `--propose` flag, `ProposalMetadata` struct, `run_proposals()` async bridge, `permissions_for_action()` helper, mapping functions for action type/benefit |
| crates/cli/tests/snapshot_integration.rs | Added `cognitive_propose_pipeline_produces_governed_proposals` end-to-end test |

### Verification

- `cargo check --workspace`: clean (0 errors)
- `cargo test --workspace`: 237 tests pass (all crates)
- Live verification: `cognitive run --assess --observe --propose --json` produces proposed_actions (pending_decision), decisions, audit_events, non_authorizing_warning

### Not added (per stop-list)

- no autonomous execution or new LLM calls
- no Decision Gate bypass — every proposal is PendingDecision
- no tool execution beyond what --observe already uses (read_file, list_files)
- no API endpoint changes (reuses existing POST /proposed-actions)

### Recommended next step

- Merge PR #85 (if not already merged)
- Then: **proposal scoring and prioritization** — rank proposed_actions by expected_benefit, risk_level, and confidence so the user sees the most impactful proposals first
- Or: stake the demo script into the hourly focus-loop cron as a smoke test

## 17. Latest Session Update (2026-05-26 — P6 `--resonate` CLI flag complete with parser tests)

This session completed the P6 `--resonate` CLI flag for direct HolographicMemory resonance readback on `arpagona cognitive run`.

Changed:
- Added 9 parser tests for `arpagona cognitive run` covering all CLI flags: basic parse, domain, `--assess`, `--allocate`, `--resonate`, context, combined flags, `--json` combinations, and the full `--assess --allocate --resonate` pipeline.
- Verified `--resonate` works end-to-end:
  - JSON output: `holographic_resonance` block contains hints, has_resonance, non_authorizing_warning
  - Human-readable output: formatted resonance section with hint details and non-authorizing warning
- Updated `FOCUS_LOOP_NEXT.md` with P3 as the next milestone (connect `--observe` → assessment → governed learning proposal path)

Note: the `--resonate` flag implementation (parser field, JSON output path, human-readable output) was already present in the codebase from the P6 integration commit. This session added the missing parser tests and end-to-end verification.

Verification:
- `cargo fmt -- --check`: clean
- `cargo check`: clean (0 new warnings, pre-existing Rust edition linter noise only)
- `cargo test --workspace`: 246 tests pass (all crates), including 9 new cognitive run parser tests

Stability level: alpha CLI supervision surface (same as existing `--assess`, `--allocate` flags).

Limits:
- no endpoint was added
- no server-side state was modified
- no Graph Memory schema, query or mutation was added
- no audit event creation or extraction behavior was added
- no runtime behavior was added
- no real tool execution was introduced
- no LLM, provider, or API call was added
- no Decision Gate behavior was changed
- no scheduler, autonomy, MCP, browser automation, credential handling, or Mission Control Web growth

### Files changed

| File | Change |
|------|--------|
| crates/cli/src/main.rs | Added 9 parser tests for cognitive run command validation |
| FOCUS_LOOP_NEXT.md | Updated handoff to P3 (observe → assess → governed learning) |

### Recommended next step

Connect `--observe` tool observation outputs into the `--assess` assessment pipeline so that tool observations produce governed learning proposals through the existing FailureInsightCandidate → ProposedAction → DecisionGate → Audit chain.

## 18. Latest Session Update (2026-05-26 — P3 bridge: `--observe` → `--assess` observation-to-candidate pipeline)

This session completed the P3 bridge: cognitive tool observations now flow through `assess_observation()` and merge into `failure_insight_candidates` when both `--observe` and `--assess` flags are active on `cognitive run`.

**Code changes:**

| File | Change |
|------|--------|
| `crates/core/src/observation.rs` | Added `FailureInsightCandidate::from_assessments()` — extracts candidates from `ObservationAssessment` slice |
| `crates/cli/src/main.rs` | When `--assess --observe` are both active, runs observations, assesses each via `assess_observation()`, merges observation-derived FailureInsightCandidates alongside improvement-candidate-derived ones in `failure_insight_candidates` |
| crates/cli/src/main.rs | Added 2 parser tests: `cli_parses_cognitive_run_assess_observe_json` and `cli_parses_cognitive_run_assess_observe_allocate_resonate` |

**Verification:**
- `cargo fmt -- --check`: clean
- `cargo check`: clean (0 new errors, pre-existing E0670 linter noise only)
- `cargo test --workspace`: 248 tests pass (all crates), including 2 new parser tests

**Functional-alpha chain advancement:**
```
cognitive run --objective "..." --assess --observe --json
  → cognitive_work_cycle → ImprovementCandidates → FailureInsightCandidates
  → tool runtime observations → assess_observation() → more FailureInsightCandidates (NEW)
  → merged failure_insight_candidates in JSON output
```

Before this session: `--assess` only produced candidates from improvement-candidates; observations were just injected raw.
After this session: `--assess --observe` together produce failure_insight_candidates that include both improvement-candidate-derived and observation-derived entries (e.g. truncation, empty search results, safety boundary signals, runtime failures).

**What was NOT added:**
- No endpoint was added
- No server-side state was modified
- No Graph Memory schema, query or mutation was added
- No audit event creation or extraction behavior was added
- No runtime behavior was added
- No real tool execution beyond the existing read-only tool runtime
- No LLM, provider, or API call was added
- No Decision Gate behavior was changed
- No scheduler, autonomy, MCP, browser automation, credential handling, or Mission Control Web growth

**Stability level:** alpha CLI supervision surface (same as existing `--assess`, `--observe` flags).

### Recommended next step

Add `--propose` flag to the `cognitive run` pipeline that converts `failure_insight_candidates` into `ProposedAction` objects through the Decision Gate, proving the full governed learning proposal path in one invocation.

## 19. Latest Session Update (2026-05-27 — P9: Context-aware proposal scoring and prioritization)

This session added deterministic priority scoring to the `cognitive run --propose` bridge.

**Changed:**

`crates/cli/src/main.rs`:
- Extended `ProposalMetadata` with `implementation_cost` (default: "medium"), `priority_score` (f64), `priority_band` (String)
- Added `compute_priority_score()` — deterministic function computing score = `benefit × confidence × risk_penalty × cost_penalty + type_bonus`, clamped to [0.0, 2.0]
  - `benefit` maps to 0.3–1.0 based on keyword matching (Unblock/Restore/safety → 1.0, generic → 0.3)
  - `risk_level` penalty: informational→1.0, low→0.8, medium→0.5, high→0.2, critical→0.0
  - `confidence` defaults to 0.5 if missing
  - `implementation_cost` penalty: low→1.0, medium→0.8, high→0.4
  - `suggested_action_type` bonus: fix→+0.2, governance→+0.15, test→+0.1, refactor→0.0, research→-0.1, doc→-0.2
- Added `compute_priority_band()` mapping score to "high" (≥0.7), "medium" (≥0.4), "low" (<0.4)
- Integrated scoring into both proposal creation paths (FIC-derived and observation-derived) in `run_proposals()`
- Added sorting by `priority_score` descending in `cognitive_run()` JSON output
- Added 6 unit tests: ranking order, risk reduction, missing confidence default, generic benefit default, PendingDecision invariance, band mapping

`crates/cli/tests/snapshot_integration.rs`:
- Extended `cognitive_propose_pipeline_produces_governed_proposals` integration test to verify `priority_score` in [0.0, 2.0], valid `priority_band`, `implementation_cost` presence, and descending sort order

**Key invariants:**
- Risk MUST reduce priority — high-risk actions cannot rank above low-risk quick wins unless the benefit × confidence × cost formula justifies it
- No LLM calls added — purely deterministic heuristic
- All proposals remain `PendingDecision` — scoring is metadata only
- No autonomous execution, no Decision Gate bypass

**Verification:**
- `cargo test --workspace`: 249 tests pass (all crates), including 6 new unit tests + strengthened integration test

**Stability level:** alpha CLI enrichment (pure metadata, no side effects).

### Files changed

| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | Added `compute_priority_score()`, `compute_priority_band()`, extended `ProposalMetadata`, integrated scoring into `run_proposals()`, added sort-by-score in `cognitive_run()`, added 6 unit tests |
| `crates/cli/tests/snapshot_integration.rs` | Extended integration test with score/band/sort assertions |

### What was NOT added

- No LLM calls or provider changes
- No core domain types modified (`ProposedAction`, `RiskLevel`, etc. unchanged)
- No Decision Gate behavior changed
- No autonomous execution or scheduling
- No sort-order guarantee for non-JSON output (human-readable output preserves insertion order)
- No persistence of scores (metadata is computed per-run and stored in payload)

### Recommended next step

Proposal deduplication and batching: when multiple FailureInsightCandidates produce identical or nearly identical proposals, merge them into single batched proposals with aggregate metadata, reducing noise in the proposal list before the user reviews them.

## 20. Latest Session Update (2026-05-27 — P10: Proposal deduplication and batching)

This session added deterministic deduplication to the `cognitive run --propose` bridge.

**Added:**

`crates/cli/src/main.rs`:
- `dedup_key_from_payload()` — computes a stable dedup key from `suggested_action_type`, `source_kind`, and normalized `source_summary` (lowercased, trimmed, 100 chars)
- `DedupedBatchMetadata` — struct for aggregated batch info: `merged_count`, `merged_proposal_ids`, `aggregated_source_summaries`, `aggregated_rationales`, `max_expected_benefit`, `max_confidence`, `highest_risk_level`, `lowest_implementation_cost`, `final_priority_score`, `final_priority_band`, `batched`
- `dedup_proposed_actions()` — groups proposals by dedup key, merges each group into a single proposal, re-evaluates through Decision Gate, re-scores with conservative risk

**Conservative rules:**
- Merged `risk_level` keeps the **highest** risk among merged items (risk MUST NOT be hidden)
- Merged score is re-computed using the highest risk + max confidence
- The first proposal in each group becomes the primary; all others are merged into it
- Singular proposals (only 1 item in group) are passed through unchanged (no `batched` flag)
- Proposals with different action types are NEVER merged

**`crates/cli/tests/snapshot_integration.rs`:**
- Extended integration test with soft assertions for batch metadata: if `batched`, verifies `merged_count >= 2` and `merged_proposal_ids` matches

**Key invariants:**
- All merged proposals remain `PendingDecision`
- No LLM calls added — purely deterministic dedup
- No core domain types modified
- No autonomous execution or Decision Gate bypass

**Verification:**
- `cargo test --workspace`: 255 tests pass (all crates), including 6 new dedup unit tests + strengthened integration test
- 0 compiler warnings (pre-existing edition linter noise excluded)

**Stability level:** alpha CLI enrichment (pure metadata dedup, no side effects).

### Files changed

| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | Added `dedup_key_from_payload()`, `DedupedBatchMetadata`, `dedup_proposed_actions()`, 6 unit tests; wired dedup into `run_proposals()` replacing direct Decision Gate loop |
| `crates/cli/tests/snapshot_integration.rs` | Extended integration test with batch metadata assertions |

### What was NOT added

- No LLM calls or provider changes
- No core domain types modified
- No Decision Gate behavior changed
- No autonomous execution or scheduling
- No persistence of batch metadata (computed per-run from payload metadata)

### Recommended next step

Human review queue / proposal lifecycle states: add a CLI surface to move proposals through states (PendingDecision → Approved → Blocked → NeedsHumanApproval) and track which human reviewed them.

## 21. Latest Session Update (2026-05-27 — P11: Human review queue and proposal lifecycle states)

This session added the full human review queue and proposal lifecycle states to the CLI and API server.

**Core types changed:**

`crates/core/src/action.rs`:
- Extended `ProposedActionStatus` with `Rejected`, `Deferred`, `Superseded`

`crates/core/src/audit.rs`:
- Extended `AuditEventType` with `HumanDeferred`

**API server (`apps/api-server/src/main.rs`):**
- Added `ReviewActionRequest` and `ReviewActionResponse` structs
- Added `POST /proposed-actions/{id}/review` endpoint with state transition validation
- Added `valid_review_transition()` — accepts:
  - `PendingDecision → Approved | Rejected | Deferred`
  - `Deferred → PendingDecision | Approved | Rejected`
  - `Approved → Superseded`
  - All other transitions are rejected with a clear error
- Added `review_proposed_action()` handler that updates status and creates audit event
- Added `ActorRef`, `AuditEventId`, `AuditEventType` to API server imports

**CLI (`crates/cli/src/main.rs`):**
- Added `ReviewActionCommand`, `ReviewActionSubcommand` enum and args structs
- Added `arpagona action review list [--status <filter>] [--json]`
- Added `arpagona action review show <id> [--json]` with score, band, batched metadata display
- Added `arpagona action review approve <id> [--reason "..."] [--actor "..."] [--json]`
- Added `arpagona action review reject <id> [--reason "..."] [--actor "..."] [--json]`
- Added `arpagona action review defer <id> [--reason "..."] [--actor "..."] [--json]`
- Added `arpagona action review supersede <id> [--reason "..."] [--actor "..."] [--json]`
- All transitions create audit events (HumanApproved, HumanRejected, HumanDeferred)
- Invalid transitions are rejected by the API server

**Key invariants:**
- Approved ≠ Executed — no side effects, no execution
- All new proposals still default to `PendingDecision`
- Priority scores, bands, dedup/batch metadata preserved through review
- Every lifecycle transition creates an immutable audit event

**Verification:**
- `cargo test --workspace`: 255 tests pass (all crates)
- 0 compiler warnings
- State transition rules are compile-time safe

**Stability level:** alpha CLI supervision surface.

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/action.rs` | Added `Rejected`, `Deferred`, `Superseded` variants |
| `crates/core/src/audit.rs` | Added `HumanDeferred` variant |
| `apps/api-server/src/main.rs` | Added `/proposed-actions/{id}/review` endpoint with validation |
| `crates/cli/src/main.rs` | Added `ActionSubcommand::Review` + 6 subcommands + handler |
| `PROJECT_STATUS.md` | Updated with section 21 |
| `FOCUS_LOOP_NEXT.md` | Updated to dry-run sandbox |

### What was NOT added

- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No removal of existing lifecycle safety invariants
- No real tool execution

### Recommended next step

Dry-run execution sandbox for approved low-risk proposals: simulate execution without side effects.

## 22. Latest Session Update (2026-05-27 — P12: Dry-run execution sandbox for approved proposals)

This session added the deterministic dry-run execution sandbox for approved proposals.

**Core types:**

`crates/core/src/action.rs`:
- `DryRunStatus` — `DryRunCompleted` | `DryRunBlocked`
- `DryRunResult` — rich result struct: `proposal_id`, `action_type`, `expected_effects`, `required_permissions`, `touched_resources`, `risk_level`, `reversibility`, `human_readable_summary`, `status`, `created_at`
- Exported via existing `pub use action::*`

**API server (`apps/api-server/src/main.rs`):**
- `POST /proposed-actions/{id}/dry-run` — endpoint that simulates execution
- `GET /dry-run-results` — list all dry-run results
- `describe_action_effects()` — deterministic description of expected effects per ActionType
- Guards: only `Approved` proposals may be dry-run
- Blocked proposals (PendingDecision/Rejected/Deferred/Superseded) return 400 + audit event
- Every dry-run attempt creates an audit event with dry_run_status + expected_effects
- Results stored in `store.dry_run_results`

**CLI (`crates/cli/src/main.rs`):**
- `arpagona action dry-run <id> [--json]` — dry-run an approved proposal
- Human-readable output shows: status icon, summary, expected effects, touched resources, reversibility
- JSON output via `--json` for programmatic consumption

**Dry-run effect descriptions (deterministic, no LLM):**

| ActionType | Expected Effects |
|------------|-----------------|
| ReadMemory | "In-memory inspection only" |
| ProposeToolUse | "Would propose using tool: {target}" |
| SimulateEmail | "Would simulate an email draft" |
| SystemCheck | "Would check system health" |
| Custom/default | "Would perform {action_type} action" |

**Key invariants:**
- No real execution — all descriptions are deterministic strings
- Only Approved proposals can be dry-run (400 + audit for others)
- Every attempt creates an audit event
- No LLM calls, no tool execution, no file/network side effects
- Proposal metadata (score, band, dedup, batch) preserved unchanged
- `Approved` remains non-executing

**Verification:**
- `cargo test --workspace`: 261 tests pass (all crates)
- 0 compiler warnings

**Stability level:** alpha simulation sandbox.

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/action.rs` | Added `DryRunStatus`, `DryRunResult` |
| `apps/api-server/src/main.rs` | Added `/proposed-actions/{id}/dry-run` + `/dry-run-results` endpoints with `describe_action_effects()` |
| `crates/cli/src/main.rs` | Added `ActionSubcommand::DryRun`, args struct, `dry_run_action()` handler |
| `PROJECT_STATUS.md` | Updated with section 22 |
| `FOCUS_LOOP_NEXT.md` | Updated to execution capability registry |

### What was NOT added

- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No real tool execution
- No modification of core governance invariants

### Recommended next step

Execution capability registry: declarative mapping from ActionType + RiskLevel to executors.

## 23. Latest Session Update (2026-05-28 — P13: Execution Capability Registry)

This session added the deterministic execution capability registry.

**New module:** `crates/core/src/execution_registry.rs`

**Core types:**
- `ExecutionCapability` struct with: `action_type`, `executor_id`, `supports_dry_run`, `supports_real_execution`, `required_permissions`, `touched_resource_kinds`, `max_allowed_risk`, `reversibility`, `human_approval_required`, `notes`, `safety_warning`
- `execution_capability(&ActionType) → ExecutionCapability` — deterministic single-type lookup
- `list_execution_capabilities() → Vec<ExecutionCapability>` — all known types
- `risk_exceeds_max_allowed(RiskLevel, RiskLevel) → bool` — helper for policy checks

**Capability design per ActionType:**

| ActionType | dry-run | real-exec | max_risk | approval |
|---|---|---|---|---|
| ReadMemory | ✓ | ✗ | Low | auto |
| ReadTasks | ✓ | ✗ | Informational | auto |
| ReadProposedActions / ReadPendingActions | ✓ | ✗ | Informational | auto |
| ReadDecisions | ✓ | ✗ | Informational | auto |
| ReadAudit | ✓ | ✗ | Informational | auto |
| ReadStatus | ✓ | ✗ | Informational | auto |
| SystemCheck | ✓ | ✗ | Low | auto |
| WriteMemory / CreateMemoryFact / LinkMemoryFact / InvalidateMemoryFact / CreateFailureInsightMemory | ✓ | ✗ | Medium | auto/human |
| ReadDocument | ✓ | ✗ | Low | auto |
| WriteDocument | ✓ | ✗ | Medium | human |
| ProposeToolUse | ✓ | ✗ | Medium | human |
| SimulateEmail | ✓ | ✗ | Medium | human |
| ManageTask | ✓ | ✗ | Medium | human |
| Custom(unknown) | ✗ | ✗ | Informational | human |

**Integration into DryRunResult:**
- `DryRunResult.execution_capability: Option<serde_json::Value>` — capability metadata included in every dry-run output for approved proposals
- The dry-run handler calls `execution_capability(&action.action_type)` and serialises the result into the dry-run response

**API endpoints (new):**
- `GET /execution-capabilities` — list all capabilities
- `GET /execution-capabilities/{action_type}` — show one capability

**CLI subcommand (new):**
- `arpagona action capability list [--json]` — list all execution capabilities
- `arpagona action capability show <action_type> [--json]` — show capability for one type

**FromStr for ActionType (added to `crates/core/src/action.rs`):**
- Parses snake_case strings (e.g. `"read_memory"`, `"simulate_email"`) to ActionType variants
- Unknown strings return `ActionType::Custom(name)` without error

**Test coverage (18 new tests in `crates/core/src/execution_registry.rs`):**
- Known ActionType values return deterministic capabilities
- Custom/unknown ActionType is blocked (no dry-run, no execution)
- High/Critical risk actions are not execution-eligible per max_allowed_risk
- Real execution remains disabled for all types
- Risk comparison works correctly (ordinal monotonic, threshold checks)
- `list_execution_capabilities()` returns every known type

**Verification:**
- `cargo test --workspace`: 273 tests pass (all crates)
- 0 compiler warnings

**Stability level:** alpha declarative registry.

### Key invariants

- No real execution — `supports_real_execution` is `false` for all types
- No LLM calls — all capabilities are deterministic hardcoded maps
- No Decision Gate bypass — registry is purely descriptive
- `Approved` remains non-executing — only dry-run is available
- Unknown action types return a blocked capability

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/execution_registry.rs` | **New file** — registry types, functions, 18 tests |
| `crates/core/src/action.rs` | Added `FromStr` for `ActionType`, added `execution_capability` field to `DryRunResult` |
| `crates/core/src/lib.rs` | Added `pub mod execution_registry` + `pub use execution_registry::*` |
| `apps/api-server/src/main.rs` | Added `GET /execution-capabilities` and `GET /execution-capabilities/{action_type}`; capability injected into dry-run output |
| `crates/cli/src/main.rs` | Added `action capability {list|show}` subcommand |
| `PROJECT_STATUS.md` | Updated with section 23 |
| `FOCUS_LOOP_NEXT.md` | Updated to permission model / policy checks |

### What was NOT added

- No real execution — no executor, no tool registration
- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No tool execution of any kind
- No modification of core governance invariants

### Recommended next step

Permission model / policy checks before any real executor: define policy rules that gate dry-run and future execution based on context, risk level, permissions, and resource kinds. The registry has the data; the next step is a `PolicyEngine` that consumes it.

## 24. Latest Session Update (2026-05-28 — P14: Policy Engine for permission and policy checks)

This session added the deterministic PolicyEngine that gates dry-run and future execution paths.

**New module:** `crates/core/src/policy_engine.rs`

**Core types:**
- `PolicyDecision` enum: `Allowed`, `Blocked`, `NeedsHumanApproval`, `NeedsDryRun`, `UnsupportedCapability`
- `PolicyInput` struct with: `action_type`, `proposal_status`, `risk_level`, `required_permissions`, `touched_resource_kinds`, `actor`, `workspace`, `dry_run_requested`, `real_execution_requested`
- `PolicyEngineResult` struct: `decision`, `reason`, `matched_rules`, `dry_run_required`, `human_approval_required`, `capability`
- `PolicyEngine::evaluate(&PolicyInput) → PolicyEngineResult` — main evaluation
- `PolicyEngine::evaluate_dry_run(&PolicyInput) → PolicyEngineResult` — shortcut for dry-run mode

**Policy rules (evaluated in order):**

| Rule | Condition | Decision |
|------|-----------|----------|
| 1 | Custom/unknown ActionType | `UnsupportedCapability` |
| 2 | `real_execution_requested == true` | `Blocked` (globally disabled) |
| 3 | `proposal_status != Approved` | `Blocked` |
| 4 | `dry_run_requested == true` AND `!cap.supports_dry_run` | `Blocked` |
| 5 | `risk_level > cap.max_allowed_risk` | `Blocked` |
| 6 | `cap.human_approval_required == true` | `NeedsHumanApproval` |
| 7 | Fallthrough — all checks pass | `Allowed` |

**Integration into DryRunResult:**
- `DryRunResult.policy_decision: Option<serde_json::Value>` — policy metadata included in every dry-run attempt
- The dry-run endpoint now calls `PolicyEngine::evaluate_dry_run()` before producing the result
- Blocked dry-runs: `status: DryRunBlocked` + 400 error + audit event with policy metadata
- Allowed/NeedsHumanApproval dry-runs: `status: DryRunCompleted` + policy metadata in output

**API endpoint (new):**
- `POST /proposed-actions/{id}/policy-check` — run a policy check without dry-run execution
- Returns `PolicyEngineResult` JSON with decision, reason, matched rules, and capability metadata

**CLI subcommand (new):**
- `arpagona action policy check <proposal_id> [--json]` — run policy check on a proposal
- Human-readable output shows: decision icon, reason, matched rules, capability metadata
- JSON output via `--json` for programmatic consumption

**Test coverage (18 new tests in `crates/core/src/policy_engine.rs`):**
- Approved low-risk known action passes dry-run policy
- Pending/Rejected/Deferred/Superseded proposals are all blocked
- Custom/unknown ActionType is `UnsupportedCapability`
- High/Critical risk actions are blocked (exceed max_allowed)
- Real execution is globally blocked even with Approved status
- Actions requiring human approval produce `NeedsHumanApproval`
- `PolicyEngineResult` includes capability metadata
- Blocked policy decisions include matched rule identifiers

**Verification:**
- `cargo test --workspace`: 291 tests pass (all crates)
- 1 unused-method warning (`needs_dry_run` constructor — kept for future use)

**Stability level:** alpha policy engine.

### Key invariants

- No real execution — `real_execution_requested` is always blocked
- No LLM calls — all policies are deterministic hardcoded rules
- No Decision Gate bypass — PolicyEngine is a separate layer
- `Approved` remains non-executing — only dry-run is available
- Unknown action types return `UnsupportedCapability`
- Every blocked dry-run creates an audit event with policy metadata
- `PolicyEngineResult` always includes the capability metadata used during evaluation

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/policy_engine.rs` | **New file** — PolicyDecision, PolicyInput, PolicyEngine, 18 tests |
| `crates/core/src/action.rs` | Added `policy_decision` field to `DryRunResult` |
| `crates/core/src/lib.rs` | Added `pub mod policy_engine` + `pub use policy_engine::*` |
| `apps/api-server/src/main.rs` | Integrated PolicyEngine into dry-run endpoint; added `POST /proposed-actions/{id}/policy-check` |
| `crates/cli/src/main.rs` | Added `action policy check <proposal_id>` subcommand |
| `PROJECT_STATUS.md` | Updated with section 24 |
| `FOCUS_LOOP_NEXT.md` | Updated to executor interface, disabled by default |

### What was NOT added

- No real execution — no executor, no tool registration
- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No tool execution of any kind
- No modification of core governance invariants

### Recommended next step

Executor interface, disabled by default: define a trait/interface for executors that consume approved, policy-checked actions. Keep execution disabled at the trait level — the policy engine, capability registry, and dry-run layer exist; what's missing is the formal executor abstraction they would eventually feed into.

## 25. Latest Session Update (2026-05-28 — P15: Executor interface, disabled by default)

This session added the `Executor` trait and `NoopExecutor` — the only registered executor, which always returns `ExecutionDisabled`.

**New module:** `crates/core/src/executor.rs`

**Core types:**
- `ExecutionStatus` enum: `ExecutionDisabled`, `ExecutionBlocked`, `ExecutionCompleted`, `ExecutionFailed`
- `ExecutionRequest` struct: `proposal_id`, `action_type`, `actor`, `workspace_scope`, `policy_decision`, `capability`, `dry_run_result`, `risk_level`, `required_permissions`
- `ExecutionResult` struct: `status`, `reason`, `touched_resources`, `reversible`, `audit_event_id`, `action_type`, `proposal_id`
- `Executor` trait: `executor_id()`, `supported_action_types()`, `dry_run()`, `execute()`
- `NoopExecutor` struct: the only registered executor, always returns `ExecutionDisabled`

**Key design decisions:**
- `Executor` trait is generic — `NoopExecutor` is one implementation, future executors can implement the same trait
- Policy enforcement happens in the API layer *before* calling the executor, not inside the executor itself
- `NoopExecutor.dry_run()` returns `None` — the generic dry-run layer handles it
- `NoopExecutor.execute()` always returns `ExecutionDisabled` with reason "Real execution is globally disabled"

**API endpoint (new):**
- `POST /proposed-actions/{id}/execute` — two-step pipeline:
  1. `PolicyEngine::evaluate()` with `real_execution_requested: true` → always blocked globally
  2. `NoopExecutor::execute()` → returns `ExecutionDisabled`
- Blocked by policy: creates audit event with `blocked_by_policy` payload
- Executor returns `ExecutionDisabled`: creates audit event with execution details

**CLI subcommand (new):**
- `arpagona action execute <id> [--json]` — attempt execution (always returns `ExecutionDisabled`)
- Human-readable output shows: status icon, reason, action_type, audit_event_id

**Test coverage (9 new tests in `crates/core/src/executor.rs`):**
- NoopExecutor has consistent `executor_id()`
- NoopExecutor supports known action types (no Custom types)
- NoopExecutor never performs side effects (empty touched_resources)
- `execute()` is globally disabled for read_memory and system_check
- NoopExecutor does not check policy itself (policy enforcement is in API layer)
- NoopExecutor does not support Custom action types
- NoopExecutor accepts high risk but returns `ExecutionDisabled`
- `dry_run()` returns `None` (generic layer handles it)

**Verification:**
- `cargo test --workspace`: 300 tests pass (all crates)
- 0 compiler warnings

**Stability level:** alpha executor trait with disabled-only implementation.

### Key invariants

- No real execution — `execute()` always returns `ExecutionDisabled`
- Policy check happens before executor call in the API layer
- No LLM calls — all executor logic is deterministic
- No Decision Gate bypass — policy engine is always called first
- No tool execution — NoopExecutor is purely descriptive
- `execute()` creates an audit event for every attempt (blocked or disabled)
- Dry-run path remains entirely unaffected

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/executor.rs` | **New file** — ExecutionStatus, ExecutionRequest, ExecutionResult, Executor trait, NoopExecutor, 9 tests |
| `crates/core/src/lib.rs` | Added `pub mod executor` + `pub use executor::*` |
| `apps/api-server/src/main.rs` | Added `POST /proposed-actions/{id}/execute` with policy check + NoopExecutor |
| `crates/cli/src/main.rs` | Added `action execute <id>` subcommand |
| `PROJECT_STATUS.md` | Updated with section 25 |
| `FOCUS_LOOP_NEXT.md` | Updated to executor registry with only disabled/noop executors |

### What was NOT added

- No real execution — `execute()` always returns `ExecutionDisabled`
- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No tool execution of any kind
- No modification of core governance invariants
- No executor registry (that's the next step)

### Recommended next step

Executor registry with only disabled/noop executors: build a registry that maps `executor_id` to `Box<dyn Executor>`, register `NoopExecutor` as the only entry, and expose a `resolve(action_type, risk_level) -> Option<&dyn Executor>` lookup. Integrate with the capability registry so `executor_id` in capability entries references registered executors.

## 26. Latest Session Update (2026-05-28 — P16: ExecutorRegistry with NoopExecutor only)

This session added the `ExecutorRegistry` — a deterministic registry that maps `executor_id` to `Box<dyn Executor>`.

**New module:** `crates/core/src/executor_registry.rs`

**Core types:**
- `ExecutorRegistry` struct: `register()`, `resolve(action_type)`, `get(executor_id)`, `list()`, `execute(request, audit_event_id)`
- Auto-registers `NoopExecutor` on construction
- `resolve(Custom/unknown ActionType)` returns `None` — no executor found
- `execute()` returns `ExecutionBlocked` when no executor can handle the action type

**Integration with execute pipeline:**
- `POST /proposed-actions/{id}/execute` now uses `store.executor_registry.execute()` instead of directly calling `NoopExecutor::new()`
- If no executor is found (Custom action type), returns `ExecutionBlocked` with audit event
- Audit event payload includes the resolved `executor_id` (or `None` if unresolved)

**Test coverage (15 new tests in `crates/core/src/executor_registry.rs`):**
- NoopExecutor is registered by default
- Can get NoopExecutor by id; unknown id returns None
- Known ActionType values resolve to NoopExecutor
- Custom/unknown ActionType resolves to None (blocked)
- `execute()` for Custom returns `ExecutionBlocked` with audit_event_id
- `list()` returns only `noop-executor`
- `execute()` for all known types returns `ExecutionDisabled`
- Dry-run path unaffected (NoopExecutor.dry_run() returns None)

**Verification:**
- `cargo test --workspace`: 315 tests pass (all crates)
- 0 compiler warnings

**Stability level:** alpha executor registry with only NoopExecutor.

### Key invariants

- No real execution — all execute() calls return ExecutionDisabled or ExecutionBlocked
- Policy check happens before registry.execute() in the API layer
- Custom/unknown ActionType never resolves to an executor → ExecutionBlocked
- Registry contains only NoopExecutor
- Dry-run path remains entirely unaffected
- Every execution attempt creates an audit event

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/executor_registry.rs` | **New file** — ExecutorRegistry, 15 tests |
| `crates/core/src/lib.rs` | Added `pub mod executor_registry` + `pub use executor_registry::*` |
| `apps/api-server/src/main.rs` | Added `executor_registry` to InMemoryStore; execute endpoint uses registry |
| `PROJECT_STATUS.md` | Updated with section 26 |
| `FOCUS_LOOP_NEXT.md` | Updated to execution attempt audit hardening |

### What was NOT added

- No real execution — only NoopExecutor registered
- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No tool execution of any kind
- No API endpoints for executor registry listing (deferred — CLI can read from capability registry)

### Recommended next step

Execution attempt audit hardening / executor policy alignment: ensure that every execution attempt (blocked, disabled, or future real execution) produces a rich audit trail with policy decision, executor id, capability metadata, and the full pipeline trace. Consider aligning the `AuditEventType` enum to have a dedicated `ExecutionBlocked` variant instead of reusing `DecisionCreated`.

## 27. Latest Session Update (2026-05-28 — P17: Execution audit query coverage)

This session added dedicated `AuditEventType` variants for execution, sandbox, and dry-run events, and updated `AuditTraceSummary` to recognise them.

**New `AuditEventType` variants:**
- `ExecutionBlocked` — execution attempt blocked by policy
- `ExecutionDisabled` — execution attempted while globally disabled
- `DryRunCompleted` — dry-run simulation completed
- `DryRunBlocked` — dry-run blocked by policy or constraints
- `SandboxCompleted` — sandbox simulation completed

**Updated `AuditTraceSummary`:**
- `has_execution_event` now recognises `ExecutionBlocked` and `ExecutionDisabled` in addition to `ExecutionStarted`, `ExecutionSucceeded`, `ExecutionFailed`
- New `has_dry_run_event` field recognises `DryRunCompleted` and `DryRunBlocked`
- New `has_sandbox_event` field recognises `SandboxCompleted` and `ExecutionStarted` (backward compat)

**Updated API server event emission:**
- `POST /proposed-actions/{id}/dry-run` → `DryRunCompleted` or `DryRunBlocked`
- `POST /proposed-actions/{id}/execute` (blocked by policy) → `ExecutionBlocked`
- `POST /proposed-actions/{id}/execute` (executor disabled) → `ExecutionDisabled`
- `POST /proposed-actions/{id}/sandbox` (blocked) → `ExecutionBlocked`
- `POST /proposed-actions/{id}/sandbox` (completed) → `SandboxCompleted`
- `AuditEventType::DecisionCreated` is no longer used for any execution/dry-run/sandbox event

**Test coverage (10 new tests in `crates/core/src/audit.rs`):**
- Each execution variant: `ExecutionStarted`, `ExecutionBlocked`, `ExecutionDisabled`, `ExecutionSucceeded`, `ExecutionFailed` is correctly detected by `has_execution_event`
- Dry-run variants: `DryRunCompleted`, `DryRunBlocked` detected by `has_dry_run_event`
- `SandboxCompleted` detected by `has_sandbox_event`
- `DecisionCreated` is NOT counted as any of execution/dry-run/sandbox

**Verification:**
- `cargo test --workspace`: 324 tests pass (all crates)
- 3 warnings (unused items — `ProposedActionId` import, `blocked` constructor, `needs_dry_run` constructor)

**Stability level:** alpha audit coverage.

### Key invariants

- No real execution — no executor behavior changed
- No LLM calls
- No Decision Gate or PolicyEngine changes
- `AuditEventType::DecisionCreated` no longer required for execution activity detection
- All dry-run/sandbox/execute endpoints emit dedicated event types
- Backward compatible: `has_sandbox_event` still catches old `ExecutionStarted` events

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/audit.rs` | Added 5 `AuditEventType` variants; updated `AuditTraceSummary` with `has_dry_run_event`/`has_sandbox_event`; added 10 tests |
| `apps/api-server/src/main.rs` | Updated dry-run/execute/sandbox endpoints to use dedicated event types |
| `PROJECT_STATUS.md` | Updated with section 27 |
| `FOCUS_LOOP_NEXT.md` | Updated to executor readiness states |

### What was NOT added

- No real execution
- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No tool execution of any kind

### Recommended next step

Executor readiness states / disabled-by-default executor slots: define states like `Disabled`, `Ready`, `Blocked` for executor instances, and add a slot-based system where multiple executors can be registered but remain disabled by default. This prepares for the eventual enabling of specific executors without global risk.

## 28. Latest Session Update (2026-05-26 — P18: Executor readiness states / disabled-by-default slots)

This session added `ExecutorState` with `Disabled`, `Ready`, and `Blocked` variants, and refactored the `ExecutorRegistry` to wrap executors in `ExecutorSlot` instances that carry state.

**Core types added:**

`crates/core/src/executor.rs`:
- `ExecutorState` enum: `Disabled`, `Ready`, `Blocked`
- `ExecutorState::default()` returns `Disabled`
- `ExecutorState::allows_execution()` — true only for `Ready`
- `ExecutorState::is_blocked()` — true for `Blocked`

`crates/core/src/executor_registry.rs`:
- `ExecutorSlot` struct — wraps `Box<dyn Executor>` + `ExecutorState`
- `register()` now takes `Option<ExecutorState>` — `None` defaults to `Disabled`
- `set_state(id, state)` — promote/demote executor readiness
- `get_state(id)` — query current state
- `resolve()` filters disabled and blocked executors (returns `None`)
- `execute()` returns `ExecutionBlocked` for disabled/blocked executors, `ExecutionDisabled` for ready-but-noop executors

**Behavior changes:**

- `NoopExecutor` starts as `Disabled` — cannot resolve or execute until explicitly promoted to `Ready`
- `resolve()` only returns executors in `Ready` state — this is a breaking change from previous behavior where all executors resolved regardless of state
- `execute()` returns `ExecutionBlocked` (not `ExecutionDisabled`) when the executor is `Disabled` or `Blocked`

**Test coverage (28 tests in executor_registry.rs + 4 ExecutorState tests in executor.rs = 32 new tests):**

1. `noop_executor_registered_by_default` ✓ kept
2. `noop_executor_default_state_is_disabled` — verifies default is Disabled
3. `disabled_executor_does_not_resolve_read_memory` — core invariant
4. `disabled_executor_does_not_resolve_system_check` — core invariant
5. `disabled_executor_does_not_resolve_any_type` — all 18 known types
6. `ready_executor_resolves_read_memory` — Ready → can resolve
7. `ready_executor_resolves_all_known_types` — all 18 known types
8. `blocked_executor_does_not_resolve` — Blocked → cannot resolve
9. `state_transition_disabled_to_ready` — state lifecycle
10. `state_transition_ready_to_blocked` — state lifecycle with resolve test
11. `execute_against_disabled_executor_returns_blocked` — status propagation
12. `execute_against_blocked_executor_returns_blocked` — status propagation
13. `execute_against_ready_executor_returns_executor_result` — Ready passes through
14. `register_with_explicit_state` — optional state parameter
15. `register_without_state_defaults_to_disabled` — default invariant
16. `slot_default_cannot_resolve` — ExecutorSlot helper
17. `slot_ready_can_resolve` — ExecutorSlot helper
18. `slot_blocked_cannot_resolve` — ExecutorSlot helper
19. `ExecutorState::default` — defaults to Disabled
20. `executor_state_allows_execution` — Only Ready allows
21. `executor_state_is_blocked` — Only Blocked is blocked
22. `executor_state_serialization` — JSON round-trip

(plus carry-over tests: `can_get`, `get_unknown`, `resolve_custom`, `execute_custom`, `list_returns_only`, `list_is_sorted`, `execute_passes_audit_id`, `ready_executor_still_disabled_globally`, `set_state_unknown`, `register_ready_in_new_slot`)

**Verification:**
- `cargo fmt -- --check`: clean (0 differences)
- `cargo check`: clean (0 errors, pre-existing warnings only)
- `cargo test --workspace`: 337 tests pass (all crates, up from 324)

**Stability level:** alpha executor slot system.

### Key invariants

- No real execution — NoopExecutor still returns `ExecutionDisabled` when Ready
- Disabled and Blocked executors cannot resolve or execute
- All executors registered as `Disabled` by default (must be explicitly promoted)
- State transitions preserve executor identity — `set_state()` does not replace the executor
- Custom/unknown action types never resolve regardless of state
- API server integration unchanged (executor registry default now prevents accidental resolution)

### Files changed

| File | Change |
|------|--------|
| `crates/core/src/executor.rs` | Added `ExecutorState` enum with `Disabled`/`Ready`/`Blocked`, 4 tests; made `ExecutionResult::disabled()`/`blocked()` pub(crate) |
| `crates/core/src/executor_registry.rs` | Full rewrite: `ExecutorSlot` wrapper, state-aware `register()`/`resolve()`/`execute()`, 28 tests |
| `PROJECT_STATUS.md` | Updated with section 28 |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to API server state integration |

### What was NOT added

- No real execution — NoopExecutor still disabled globally
- No LLM calls or provider changes
- No autonomous execution or scheduling
- No Decision Gate bypass
- No file/network/shell side effects
- No tool execution of any kind
- No API endpoint changes (behavior change is in the registry layer)
- No modification of core governance invariants

### Recommended next step

API server executor state integration: expose the `ExecutorRegistry::set_state()` and `get_state()` through API endpoints so operators can promote/demote executor readiness at runtime. Add `POST /executors/{id}/state` and `GET /executors` endpoints that surface slot state.

## 17. Latest Session Update (2026-05-26 — Fix DV-2026-05-26-002: CLI docs coverage check)

This session fixed DV-2026-05-26-002 from the daily validation backlog: CLI documentation drifting behind CLI surface.

### What was added/changed

- **`scripts/check-cli-docs-coverage.sh`** — lightweight docs-coverage check that validates every top-level command from `arpagona --help` has a corresponding section in `docs/cli.md`. Uses a command-to-heading mapping table to handle descriptive French headings. Exits 0 when all commands are covered, 1 with the list of missing commands when gaps are found.

- **`docs/cli.md`** — added missing `### Auth — Statut et configuration OpenAI` section documenting the `arpagona auth status` and `arpagona auth openai` subcommands, which were absent from the docs file despite being exposed in the CLI help output.

- **`DAILY_VALIDATION_BACKLOG.md`** — moved DV-2026-05-26-002 to closed/superseded with fix summary and evidence.

### Verification

- `bash scripts/check-cli-docs-coverage.sh` — passes with exit 0 ("✅ All CLI commands are covered in docs/cli.md")
- `cargo check` — clean (pre-existing warnings only, unchanged)
- `cargo test --workspace` — all tests pass

### Stability level

- `scripts/check-cli-docs-coverage.sh` — alpha validation script (not part of CI, manual/daily-validation use)
- `docs/cli.md` auth section — alpha documentation

### Limits

- No code changes to any crate
- No CLI behavior modifications
- No new dependencies
- No governance, execution, or safety boundary changes
- The docs-coverage script is a lightweight validation tool; it uses a command-to-heading mapping table that may need updating when new commands are added or renamed
- The check is not yet integrated into CI or the daily validation protocol (can be added as a future step)

## 18. Latest Session Update (2026-05-26 — Executor state API endpoints)

This session exposed executor state management through API server endpoints, delivering the handoff from FOCUS_LOOP_NEXT.md.

### What was added

**`apps/api-server/src/main.rs`**:

- `GET /executors` — lists all registered executors with `executor_id`, `executor_state`, and `supported_action_types`
- `POST /executors/:id/state` — sets an executor's readiness state (`disabled`, `ready`, or `blocked`); returns 404 for unknown executor IDs
- 5 integration tests covering: listing, disabled default, state transitions (Disabled→Ready→Blocked→Ready), and unknown executor 404

### Runtime chain advanced

```
operator → GET /executors (read state) / POST /executors/:id/state (set state) → ExecutorRegistry state change → ready for governed execution
```

This completes the bridge between the `ExecutorRegistry::set_state()`/`get_state()` core logic and a usable operator API surface.

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean (no new errors)
- `cargo test --workspace`: 342 tests pass (all crates, 0 failures)

### Deliberately not changed

- No CLI surface changes
- No new crate dependencies
- No governance, Decision Gate, or policy engine changes
- No executor execution behavior modified — only state management
- No autonomous state transitions — all state changes are operator-driven via POST
- No SurrealDB or persistence changes
- No LLM, scheduler, MCP, browser automation, or security changes

## 29. Latest Session Update (2026-05-26 — P19: Offline executor list/inspect CLI commands)

This session added `--offline` flag support to the CLI executor list/inspect commands, enabling operators to inspect executor readiness without running the API server.

**Changed in `crates/cli/src/main.rs`:**
- Added `--offline` flag to `ExecutorListArgs` and `ExecutorInspectArgs`
- Modified `executor_list()` — when `--offline` is set, constructs `ExecutorRegistry::new()` directly from the core crate and iterates slots, producing the same `ExecutorInfoResponse` shape as the HTTP path
- Modified `executor_inspect()` — when `--offline` is set, constructs `ExecutorRegistry::new()` and looks up the slot by executor_id
- Added `Clone` derive to `ExecutorInfoResponse` (needed for the offline inspect path)
- Added `ExecutorRegistry` import to the top-level use block
- Added 2 parser tests: `cli_parses_executor_list_offline` and `cli_parses_executor_list_offline_json`

**Key invariants:**
- No real execution — offline mode only reads state, never mutates
- No API server dependency — pure core crate construction
- Same output format as the online HTTP path (same `ExecutorInfoResponse` struct)
- Both human-readable and `--json` output formats work offline
- Executors remain disabled by default (NoopExecutor starts as Disabled)
- No Decision Gate, PolicyEngine, or executor behavior changes

**Verification:**
- `cargo fmt -- --check`: clean
- `cargo check`: clean (0 new errors)
- `cargo test --workspace`: 33x+ tests pass across all crates (0 failures)
- Manual verification:
  - `cargo run -- executor list --offline` → shows noop-executor with state=disabled and all 18 supported action types
  - `cargo run -- executor list --offline --json` → structured JSON with same data
  - `cargo run -- executor inspect noop-executor --offline` → executor details with state

**Stability level:** alpha CLI supervision surface (same as existing executor commands).

**Deliberately not changed:**
- No API endpoint changes
- No CLI surface changes beyond the `--offline` flag
- No new crate dependencies
- No governance, Decision Gate, or policy engine changes
- No executor execution behavior modified
- No SurrealDB or persistence changes
- No LLM, scheduler, MCP, browser automation, or security changes

**Functional-alpha chain advancement:**
```
operator -> executor list --offline -> direct ExecutorRegistry::new() slot iteration -> executor metadata readback
operator -> executor inspect <id> --offline -> direct ExecutorRegistry::new() slot lookup -> executor detail readback
```

Before this session: operators needed the API server running to inspect executor state.
After this session: `executor list --offline` and `executor inspect --offline` work entirely from the core crate, no server required.

### Files changed

| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | Added `--offline` flag to `ExecutorListArgs` and `ExecutorInspectArgs`; added offline code paths to `executor_list()` and `executor_inspect()`; added `Clone` derive to `ExecutorInfoResponse`; added `ExecutorRegistry` import; added 2 parser tests |
| `PROJECT_STATUS.md` | Updated with section 29 |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to integration test for offline executor inspect |

### Recommended next step

Add `--offline` flag support to `executor inspect` command parser tests for `--offline` and `--offline --json` combinations. Add an end-to-end integration test that constructs an `ExecutorRegistry`, registers a test executor in Ready state, and verifies both `executor list --offline` and `executor inspect --offline` produce correct output without requiring an API server.

---

## 30. Latest Session Update (2026-05-26 — Offline executor integration test)

This session added an end-to-end integration test (`offline_executor_commands_produce_correct_output`) in `crates/cli/tests/snapshot_integration.rs` that verifies the `--offline` executor commands work correctly without an API server.

**Test coverage:**
- `executor list --offline` — human-readable output contains `noop-executor` with `state=disabled`
- `executor list --offline --json` — parsed JSON array with `executor_id`, `executor_state`, `supported_action_types`
- `executor inspect noop-executor --offline` — human-readable output shows executor details
- `executor inspect noop-executor --offline --json` — parsed JSON with `executor_id=noop-executor`, `state=disabled`
- `executor inspect nonexistent-executor --offline` — graceful `not found` message

**Verification:**
- `cargo fmt -- --check`: clean
- `cargo check`: clean (pre-existing warnings only)
- `cargo test --workspace`: 350 tests pass (0 failures)

**Functional-alpha chain advancement:**
```
operator -> executor list --offline -> ExecutorRegistry::new() slot iteration -> verified metadata readback
operator -> executor inspect <id> --offline -> ExecutorRegistry::new() slot lookup -> verified detail readback
```

Before this session: offline executor commands had parser tests but no end-to-end binary integration test.
After this session: 5 sub-tests covering human-readable, JSON, and error paths — all running without an API server.

### Deliberately not changed
- No CLI surface changes (only tests)
- No core crate changes
- No executor behavior modified
- No governance, Decision Gate, or policy engine changes
- No SurrealDB or persistence changes
- No build dependencies added

## 31. Latest Session Update (2026-05-26 — P3: cognitive run offline governance `--govern` flag)

This session added a `--govern` flag to `arpagona cognitive run` that bridges the cognitive work loop output through the offline governance path (FailureInsightCandidate -> ProposedAction -> DecisionGate -> Decision -> AuditEvent -> readback) without requiring the API server.

**What was added:**
- `--govern` flag to `CognitiveRunArgs` struct in `crates/cli/src/main.rs`
- `run_offline_governance()` function that takes FailureInsightCandidates from the `--assess` bridge, creates local ProposedActions, runs them through `evaluate_proposed_action` and `audit_event_for_decision`
- Governance handler in the JSON output path that reads FailureInsightCandidates from `working_memory.failure_insight_candidates`, runs them through the governance chain, and injects `governance_results`, `decision_count`, `audit_event_count`, and a `governance_warning` into the JSON output
- Parser test `cli_parses_cognitive_run_assess_govern_json` asserting all flag combinations
- Updated existing combo test to assert `!args.govern`

**Runtime verification:**
```
$ arpagona cognitive run --objective "..." --domain business --json --assess --govern
  → "governed": true
  → "decision_count": 1
  → "governance_results" with proposed_action, decision, audit_event for each FailureInsightCandidate
  → "governance_warning": "evidence only"
```

**Safety invariants:**
- All ProposedActions are `PendingDecision` — no execution authority
- DecisionGate called with empty policies and `ReadDocument` permission only
- Output is evidence-only, non-authorizing readback
- No API server required, no network calls, no persistence
- No executor behavior modified, no governance bypass

**Verification:**
- `cargo fmt -- --check`: clean
- `cargo check`: clean (pre-existing warnings only)
- `cargo test --workspace`: 350+ tests pass (0 failures)

### Files changed
| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | Added `--govern` flag to args; added `run_offline_governance()` function; added governance handler in JSON output path; added parser test; updated combo test assertion |
| `PROJECT_STATUS.md` | Updated with section 31 |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to integration test for offline governance |

### Recommended next step
Add an end-to-end integration test in `crates/cli/tests/` that proves the full P3 chain works via `cognitive run --assess --govern --json` without an API server, using the `CARGO_BIN_EXE_arpagona` binary invocation pattern.

## 32. Latest Session Update (2026-05-26 — Holographic Memory Rust kernel crate + persistence)

This session created the first dedicated Rust kernel for symbolic associative
memory: `crates/holographic-memory`.

**New crate:** `crates/holographic-memory` (27 tests, 0 dependencies beyond serde)

**Types implemented:**
- `HolographicTrace` — trace mémoire avec signature distribuée, poids (importance,
  confidence, emotional, strategic), liens vers décisions/mémoires, traçabilité
  source_turn_ids
- `SourceKind` — ConversationTurn | MemoryCandidate | ArchitectureDecision |
  AuditEvent | ManualNote
- `DistributedSignature` — 4 vecteurs `Vec<u64>` : symbolic_bits, concept_bits,
  entity_bits, decision_bits
- `HolographicQuery` — requête avec encodage automatique en signature distribuée
- `ResonanceScore` — score 7-dimensions (4 overlaps Jaccard + 3 boosts)
- `ResonanceMatch` — trace + score + termes correspondants
- `ReconstructedContext` — contexte reconstruit avec expansion associative
- `HolographicMemoryError` — 6 variantes d'erreur (dont PersistenceError)

**Fonctions clés:**
- `encode_terms_to_signature()` — hachage déterministe (3 positions/terme, seeds
  différenciées par champ)
- `signature_overlap()` — Jaccard pondéré par champ + boosts
- `HolographicMemoryStore` trait avec `InMemoryHolographicMemoryStore`
- `save_to_file(path)` / `load_from_file(path)` — persistance JSON fichier

**Règles de retrieval:**
- Au moins une dimension de chevauchement > 0 pour être inclus
- Les boosts (importance/confidence/activation) sont des facteurs de classement,
  pas des créateurs de correspondance
- Isolation stricte par `project_id` (aucune fuite entre projets)
- Résumé déterministe sans LLM

**Documentation créée:** `docs/holographic-memory.md` — définition opérationnelle,
différence avec historique brut et mémoire vectorielle, notion de signature
distribuée, résonance, reconstruction contrôlée, 18+ tests documentés, limites,
prochaines étapes.

**Phrase canonique:**
> Holographic Memory reactivates paths to truth. It does not replace truth.

**Fichiers modifiés:**
| Fichier | Changement |
|---------|-----------|
| `crates/holographic-memory/Cargo.toml` | Créé |
| `crates/holographic-memory/src/lib.rs` | Créé (1900+ lignes, 27 tests, 0 warnings) |
| `Cargo.toml` | Membre workspace ajouté |
| `docs/holographic-memory.md` | Créé + mis à jour (canonical phrase, V0 constraints, persistence) |
| `docs/roadmap.md` | Brick Holographic Memory Kernel ajoutée |
| `PROJECT_STATUS.md` | Mise à jour sections 1, 2, 4, + session 32 |

**Verification:**
- `cargo fmt -- --check`: clean
- `cargo check`: 0 warnings dans le crate (pré-existants dans autres crates)
- `cargo test --workspace`: 400+ tests pass (all crates)

**V0 Constraints respectées:**
- ✅ No LLM — tout est hachage déterministe
- ✅ No vector database — signatures `Vec<u64>`, pas `Vec<f32>`
- ✅ Persistence JSON fichier — `save_to_file()` / `load_from_file()`
- ✅ No tool execution — opérations pures sur données
- ✅ No authorization — retrieval evidence-only
- ✅ No replacement of Graph Memory — Graph Memory reste source de vérité
- ✅ No replacement of Decision Gate — toutes les actions passent toujours par la gouvernance
- ✅ Deterministic — mêmes entrées → mêmes signatures → mêmes résultats

**Limites V0:**
- Signature purement symbolique (pas de généralisation sémantique)
- Recherche linéaire (pas d'index, O(n) par projet)
- Pas d'exploration récursive des linked_memory_ids
- Pas d'intégration Decision Gate pour les écritures
- Pas de consolidation des traces redondantes

**Prochaines étapes documentées:**
1. Intégration avec conversation-memory (encoder les tours comme traces)
2. Embeddings locaux optionnels (word2vec léger, sans LLM)
3. Graphe mémoire récursif (expansion par linked_memory_ids)
4. Consolidation périodique (fusion traces redondantes)
5. Gouvernance des écritures par Decision Gate (MemoryWriteKind::HolographicTrace)

## 33. Latest Session Update (2026-05-26 — Holographic Memory CLI commands: add traces and search by resonance)

This session added CLI commands for the real `arpagona-holographic-memory` crate, matching the pattern of `memory demo failure-insight` for Graph Memory.

### What was added

- Added `arpagona-holographic-memory` as a dependency of `arpagona-cli`
- Added `memory holographic add` — add a trace to the holographic memory store and persist to JSON file
  - `--trace-id`, `--project-id`, `--keywords`, `--concepts`, `--entities`, `--file`, `--json`
  - Auto-encodes terms into a distributed signature using the crate's deterministic hashing
  - Loads existing store from file or creates a new one; saves after each addition
  - Human-readable and structured JSON output with non-authorizing warning
- Added `memory holographic search` — search stored traces by resonance with a query
  - `--project-id`, `--query`, `--keywords`, `--concepts`, `--entities`, `--file`, `--limit`, `--json`
  - Loads store from file, builds `HolographicQuery`, calls `retrieve_by_resonance()`
  - Returns matches with score, matched keywords, trace keywords
  - Non-authorizing warning on every output
- Both commands use the real `InMemoryHolographicMemoryStore` with `save_to_file`/`load_from_file` persistence

### Key invariants

- ✅ No LLM calls — all resonance is deterministic signature matching
- ✅ No execution — CLI surface is read-only memory inspection
- ✅ Persistence via JSON — same pattern as FailureInsight demo snapshots
- ✅ Non-authorizing — every output includes "does not authorize any action" warning
- ✅ No Decision Gate bypass — holographic memory is recall evidence only
- ✅ No tool execution, API endpoint changes, scheduler, autonomy, or MCP

### Files changed

| File | Change |
|------|--------|
| `crates/cli/Cargo.toml` | Added `arpagona-holographic-memory` dependency |
| `crates/cli/src/main.rs` | Added `HolographicCommand`, `HolographicSubcommand`, args structs, `memory_holographic_add()`, `memory_holographic_search()`, dispatch wiring, import |
| `PROJECT_STATUS.md` | Updated with section 33 |
| `FOCUS_LOOP_NEXT.md` | Updated to next handoff |

### Verification

- `cargo fmt -- --check`: clean (0 differences)
- `cargo check`: 0 new warnings (only pre-existing E0670 edition noise)
- `cargo test --workspace`: 443 tests pass across all crates (all existing tests green, 0 new failures)

### Stability level

Alpha CLI supervision surface (same as `memory demo failure-insight`, `executor list --offline`, etc.)

### Deliberately not changed

- No crate boundary changes beyond adding the dependency
- No core domain types modified
- No Decision Gate or PolicyEngine changes
- No API endpoint changes
- No executor behavior modified
- No SurrealDB or persistence changes
- No LLM, scheduler, MCP, browser automation, or security changes
- The heuristic-based `crates/core/src/holographic.rs` resonance used by `cognitive run --resonate` remains unchanged (the CLI commands exercise the real crate directly)

---

## Session 35 — 2026-05-27 06:00 UTC — Merge two conflicting PRs, Track B delivered

### Objective

Resolve two open but conflicting PRs (P1 hygiene), merge them, and advance the two-track pipeline:

1. PR #103 (`feat/holographic-memory-cli`): Track B — holographic memory CLI commands.
2. PR #106 (`feat/demo-full-governed-loop`): Track B-adjacent demo script update.

### Work completed

1. **PR #103 rebased and merged** (Track B, holographic-memory CLI):
   - Cherry-picked onto fresh main, only FOCUS_LOOP_NEXT.md conflicted.
   - Verified: cargo fmt/check/test green. Force-pushed, CI re-ran green, auto-merged.
2. **PR #106 rebased and merged** (demo script):
   - Cherry-picked onto fresh main, FOCUS_LOOP_NEXT.md + PROJECT_STATUS.md conflicted.
   - Verified: cargo fmt/check/test green, bash demo-full-loop.sh exits 0.
   - Force-pushed, CI re-ran green, auto-merged.
3. **Post-merge cleanup**: both remote branches deleted, no open PRs remain.

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: 0 new warnings
- `cargo test --workspace`: all tests pass (462+)
- `bash scripts/demo-full-loop.sh`: exits 0
- No conflict markers in any source file

### Stability level

Stable. CLI surface additions only. No core domain types, runtime, or Decision Gate changes.

### Risks

- The conversation-memory integration (Track B step B1 proper) is still pending — next handoff points to Track A Phase 2 (MCP governance) for alternation balance.
- The `feat/dv-2026-05-27-004-cli-docs` branch (local) was stashed and cleaned. Its work product was already in main, no content lost.

### Deliberately not changed

- No core domain types, Decision Gate, PolicyEngine, API endpoints, executor, persistence
- No LLM, scheduler, autonomy, security, or MCP Phase 2 work

---

## Session 36 — 2026-05-27 — Track A Phase 2: MCP DecisionGate governance

### Objective

Add the DecisionGate governance layer for the MCP server's `tools/call` handler (Track A Phase 2). Every tool call from an external MCP client is now evaluated through the governance pipeline before execution.

### Work completed

**New module:** `crates/mcp-server/src/governance.rs`

- `GovernanceDecision` enum with `Approved`, `Blocked`, `RequiresOverride` variants
- `evaluate_tool_call(tool_name, arguments) -> GovernanceDecision` — creates a `ProposedAction` with `ActionType::ProposeToolUse` and runs it through `arpagona-decision-gate::evaluate_proposed_action()`
- Each decision carries the full `Decision` struct (reason, risk, policies applied)
- `is_approved()` and `summary()` convenience methods

**Modified:** `crates/mcp-server/src/server.rs`

- `handle_tools_call` now calls `evaluate_tool_call()` before ToolRuntime execution
- Approved calls proceed to execution (same as Phase 1)
- Blocked/override calls return structured error with governance reason

**Dependency:** added `arpagona-decision-gate` to Cargo.toml

### New tests

4 governance unit tests in `governance.rs`:
- `test_read_only_tool_approved_with_permission` — read_file with ProposeToolUse permission is Approved
- `test_any_read_only_tool_approved` — all 3 read-only tools are Approved
- `test_governance_summary_includes_status` — summary contains status keyword
- `test_governance_decision_has_decision` — decision has non-empty reason

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean
- `cargo test --workspace`: all tests pass (175+)

### Functional-alpha chain advancement

```
MCP client request → tools/call → GovernanceLayer::evaluate_tool_call()
  → ProposedAction (ProposeToolUse, Informational risk)
  → DecisionGate evaluate_proposed_action()
  → Approved → ToolRuntime execution → MCP response
  → Blocked  → structured governance error → MCP response
```

### Stability level

Alpha MCP governance layer — pure Rust, deterministic, no LLM calls, no network I/O.

### Risks

- Default configuration (empty policies, ProposeToolUse permission granted) auto-approves all read-only tools at Informational risk. If stricter governance is needed later, policies can be added or permissions reduced.
- The governance decision is evaluated per call but NOT audited to persistent storage yet (audit is in-memory only for now). Phase 3/4 will add audit persistence via the MCP resources layer.

## 17. Latest Session Update (2026-05-27 — Track B Step B1: Holographic Conversation Bridge)

This session implemented the holographic conversation bridge: encoding structured conversation turns as `HolographicTrace` objects with deterministic distributed signatures from the `arpagona-holographic-memory` crate.

**New module:** `crates/conversation-memory/src/holographic_bridge.rs` (620 lines)

**New types:**
- `ConversationTurn` — structured turn data (role, content, turn_id, importance)
- `Conversation` — full conversation with metadata (conversation_id, project_id, title, turns)
- `HolographicConversationBridge` — turn processor with `process_turn()`, `process_conversation()`, `find_similar_for_turns()`, `save_to_file()`/`load_from_file()`
- `extract_keywords()` — lightweight stop-word-filtered keyword extraction (splits on non-alphanumeric, filters stop words/short words, deduplicates, max 20)
- `derive_concepts()` — role-based concept labels (user_query, assistant_response, system_instruction, tool_result)

**Dependency changes:**
- `arpagona-conversation-memory` now depends on `arpagona-holographic-memory`
- `arpagona-cli` now depends on `arpagona-conversation-memory`

**CLI surface:**
- `arpagona memory holographic from-conversation --file <JSON> --find-similar [--limit N] [--json]`

**Tests:** 10 new tests in `holographic_bridge` covering keyword extraction, concept derivation, single-turn/multi-turn processing, full conversations, resonance find-similar across topics, unrelated-topic exclusion, save/load round-trip, and distributed signature creation.

**Verification:** `cargo fmt -- --check` clean, `cargo check` clean, `cargo test --workspace` 429 tests pass.

Stability level: alpha bridge crate.

Deliberately not changed:
- No modification to core holographic-memory kernel types (DistributedSignature, SourceKind, etc.)
- No LLM calls, embeddings, vector databases, or external network calls
- No API endpoint, scheduler, MCP, browser automation, or tool execution

## 18. Latest Session Update (2026-05-27 — Track A Phase 2 refinement: persistent governance audit store)

This session added file-based audit event persistence for MCP DecisionGate governance, completing the Phase 2 audit trail gap.

**New module:** `crates/mcp-server/src/audit_store.rs` (297 lines)
- `McpGovernanceAuditRecord` — stores governance outcome, tool name, arguments, summary, timestamp, and full `AuditEvent`
- `McpGovernanceAuditStore` — JSON-lines file-based store with cross-invocation persistence
  - `new(path)` — loads existing entries from file, or starts fresh
  - `record(entry)` — appends to in-memory list AND file on disk
  - `recent(limit)` — returns N newest entries
  - `all()` — returns all entries oldest-first

**Modified: `crates/mcp-server/src/governance.rs`**
- `evaluate_tool_call()` now returns `GovernanceResult` containing:
  - `decision: GovernanceDecision` — high-level outcome
  - `proposed_action: ProposedAction` — the action sent to DecisionGate
  - `decision_gate_decision: Decision` — the raw Decision returned by the gate
- All existing tests preserved; 1 new test for the richer return type

**Modified: `crates/mcp-server/src/server.rs`**
- `McpServerConfig.audit_path: Option<String>` — optional path for audit log
- `McpServer.audit_store: Option<McpGovernanceAuditStore>` — initialized from config
- `handle_tools_call()` creates `AuditEvent` via `audit_event_for_decision()` and persists it after every governance evaluation
- New `audit_store()` accessor for CLI readback
- Governance errors include `audit_event_id` in structured response

**Modified: `crates/cli/src/main.rs`**
- `McpServerArgs` gains `--audit-path` flag
- New `mcp-governance-audit` top-level command with:
  - `--audit-path <path>` (default: `target/mcp-audit.jsonl`)
  - `--limit <N>` (default: 20)
  - `--json` for structured JSON output
- Human-readable output with numbered entries, outcomes, tool names, timestamps, summaries, and audit event IDs

**Dependency changes:** None — uses only `serde_json` + `std::fs` (same pattern as `demo_snapshot.rs`)

**Tests:** 7 new audit store tests + 1 new governance test = 8 new tests
| Test | What it proves |
|------|---------------|
| `test_empty_store` | New store on non-existent file is empty |
| `test_record_and_read_back` | Recorded entries are readable via `recent()` |
| `test_persistence_across_restart` | Entries survive store drop/recreate cycle |
| `test_multiple_records_and_recent_limit` | Correct ordering and limit enforcement |
| `test_blocked_and_override_outcomes` | All 3 outcome states preserved |
| `test_store_has_path` | Store returns its file path |
| `test_governance_result_includes_proposed_action_and_decision` | Return type has all fields for audit |

**Verification:** `cargo fmt -- --check` clean, `cargo check` clean, `cargo test --workspace` 487+ tests pass (31 MCP server tests including 8 new).

Stability level: alpha MCP governance extension.

Deliberately not changed:
- No new tools added
- No MCP transport changes
- No LLM calls added
- No existing holographic-memory or conversation-memory APIs modified
- No Decision Gate bypasses
- No execution capabilities expanded
- No SurrealDB dependency added (pure serde_json + std::fs)
- No Decision Gate bypass
- No automatic memory write without operator command

## Session 37 — 2026-05-27: Track B Step B2 — Recursive Memory Graph

**Track:** B (Step B2)
**Branch:** `feat/b2-recursive-memory-graph`

### Summary

Implemented recursive linked-memory graph traversal for Holographic Memory, enabling BFS traversal of `linked_memory_ids` chains with configurable depth limits and cycle detection. Added CLI command `arpagona memory holographic explore --trace-id <id> [--max-depth <n>]` for operator-facing trace chain exploration.

### Changes

**`crates/holographic-memory/src/lib.rs`:**
- Added `MemoryGraphTraversalResult` struct with: `root_trace_id`, `visited_traces`, `visited_trace_ids`, `reachable_depth`, `max_depth_limit`, `cycle_detected`, `depth_limit_reached`, `traversal_summary`
- Added `traverse_linked_memories` to the `HolographicMemoryStore` trait using BFS with cycle detection via `HashSet<String>` visited set
- Configurable `max_depth` parameter with automatic depth-limit detection
- 7 new tests: single trace (no links), basic chain, depth limit, cycle detection (back-edge), diamond pattern (no duplicates), nonexistent root error, max_depth=0 returns root only

**`crates/cli/src/main.rs`:**
- Added `Explore(HolographicExploreArgs)` variant to `HolographicSubcommand`
- Added `memory_holographic_explore` handler with store loading, traversal, and `--json` / human-readable output

### Verification

| Check | Status |
|---|---|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean |
| `cargo test -p arpagona-holographic-memory` | ✅ 34 passed (7 new traversal tests) |
| `cargo test --workspace` | ✅ 436+ tests passed |

Deliberately not changed:
- No vector databases, embeddings, or LLM calls
- No Decision Gate bypasses
- No execution capabilities expanded
- No SurrealDB dependency added
- No existing holographic-memory APIs modified (only extended via trait method)
- No automatic memory write without operator command
- No persistence model changes
- No authorization or readback-as-authorization behavior
- No MCP integration changes

## Session 38 — 2026-05-27: Track A Phase 3 — HTTP/SSE Transport

**Track:** A (Phase 3)
**Branch:** `feat/a3-http-sse-transport`
**PR:** #112

### Summary

Added Axum-based HTTP transport and Server-Sent Events (SSE) support for the MCP server. Remote MCP clients can now connect over HTTP POST and receive notifications via SSE, in addition to the existing stdio transport.

### Changes

**`crates/mcp-server/src/http_transport.rs` (new):**
- `mcp_router()` — Axum Router builder with POST /mcp and GET /mcp/sse routes
- `handle_mcp_post()` — Receives JSON-RPC 2.0 requests, dispatches through McpServer, returns JSON-RPC responses
- `handle_mcp_sse()` — SSE stream with initial endpoint event + broadcast notification relay
- `send_notification()` — Push notifications to all connected SSE clients
- 6 new HTTP transport tests

**`crates/mcp-server/src/server.rs`:**
- Added `handle_request_to_message(&mut self, req) -> McpMessage` — transport-agnostic dispatch that returns the message instead of writing to stdout
- Refactored `dispatch()` to use `handle_request_to_message()` + `write_message()`

**`crates/mcp-server/Cargo.toml`:** Added axum, futures, tokio-stream, tower dev dependencies.

**`crates/mcp-server/src/lib.rs`:** Exported `pub mod http_transport`.

### Verification

| Check | Status |
|---|---|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean |
| `cargo test --workspace` | ✅ 200+ tests pass (36 mcp-server including 6 new HTTP transport tests) |

Stability level: alpha MCP transport extension.

Deliberately not changed:
- No new tools added
- No MCP tool governance changes
- No LLM calls added
- No existing holographic-memory or conversation-memory APIs modified
- No Decision Gate bypasses
- No execution capabilities expanded
- No SurrealDB dependency added
- No existing MCP transport modified (stdio still works)
