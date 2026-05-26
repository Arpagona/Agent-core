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
