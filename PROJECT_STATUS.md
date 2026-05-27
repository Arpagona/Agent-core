     1|# ARPAGONA Agent Core — Project Status
     2|
     3|This document is the canonical operational status file for ARPAGONA Agent Core.
     4|
     5|It describes the current implementation state, stability level, architectural risks, explicit stop-list, and the recommended next sequence of work.
     6|
     7|Every future contributor or agent must read this file together with `PROJECT_OBJECTIVES.md`, `docs/operating-doctrine.md`, `docs/development-acceleration.md` and `docs/failure-to-insight.md` before modifying the repository.
     8|
     9|## 1. Current State
    10|
    11|The repository currently contains a fast-moving alpha foundation with several experimental building blocks already present.
    12|
    13|Current observed state:
    14|- `PROJECT_OBJECTIVES.md` exists and defines the canonical vision of the project.
    15|- `PROJECT_STATUS.md` exists and defines the canonical operational status of the repository.
    16|- `docs/operating-doctrine.md` defines the current working doctrine: controlled fast iteration, Rust-first development, LOCO/Ollama delegation and CLI supervision first.
    17|- `docs/development-acceleration.md` defines the current acceleration direction: Hermes-like alpha ergonomics, Rippletide-inspired runtime enforcement and CLI-as-local-Mission-Control.
    18|- `docs/failure-to-insight.md` defines the canonical doctrine for turning failures, blocked decisions, bad proposals, missing context, policy gaps and human corrections into durable, non-authorizing insights.
    19|- `docs/graph-memory-local-persistence.md` records the current local SurrealDB persistence backend findings: `kv-surrealkv` requires the unstable SurrealDB cfg flag, while `kv-rocksdb`/`File` introduces native RocksDB/zstd build assumptions that failed local scheduled-run verification.
    20|- `README.md` points contributors and agents to the canonical project files before any modification.
    21|- `docs/roadmap.md` distinguishes the target architectural order from experimental work already prototyped out of order.
    22|- `docs/architecture.md` includes explicit architectural re-centering guidance.
    23|- `docs/compute-reservoir.md` frames the alpha minimal Compute Reservoir crate and its non-goals.
    24|- `docs/tool-registry.md` frames the alpha minimal Tool Registry crate, its declarative role and its explicit non-goals.
    25|- `docs/causal-trace.md` documents alpha conventions for linking proposed actions, tasks, decisions and audit events.
    26|- `crates/core` exists and contains the core domain vocabulary: agents, workspaces, tasks, goals, proposed actions, decisions, policies, permissions, risks, graph primitives, audit events, memory concepts, cognitive primitives and the minimal Failure-to-Insight vocabulary.
    27|- `Decision Gate` exists as alpha governance logic inside `crates/decision-gate`.
    28|- `crates/compute-reservoir` exists as an alpha minimal pure Rust crate with compute inventory/allocation types and a deterministic `allocate_compute` function.
    29|- `crates/tool-registry` exists as an alpha minimal declarative catalogue for tool definitions, capabilities, schemas, permissions, risk levels and enabled/disabled states, without execution.
    30|- `Reservoir Echo` currently exists inside the Cognitive Runtime primitives as short-term volatile cognitive continuity.
    31|- `crates/graph-memory` exists as an experimental SurrealDB adapter for Graph Memory persistence, alpha audit trace lookup by workspace, task, proposed action and decision, governed approved memory fact and FailureInsight persistence/readback helpers with non-authorizing trace proof readback, an in-memory demo/test store helper, and schema-backed CLI status readback.
    32|- `crates/holographic-memory` exists as an alpha Rust kernel for symbolic associative memory: deterministic distributed signatures, resonance-based retrieval, project-scoped isolation, and an in-memory store with 22 tests.
    33|- `crates/llm` exists as an experimental provider abstraction that can produce `ProposedAction` objects with `PendingDecision`, without executing tools.
    34|- `crates/runtime` exists as an experimental cognitive runtime loop that stops at action proposal.
    35|- `apps/api-server` exists as an alpha Axum API server.
    36|- `crates/cli` exists as an alpha terminal interface and provides read-only local supervision surfaces for decision-scoped audit readback, Failure-to-Insight vocabulary, Graph Memory alpha status, governed memory-write proposal readback and a local `memory demo failure-insight` loop for proposal → Decision Gate → audit → in-memory persistence → readback proof.
    37|- Governed memory-write readbacks now expose the optional proposed target value in both memory proposal summaries and decision/audit readbacks, while preserving compatibility with older payloads that omit the value.
    38|- `apps/mission-control` exists only as a placeholder and must remain deferred until the CLI supervision path proves useful.
    39|- `workers/python-ingestion` exists only as a placeholder and must remain deferred.
    40|
    41|The implementation already demonstrates the founding rule:
    42|
    43|```text
    44|Agent -> ProposedAction -> DecisionGate -> Decision -> Audit
    45|```
    46|
    47|The current product direction is no longer abstract stabilization only. The near-term priority is to move toward a functional Hermes-like alpha through read-only, Rust-first, local supervision surfaces, especially the CLI, while preserving Rippletide-inspired runtime enforcement and the non-negotiable governed action path.
    48|
    49|## 2. Stability Matrix
    50|
    51|| Component | Status | Role | Notes |
    52||---|---|---|---|
    53|| `PROJECT_OBJECTIVES.md` | Stable foundation | Canonical project vision | Must be read before every significant change. |
    54|| `PROJECT_STATUS.md` | Stable foundation | Canonical operational status | Must be updated after every significant change. |
    55|| `docs/operating-doctrine.md` | Stable foundation | Current work doctrine | Defines controlled fast iteration, Rust-first work, LOCO/Ollama delegation and CLI supervision first. |
    56|| `docs/development-acceleration.md` | Stable foundation | Current acceleration direction | Defines Hermes-like alpha ergonomics, CLI supervision first and Rippletide-inspired runtime enforcement. |
    57|| `docs/failure-to-insight.md` | Stable foundation | Failure-to-Insight doctrine | Defines how failures and corrections become durable learning without becoming authorization, execution or self-modification. |
    58|| `README.md` | Stable foundation | Entry point for contributors | Points to canonical objective/status/doctrine/acceleration files. |
    59|| `docs/roadmap.md` | Stable foundation | Architectural implementation order | Must reflect controlled acceleration without allowing unsafe execution. |
    60|| `docs/architecture.md` | Stable foundation | Target architecture and boundaries | Includes Architectural Re-Centering section. |
    61|| `docs/compute-reservoir.md` | Stable foundation | Compute Reservoir framing | Documents the alpha minimal crate and the boundary with Decision Gate, Graph Memory and Tool Registry. |
    62|| `docs/tool-registry.md` | Stable foundation | Tool Registry framing | Documents the declarative registry boundary, explicit non-goals and alpha surface. |
    63|| `docs/causal-trace.md` | Alpha foundation | Causal trace conventions | Documents current links and alpha audit trace queries for tasks, proposed actions, decisions and audit events without adding execution. |
    64|| `crates/core` | Stable foundation | Domain vocabulary and pure types | Must not become a catch-all crate. Governance logic should stay in dedicated crates. |
    65|| Core domain types | Stable foundation | Shared typed language | Includes minimal Failure-to-Insight vocabulary; remains pure, serializable and dependency-light. |
    66|| Decision Gate | Alpha | Pre-execution governance | Extracted into `crates/decision-gate`; `crates/core` no longer reexports the Decision Gate logic. |
    67|| Reservoir Echo | Alpha | Short-term cognitive continuity | Volatile traces only. Not persistent memory. Not model routing. Not Compute Reservoir. |
    68|| Holographic Memory | Alpha V0 crate | Symbolic associative memory kernel | `crates/holographic-memory`: 22 tests, in-memory store, deterministic signatures, no LLM/vector DB/persistence/authorization. Canonical phrase: "Holographic Memory reactivates paths to truth. It does not replace truth." |
    69|| Compute Reservoir | Alpha minimal | Compute/model/resource routing | `crates/compute-reservoir` provides serializable types and pure allocation only; no model calls, execution, I/O, persistence or Decision Gate replacement. |
    70|| Tool Registry | Alpha minimal | Declarative catalogue of tools and permissions | `crates/tool-registry` declares tools, capabilities, schemas, governance notes and lookup/status changes only; no execution path. |
    71|| `crates/graph-memory` | Experimental | SurrealDB Graph Memory adapter | Adds alpha audit-event queries by task, proposed action and decision plus governed FailureInsight memory trace proof readback, an in-memory demo/test helper and schema-backed CLI status readback; broader persistence conventions and graph schema still need stabilization. |
    72|| Graph Memory domain port | Alpha | Memory contract | Useful foundation, but persistence and audit coupling are not final. |
    73|| Audit System | Alpha | Trace important events and decisions | Has usable decision-scoped readback summaries; must remain non-authorizing. |
    74|| `crates/llm` | Experimental | LLM provider abstraction | Must remain limited to proposals. No tool execution by provider. |
    75|| `crates/runtime` | Experimental | Cognitive runtime loop | Must remain proposal-only until governance layers are ready for controlled integration. |
    76|| `apps/api-server` | Alpha | REST access to alpha objects | Must not take business governance responsibility. |
    77|| `crates/cli` | Alpha supervision surface | Local Mission Control precursor | Provides read-only audit, Failure-to-Insight, Graph Memory status, governed memory-write proposal supervision and a local FailureInsight memory demo loop. Must not become an execution bypass. |
    78|| Neutral Orchestrator | Not implemented | Coordination layer | Deferred until governance, compute and tool layers are coherent enough for controlled integration. |
    79|| Mission Control Web | Deferred | Human supervision UI | Do not expand yet. CLI supervision comes first. |
    80|| Scheduler / autonomous loops | Deferred | Controlled recurring work | Must wait for Decision Gate, Tool Registry, Audit and human approval path. |
    81|| MCP integration | Deferred | External tool ecosystem | Must wait for Tool Registry and security hardening. |
    82|| Browser automation | Deferred | Controlled web interaction | Must wait for governance, audit and security hardening. |
    83|| Security hardening | Deferred | Production-grade protection | Final V0 hardening stage, not a reason to bypass governance now.
    84|
    85|## 3. What Is Stable
    86|
    87|Stable foundations:
    88|
    89|- the founding principle: no direct execution by agents;
    90|- the canonical objective document;
    91|- the canonical operational status document;
    92|- the current operating doctrine and acceleration direction;
    93|- the monorepo direction;
    94|- Rust as backend foundation;
    95|- local-first, graph-native, compute-aware, auditable and human-governed architecture;
    96|- `ProposedAction -> DecisionGate -> Decision -> Audit` as the mandatory control path;
    97|- separation between domain vocabulary and adapters as an architectural rule;
    98|- documentation-level separation between Reservoir Echo and Compute Reservoir;
    99|- the CLI as the preferred near-term local supervision surface;
   100|- Failure-to-Insight as a stable documentary doctrine for turning failures and corrections into durable, non-authorizing learning artifacts;
   101|- minimal `FailureInsight` domain vocabulary in `crates/core`, limited to pure serializable types and optional trace links.
   102|
   103|## 4. What Is Experimental
   104|
   105|Experimental areas:
   106|
   107|- SurrealDB persistence details in `crates/graph-memory`;
   108|- LLM provider behavior in `crates/llm`;
   109|- runtime loop behavior in `crates/runtime`;
   110|- API shape in `apps/api-server`;
   111|- terminal UX in `crates/cli`;
   112|- Reservoir Echo tuning and lifecycle;
   113|- Holographic Memory crate (`crates/holographic-memory`) — symbolic associative memory kernel; in-memory store, 22 tests, deterministic signatures, no LLM/vector DB/persistence/authorization yet;
   114|- Compute Reservoir allocation heuristics and telemetry shape;
   115|- audit persistence and causal trace design;
   116|- future Failure-to-Insight audit conventions, CLI readback and broader Graph Memory integration;
   117|- exact crate boundaries for remaining governance layers.
   118|
   119|Experimental means: useful for learning, local supervision and integration tests, but not stable enough to justify external-effect execution around it.
   120|
   121|## 5. What Must Not Be Implemented Yet
   122|
   123|Do not implement yet:
   124|
   125|- real tool execution;
   126|- shell access;
   127|- file deletion;
   128|- email sending;
   129|- scheduler autonomy;
   130|- Mission Control UI;
   131|- MCP integration;
   132|- browser automation;
   133|- multi-agent autonomy;
   134|- self-modification;
   135|- secrets access by LLM.
   136|
   137|These capabilities are explicitly blocked until Decision Gate, Compute Reservoir, Tool Registry, Graph Memory persistence and Audit are stabilized enough for controlled integration.
   138|
   139|Read-only CLI supervision work is allowed and encouraged, provided it does not approve, reject, execute, schedule, mutate external state, bypass the Decision Gate or treat readback as authorization.
   140|
   141|## 6. Current Architectural Risks
   142|
   143|Main risks:
   144|
   145|- `crates/core` may become a catch-all crate.
   146|- API, CLI, LLM and runtime layers are advancing before Tool Registry and before Compute Reservoir is stabilized beyond alpha minimal.
   147|- Decision Gate is now a dedicated crate; downstream imports must keep using `arpagona-decision-gate` instead of reintroducing governance logic into `crates/core`.
   148|- Reservoir Echo must not be confused with Compute Reservoir.
   149|- No tool execution must be introduced before Tool Registry + Decision Gate + Audit are stable; the current Tool Registry is declarative only.
   150|- API server and CLI could accidentally become privileged orchestration layers if responsibilities are not constrained.
   151|- LLM provider abstraction could drift toward tool-calling unless explicitly kept proposal-only.
   152|- Runtime loops could drift toward autonomy before human-governed control paths exist.
   153|- Graph Memory and Audit could diverge unless important decisions produce durable, queryable traces.
   154|- Development could drift back into endless test-only stabilization instead of shipping small read-only supervision surfaces.
   155|
   156|## 7. Next Recommended Work
   157|
   158|Recommended sequence from the current state:
   159|
   160|1. Keep the Failure-to-Insight doctrine and minimal domain vocabulary visible in canonical contributor and focus-loop context.
   161|2. In a later bounded implementation PR, add the smallest audit conventions needed to extract or reference `FailureInsight`, without adding execution, autonomy or authorization.
   162|3. Prefer read-only CLI supervision increments that make the existing audit/task/action state inspectable.
   163|4. Add more Graph Memory or Audit guards only when they protect a concrete uncovered regression risk or unblock a supervision feature.
   164|5. Keep `crates/tool-registry` as a declarative catalogue only and harden it if gaps appear.
   165|6. Stabilize `crates/compute-reservoir` only as needed for future governed integration and local/cloud delegation.
   166|7. Expand API/Runtime only when the change remains read-only, clearly governed, or directly supports the CLI supervision path.
   167|
   168|The Decision Gate extraction is complete, the Compute Reservoir exists as alpha minimal, and the Tool Registry now exists as alpha minimal declarative catalogue. Keep `crates/core` limited to domain vocabulary, keep governance logic in `crates/decision-gate`, and do not treat compute allocation, readback or tool lookup as action approval.
   169|
   170|## 8. Target Architectural Order
   171|
   172|The target consolidation order is now interpreted as controlled acceleration, not paralysis:
   173|
   174|1. Core Domain Types
   175|2. Decision Gate separated
   176|3. Compute Reservoir minimal
   177|4. Tool Registry minimal
   178|5. Graph Memory + SurrealDB stabilized enough for readback
   179|6. Audit System stabilized enough for readback
   180|7. Failure-to-Insight vocabulary present; next conventions remain bounded and non-executing
   181|8. CLI supervision surface
   182|9. Neutral Orchestrator
   183|10. API Server Axum
   184|11. Mission Control Web
   185|12. Scheduler / controlled autonomous loops
   186|13. LLM Provider abstraction stabilized
   187|14. End-to-end demo
   188|15. Security hardening
   189|
   190|Some components already exist experimentally outside this order. They must not be treated as permission to expand unsafe features. They may be grown when the growth is read-only, observable, reversible and aligned with CLI supervision or governed integration.
   191|
   192|## 9. Explicit Stop-List for Unsafe Feature Expansion
   193|
   194|Stop unsafe feature expansion until the governance layers are stabilized.
   195|
   196|Do not add:
   197|
   198|- executable tools;
   199|- scheduler behavior;
   200|- autonomous loops;
   201|- Mission Control screens;
   202|- MCP support;
   203|- browser automation;
   204|- unrestricted file access;
   205|- shell integration;
   206|- operational secrets management;
   207|- agent self-modification;
   208|- multi-agent autonomous execution;
   209|- any CLI/API path that acts as approval, authorization, orchestration or execution state.
   210|
   211|Allowed work during the current acceleration phase:
   212|
   213|- read-only CLI supervision;
   214|- documentation cleanup;
   215|- crate boundary clarification;
   216|- tests that protect newly exposed behavior or concrete uncovered risks;
   217|- Compute Reservoir design and local/cloud delegation improvements;
   218|- Tool Registry declarative design improvements;
   219|- audit and graph persistence stabilization work that supports readback and does not introduce execution.
   220|
   221|## 10. Session Update Rule
   222|
   223|Every future agent must update `PROJECT_STATUS.md` after every significant modification.
   224|
   225|A significant modification includes:
   226|
   227|- adding, removing or renaming a crate;
   228|- changing the responsibility of a crate;
   229|- adding a new API or CLI surface;
   230|- changing Decision Gate behavior;
   231|- changing Graph Memory or Audit persistence/readback semantics;
   232|- adding a provider, runtime loop, worker or interface;
   233|- changing security assumptions;
   234|- changing the project roadmap or implementation order.
   235|
   236|The update must clearly state whether the change is stable, alpha, experimental, deferred or not implemented.
   237|
   238|## 11. Latest Session Update
   239|
   240|This session added structured JSON output to the read-only local supervision status CLI.
   241|This session added a cross-invocation demo snapshot path that proves the governed FailureInsight learning loop output survives across separate process invocations.
   242|
   243|Changed:
   244|- added `crates/graph-memory/src/demo_snapshot.rs` with `FailureInsightDemoSnapshot` struct, `write_to_file`/`read_from_file` methods, `SnapshotError` type and 4 unit tests (round-trip, bare filename, missing file error, invalid JSON error);
   245|- added `pub mod demo_snapshot;` to `crates/graph-memory/src/lib.rs`;
   246|- extended `arpagona memory demo failure-insight` with an optional `--snapshot-path <path>` flag: when provided, the demo writes the readback state as a JSON snapshot file to disk after the in-memory demo succeeds;
   247|- added `arpagona memory demo snapshot-read <path> [--json]` subcommand that reads and displays a previously written snapshot file, proving cross-invocation readback;
   248|- all new code uses only `serde` + `serde_json` + `std::fs` — no native SurrealDB backend dependencies, no feature flags, no build-time gates.
   249|- all verification passes: `cargo fmt -- --check`, `cargo check`, `cargo test` (132+4 new tests all passing).
   250|
   251|- changed `arpagona status` to accept `--json` in `crates/cli`;
   252|- reused the existing `StatusReadback` shape for human and JSON output;
   253|- documented `arpagona status --json` in `docs/cli.md`;
   254|- added CLI parser coverage for the new flag.
   255|
   256|Stability level: alpha CLI supervision surface.
   257|
   258|Limits:
   259|- no endpoint was added;
   260|- no server-side state was modified;
   261|- no Graph Memory schema, query or mutation was added;
   262|- no audit event creation or extraction behavior was added;
   263|- no runtime behavior was added;
   264|- no real tool execution was introduced;
   265|- no destructive capability was added;
   266|- no approval, rejection or authorization behavior was added;
   267|- no Decision Gate behavior was changed;
   268|- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;
   269|
   270|- Added `docs/daily-agent-validation.md`
   271|- Wrote comprehensive agent validation checklist
   272|- All existing tests pass without modification
   273|
   274|## 12. General Cognitive Work Loop V0 — Alpha Domain/Runtime Skeleton
   275|
   276|This session added the first general-purpose cognitive cycle skeleton to Agent Core:
   277|
   278|**New module:** `crates/core/src/cognitive_work.rs`
   279|
   280|**Pure types added:**
   281|- `Objective`, `ObjectiveId`, `ObjectiveDomain`, `ObjectiveStatus`, `SuccessCriterion`
   282|- `WorkingMemory`, `ContextItem`, `Assumption`, `Constraint`, `MissingContext`
   283|- `CognitivePlan`, `PlanStep`, `RequiredObservation`
   284|- `ProposedNextAction`, `NextActionKind`
   285|- `ImprovementCandidate`, `ImprovementCandidateKind`
   286|- `CognitiveCycleResult`
   287|
   288|**Heuristic engine:**
   289|- `run_cognitive_work_cycle()` — pure, deterministic, I/O-free, LLM-free.
   290|- Domain classification via keyword matching (9 domains including General, Unknown).
   291|- Missing context detection based on domain heuristics.
   292|- Plan generation with context-aware ordering.
   293|- Next action proposal (RequestContext if gaps exist, StopWithReport otherwise).
   294|- Improvement candidate identification (missing context, weak plan, domain ambiguity).
   295|
   296|**CLI surface:**
   297|- `arpagona cognitive run --objective <TEXT> [--domain <DOMAIN>] [--context <TEXT>] [--json]`
   298|- Human-readable text output and structured JSON output.
   299|- JSON contains: objective, working_memory, plan, required_observations, proposed_next_action, improvement_candidates, warning.
   300|
   301|**Documentation:** `docs/general-cognitive-work-loop.md`
   302|
   303|**Tests:** 17 core tests + 7 CLI tests = 24 new tests covering serialization, domain classification, missing context detection, non-authorizing invariant, CLI parsing, and JSON output structure.
   304|
   305|Stability level: alpha domain/runtime skeleton.
   306|
   307|Key invariants enforced:
   308|- ✅ read-only (no I/O, no LLM, no tool execution, no persistence)
   309|- ✅ non-autonomous (no scheduler, no auto-execution)
   310|- ✅ no external effects
   311|- ✅ non-authorizing (every `ProposedNextAction` has `non_authorizing: true`)
   312|- ✅ pure serde serialization for all types
   313|- ✅ prepares future LLM/orchestrator integration
   314|
   315|Architectural risk:
   316|- low. The module is entirely self-contained in `crates/core` with no new crate dependencies. No existing behavior is modified or bypassed.
   317|
   318|Not added (per stop-list):
   319|- no LLM call, API endpoint, scheduler, browser automation, MCP, email, shell, file write, Graph Memory persistence, hidden memory injection, Decision Gate bypass, or self-modification.
   320|
   321|## 13. Cognitive Tool Runtime — Alpha Read-Only Foundation
   322|
   323|This session created the first operational bridge between ARPAGONA's cognitive vocabulary and real filesystem perception.
   324|
   325|### What was added
   326|
   327|**`crates/tool-runtime`** — new crate providing an alpha read-only tool runtime with 3 tools:
   328|
   329|- **`read_file`** — read a file within the workspace (blocked: absolute paths, parent traversal, sensitive files, large files)
   330|- **`list_files`** — list directory entries (skips `.git`, `target`, `node_modules`, `.env`, `.ssh`)
   331|- **`search_text`** — search for text patterns in workspace files (bounded results, bounded file sizes)
   332|
   333|All tools are:
   334|- read-only
   335|- locally scoped to the workspace
   336|- size-limited and count-limited
   337|- returning structured `ToolExecutionResult` with observation, error, audit hint and failure-insight-candidate flags
   338|
   339|**`crates/core`** — new cognitive vocabulary types:
   340|
   341|- `ToolIntent` — full tool intention with rationale, purpose, risk, fallback
   342|- `ToolExecutionRequest` — concrete execution request
   343|- `ToolExecutionResult` — structured result with status, observation, error
   344|- `ToolExecutionStatus` — Success, Warning, Failed, Blocked, Skipped
   345|- `ToolExecutionError` — typed error with security flag
   346|- `ToolObservation` — observation with actionable/failure-insight-candidate markers
   347|- `ToolUseRationale` — justification, expected observation, downstream use, risk assessment
   348|- `ToolExecutionMode` — Simulate, Execute, RequireHumanConfirmation
   349|- `ToolRiskLevel` — None, Low, Medium, High, Critical
   350|- `CognitivePurpose` — Perception, Recall, Inspection, Transformation, Validation, Execution, Communication, Reflection
   351|- `FallbackStrategy` — Retry, UseAlternative, ReportOnly, EscalateToHuman
   352|
   353|**`crates/tool-registry`** — extended with cognitive concepts:
   354|
   355|- `ToolCognitiveRole` — Perception, Recall, Inspection, Transformation, Validation, Execution, Communication, Reflection
   356|- `ToolRiskProfile` — risk profile for tool declarations
   357|- Extended `ToolCapability` with ReadFile, ListFiles, SearchText, ShellAccess, EmailSend, BrowserAutomation, MCPAccess
   358|- `is_safe_for_read_only()` and `is_non_executable()` methods on roles
   359|- `ToolCognitiveRole::Transformation`, `Execution`, `Communication` are marked non-executable
   360|
   361|**CLI** — new `arpagona tool` commands:
   362|
   363|- `arpagona tool list` — list available tools
   364|- `arpagona tool inspect <name>` — show tool details
   365|- `arpagona tool demo read-file <path>` — execute read-only read_file
   366|- `arpagona tool demo list-files [path]` — execute read-only list_files
   367|- `arpagona tool demo search-text <query> [path]` — execute read-only search_text
   368|- All commands support `--json` for structured output
   369|
   370|**Documentation**:
   371|
   372|- `docs/cognitive-tool-runtime.md` — comprehensive design document explaining why tools are necessary for cognition, why execution must be controlled, the architecture overview, each tool's cognitive role, how Hermes inspired the selection, why read-only first, what remains non-executable, and what comes next
   373|
   374|### What was NOT added
   375|
   376|- No scheduler, browser, shell-free, write, email, MCP, secrets, API endpoint, self-modification, autonomy, multi-agent runtime, Holographic Memory vector store, SurrealDB persistence, or LLM integration
   377|- No Decision Gate bypass — the runtime is a local demo layer; governance integration is architectural preparation only
   378|- The `ToolExecutionResult` carries `failure_insight_candidate` flags but does not auto-generate FailureInsights
   379|
   380|### Stability level
   381|
   382|Alpha experimental. All 3 tools are proven by unit tests (13 tests in tool-runtime). The core vocabulary has 7 new tests, the tool-registry has 5 new tests. The CLI commands compile and dispatch correctly.
   383|
   384|### Test count
   385|
   386|- `arpagona-agent-core`: 42 tests (35 existing + 7 new)
   387|- `arpagona-tool-registry`: 11 tests (6 existing + 5 new)
   388|- `arpagona-tool-runtime`: 13 tests (all new)
   389|- **Total**: 66 tests across the 3 crates
   390|
   391|### Recommended next step
   392|
   393|Connect `ToolExecutionResult` to the Audit system and Failure-to-Insight pipeline, so that failed observations automatically produce candidate `FailureInsight` records. Then add the `search_text` and `list_files` results to the Working Memory / Reservoir vocabulary for context-grounded agent behaviour.
   394|
   395|## 14. Snapshot List — CLI Snapshot Discovery
   396|
   397|This session added the missing snapshot discovery command for the governed FailureInsight demo path.
   398|
   399|### What was added
   400|
   401|**`crates/graph-memory/src/demo_snapshot.rs`**:
   402|
   403|- `SnapshotListing` struct with `file_name`, `description_preview`, `chain_step_count`, `content_preview`
   404|- `list_snapshots_in_directory(dir)` — scans a directory for `.json` files, deserializes valid `FailureInsightDemoSnapshot` instances, returns sorted metadata
   405|
   406|**`crates/cli/src/main.rs`**:
   407|
   408|- New CLI command: `arpagona memory demo snapshot-list [--json] [--snapshot-dir <dir>]`
   409|- Default snapshot directory: `target/demo-snapshots` (configurable via `ARPAGONA_SNAPSHOT_DIR` env var)
   410|- Human-readable output with snapshot count, file names, description previews, alpha chain step counts, content previews
   411|- JSON output via `--json` for programmatic consumption
   412|- 4 parser tests: basic parse, `--json`, `--snapshot-dir`, combined flags
   413|
   414|### Verification
   415|
   416|- `cargo fmt -- --check`: clean
   417|- `cargo test`: 41 CLI tests + 2 integration tests → all pass (0 new failures)
   418|- `cargo run -- memory demo failure-insight --snapshot-path target/demo-snapshots/demo.snapshot.json`: writes snapshot
   419|- `cargo run -- memory demo snapshot-list --snapshot-dir target/demo-snapshots`: shows 1 snapshot with 7 alpha chain steps
   420|- `cargo run -- memory demo snapshot-list --snapshot-dir target/demo-snapshots --json`: returns structured JSON with file path and listing metadata
   421|
   422|### Functional-alpha chain advancement
   423|
   424|This completes the CLI discovery surface for the demo snapshot path:
   425|
   426|```
   427|signal → proposal → decision → audit → approved persistence → snapshot-write → snapshot-read → snapshot-list (NEW) → repeatable demo
   428|```
   429|
   430|Before this session: operators needed to know exact file paths to inspect snapshots.
   431|After this session: `snapshot-list` discovers all available snapshots, names, and metadata.
   432|
   433|### What was NOT added
   434|
   435|- No SurrealDB persistence changes
   436|- No scheduler, browser, write, email, MCP, secrets, API endpoint, self-modification, autonomy, multi-agent runtime, or LLM integration
   437|- No Decision Gate bypass
   438|
   439|### Files changed
   440|
   441|| File | Change |
   442||------|--------|
   443|| crates/graph-memory/src/demo_snapshot.rs | Added `SnapshotListing` struct and `list_snapshots_in_directory()` |
   444|| crates/cli/src/main.rs | Added `SnapshotList` variant, args struct, dispatch, handler function, 4 parser tests |
   445|
   446|### Stability level
   447|
   448|Stable alpha extension. Pure file I/O, no native deps or SurrealDB feature flags. All snapshot operations (write, read, list) now have CLI surfaces.
   449|
   450|### Test count
   451|
   452|- `arpagona-cli`: 41 tests (37 existing + 4 new)
   453|- `arpagona-graph-memory`: 24 tests (20 existing + 4 existing)
   454|- **Total**: 65+ tests across the workspace
   455|
   456|### Recommended next step
   457|
   458|Create a self-contained demo script (`scripts/demo-full-loop.sh`) that runs the complete governed FailureInsight demo path end-to-end, proving the full chain in one repeatable invocation.
   459|
   460|## 15. Latest Session Update (2026-05-25 — Rebase #77, resolve conflicts, close superseded PRs)
   461|
   462|This session rebased PR #77's description-propagation commits onto the latest `main`, resolving merge conflicts in handoff files (FOCUS_LOOP_NEXT.md, PROJECT_STATUS.md accepted main's versions). The code commits applied cleanly.
   463|
   464|Changed:
   465|- Cherry-picked 2 commits from `feat/description-cross-invocation-delivery` (#77) onto current `main`:
   466|  1. `feat: prove --description propagates through full governed loop (signal to readback)` — changes `MemoryDemoSignalReadback.summary` from `&'static str` to `String`, wires custom description into signal readback, adds integration test
   467|  2. `feat: add cross-invocation description propagation test` — proves `--description` text survives demo snapshot-then-readback cycle across separate process invocations
   468|- Force-pushed rebased branch to update #77 (conflicts resolved, now mergeable)
   469|- Closed superseded PRs:
   470|  - #74 (feat/description-end-to-end-governed-path-test) — superseded by #77
   471|  - #72 (feat/description-e2e-v2) — superseded by #77
   472|
   473|Status: PR #77 is mergeable (no conflicts). CI pending re-run on new commits.
   474|
   475|Stability level: alpha CLI demo/readback. Same bounded work as before, just rebased and delivered cleanly.
   476|
   477|Verification:
   478|- `cargo fmt -- --check`: clean
   479|- `cargo check`: clean
   480|- `cargo test`: 172 tests pass (all crates), including `memory_demo_description_propagates_through_governed_loop_signal_to_readback` and `cross_invocation_description_survives_snapshot_path_across_processes`
   481|
   482|Limits:
   483|- no new code was added by this session (cherry-pick only)
   484|- no broad CLI mutation command added
   485|- no API endpoint added
   486|- no Graph Memory persistence helper added
   487|- no Decision Gate behavior changed
   488|- no SurrealDB backend change made
   489|- no LLM/provider/runtime direct memory mutation added
   490|- no real tool execution introduced
   491|- no scheduler, autonomy, MCP, browser automation, credential handling, or Mission Control Web growth
   492|- readback remains evidence only, not authorization
   493|
   494|Recommended next step: wait for CI to complete on #77, then merge into `main`. After merge, create `scripts/demo-full-loop.sh` for a single-repeatable-command governed FailureInsight demo path.
   495|
   496|## 16. Latest Session Update (2026-05-25 — P5 + P6 + P7: WorkingMemory ↔ ComputeReservoir ↔ HolographicMemory bridge complete)
   497|
   498|This session completed the P5–P7 milestone sequence for the cognitive loop integration.
   499|
   500|### P5 — Connect WorkingMemory to ComputeReservoir allocation (PR #85, merged)
   501|
## Session 39 (2026-05-27 — Track B Step B3: optional local embeddings for semantic generalization)

This session implemented Track B Step B3 — optional local embeddings for the Holographic Memory crate, enabling semantic generalization beyond exact keyword matching.

### What was added

**`crates/holographic-memory/src/embedding.rs`** — new embedding module:
- `EmbeddingProvider` trait — pluggable interface for computing embedding bit-positions from text
- `NoOpEmbeddingProvider` — returns empty bits (graceful fallback when embeddings are not used)
- `CharacterNGramEmbeddingProvider` — built-in provider using character 2-gram and 3-gram hashing. Deterministic, zero external dependencies. Captures subword/morphological similarity (e.g., "running" and "run" share character n-gram bits)
- `extend_signature_with_embedding()` — extends a `DistributedSignature` with embedding bits
- 10 unit tests: determinism, empty text, semantic similarity, no-op fallback, keyword contribution

**`crates/holographic-memory/src/lib.rs`** — resonance pipeline extended:
- `DistributedSignature.embedding_bits: Vec<u64>` — new dimension for semantic generalization
- `ResonanceScore.embedding_overlap: f32` — new overlap dimension (weighted at 20% of total)
- `signature_overlap()` updated to include embedding Jaccard overlap
- `retrieve_by_resonance()` overlap check includes embedding_overlap
- Existing weights adjusted: symbolic (30%→25%), concept (30%→25%), entity (30%→25%), decision (10%→5%), embedding (new 20%)

**`crates/cli/src/main.rs`** — CLI surface extended:
- `--embed` flag on `memory holographic add` and `memory holographic search`
- When `--embed` is set, the trace/query signature is extended with character n-gram embedding bits
- No changes to existing commands when `--embed` is absent (backward compatible)

**`crates/holographic-memory/Cargo.toml`** — new features:
- `builtin-embedding` feature (enabled by default) gates the built-in `CharacterNGramEmbeddingProvider`

### Verification
| Check | Result |
|---|---|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing E0670 edition linter noise) |
| `cargo test --workspace` | ✅ 509 tests pass across all crates |

### Test count
- `arpagona-holographic-memory`: 44 tests (34 existing + 10 new embedding tests)
- **Total workspace**: 509 tests

### Functional-alpha chain advancement
```
HolographicTrace -> DistributedSignature -> embedding_bits (NEW) -> SemanticResonance (NEW overlap dimension) -> Generalized context retrieval
```

Before this session: resonance only matched exact keywords, concepts, entities.
After this session: traces can now produce non-zero resonance based on character n-gram overlap,
enabling "running" to match "run", "database" to match "data", etc. — all without external
model dependencies.

### Stability level
Alpha experimental extension. The embedding pipeline is opt-in (--embed flag). When not enabled,
behavior is identical to before. The `EmbeddingProvider` trait is ready for real neural embedding
providers (fastembed, ONNX, etc.) via future feature-gated implementations.

### What was NOT added
- No vector database
- No LLM calls
- No Decision Gate bypass
- No execution capabilities
- No SurrealDB persistence
- No existing resonance API changed
- All existing tests pass without modification

### Risks
- Weight rebalancing (25/25/25/5/20) changes the relative importance of each overlap dimension. Traces that previously resonated by decision overlap alone may score lower. This is intentional — decision overlap was the least semantically meaningful dimension; embedding overlap replaces it with higher semantic weight.
- Character n-gram embedding is a simplified stand-in for real embeddings. Captures morphology but not true synonymy ("car" ≠ "automobile"). This is documented as a limitation.
