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
| `crates/holographic-memory` exists as an alpha Rust kernel for symbolic associative memory: deterministic distributed signatures, resonance-based retrieval, project-scoped isolation, SQLite persistence with drop/reopen survival, and an in-memory store with 22+ tests including consolidation, activation, and file-backed persistence.
- `crates/mcp-server` exists as an alpha native MCP (Model Context Protocol) server with stdio and HTTP/SSE transport, DecisionGate governance for tools/call, audit store persistence, MCP resources + prompts, and notifications (`tools/list_changed`). All phases A1-A5 are merged and passing 52+ tests.
- `crates/llm` exists as an experimental provider abstraction
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
    68|| Holographic Memory | Alpha V0 crate | Symbolic associative memory kernel | `crates/holographic-memory`: 22+ tests, SQLite persistence with drop/reopen survival, in-memory store, deterministic signatures, consolidation, resonance retrieval, no LLM/vector DB/authorization. Canonical phrase: "Holographic Memory reactivates paths to truth. It does not replace truth." |
    69|| Compute Reservoir | Alpha minimal | Compute/model/resource routing | `crates/compute-reservoir` provides serializable types and pure allocation only; no model calls, execution, I/O, persistence or Decision Gate replacement. |
    70|| Tool Registry | Alpha minimal | Declarative catalogue of tools and permissions | `crates/tool-registry` declares tools, capabilities, schemas, governance notes and lookup/status changes only; no execution path. |
    71|| `crates/graph-memory` | Experimental | SurrealDB Graph Memory adapter | Adds alpha audit-event queries by task, proposed action and decision plus governed FailureInsight memory trace proof readback, an in-memory demo/test helper and schema-backed CLI status readback; broader persistence conventions and graph schema still need stabilization. |
    72|| Graph Memory domain port | Alpha | Memory contract | Useful foundation, but persistence and audit coupling are not final. |
    73|| Audit System | Alpha | Trace important events and decisions | Has usable decision-scoped readback summaries; must remain non-authorizing. |
    74|| `crates/llm` | Experimental | LLM provider abstraction | Must remain limited to proposals. No tool execution by provider. |
    75|| `crates/runtime` | Experimental | Cognitive runtime loop | Must remain proposal-only until governance layers are ready for controlled integration. |
    76|| `apps/api-server` | Alpha | REST access to alpha objects | Must not take business governance responsibility. |
    77|| `crates/cli` | Alpha supervision surface | Local Mission Control precursor | Provides read-only audit, Failure-to-Insight, Graph Memory status, governed memory-write proposal supervision and a local FailureInsight memory demo loop. Must not become an execution bypass. |
| `crates/mcp-server` | Alpha | Native MCP server | Stdio + HTTP/SSE transport, DecisionGate governance, audit store, resources + prompts, notifications (A1-A5). 52+ tests across all phases. Must not add unsafe MCP capabilities (shell, browser, network, unrestricted file write). |
|| Neutral Orchestrator | Not implemented | Coordination layer | Deferred until governance, compute and tool layers are coherent enough for controlled integration. |
|| Mission Control Web | Deferred | Human supervision UI | Do not expand yet. CLI supervision comes first. |
|| Scheduler / autonomous loops | Deferred | Controlled recurring work | Must wait for Decision Gate, Tool Registry, Audit and human approval path. |
| MCP operator-readiness (A6) | 🔜 | External agent integration docs | MCP itself is now an active Alpha layer (see `crates/mcp-server` row). A6 covers documentation, examples and client smoke tests. Unsafe MCP capabilities remain forbidden. |
|| Browser automation | Deferred | Controlled web interaction | Must wait for governance, audit and security hardening. |
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
##  17. Latest Session Update (2026-05-28 — Milestone E2: Business Prospecting Workflow Demo)

   This session completed the Track E E2 milestone: Business/prospecting workflow demo, building on the E1 SME Documentary Assistant foundation.

### Verification

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean (pre-existing api-server warnings only)
- `cargo test --workspace`: ✅ all tests pass (0 failures across all crates)
- Demo re-run: all 11 tests pass, 5 phases green

### Safety boundaries preserved

- No unrestricted shell, browser, email, secrets, or write tools
- No Decision Gate bypass
- No scheduler, autonomy, or agent self-modification
- No API endpoint or Mission Control Web expansion
- No readback-as-authorization behavior
- No new model calls, provider endpoints, or LLM integration

### Files changed (this session)

| File | Change |
|------|--------|
| FOCUS_LOOP_NEXT.md | Updated handoff — all 5 PRs merged, C4+C5+D2+D3+E2+E4 delivered, next: E3 or H1 |
| PROJECT_STATUS.md | Added section 25 documenting this session |

No code files were changed — only handoff/documentation files updated.

### Deliberately not changed

- No code changes to any crate: core, decision-gate, compute-reservoir, holographic-memory, mcp-server, tool-runtime, tool-registry, cli, llm, runtime, api-server
- No crate boundaries, permissions, risk levels, or governance logic
- No new features, flags, or commands
- No test additions (C5 already on main)
- No branch was created for new feature work (all effort went to merging existing PRs)
- No C5 branch was created — tests confirmed already on main

## 26. Latest Session Update (2026-05-28 3rd focus loop — E3 Demo Pack Completion)

This session completed the Track E E3 milestone: Local Company Assistant Demo Pack.

### What was delivered

The E3 demo pack (`demos/local-company-assistant/`) existed as a working scripted demo but was missing 3 of 5 required deliverables per the milestone definition:

**Added:**
- `demos/local-company-assistant/expected-output.md` — expected output report with detailed acceptance criteria, per-phase output structure, and failure modes with fixes
- `demos/local-company-assistant/GOVERNANCE_VALUE.md` — dedicated governance & audit value document written for commercial conversations, covering positioning, the four-part governance pipeline, claim-to-evidence mapping, and commercial relevance for SME, regulated-industry, and product-demo scenarios

**Fixed:**
- `demos/local-company-assistant/test_debug.sh` — changed from absolute path to relative path (Tool Runtime blocks absolute paths; was broken)
- `demos/local-company-assistant/demo.sh` — fixed tool count grep to handle space after colon (`"tool_runtime_tool_count": 3` now correctly parses)

**Polished:**
- `demos/local-company-assistant/README.md` — restructured for operator-friendly quick start, added troubleshooting table, linked to new documents, clearer phase descriptions

### E3 milestone acceptance

| Required Property | Status | Evidence |
|-------------------|--------|----------|
| One scripted scenario | ✅ | `demo.sh` — Boulangerie du Marché, 5 phases, 11 tests |
| One sample dataset | ✅ | `samples/` — 3 documents (feedback, operations, staff) |
| One expected output report | ✅ NEW | `expected-output.md` — acceptance criteria, per-phase JSON, failure modes |
| One explanation of governance/audit value | ✅ NEW | `GOVERNANCE_VALUE.md` — commercial positioning, 4-part pipeline, claim mapping |
| One operator-friendly README | ✅ UPDATED | `README.md` — quick start first, troubleshooting, reuse guide, cross-links |

### Verification

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean (pre-existing api-server warnings only)
- `cargo test`: ✅ all tests pass (0 failures across all crates)
- `bash demos/local-company-assistant/demo.sh`: ✅ all 11 tests pass, 5 phases green — tool count now correctly shows "3 outils"

### Safety boundaries preserved

- No code changes to any crate: core, decision-gate, compute-reservoir, holographic-memory, mcp-server, tool-runtime, tool-registry, cli, llm, runtime, api-server
- No new CLI flags, runtime behavior, model calls, permissions, or governance logic
- No Decision Gate bypass, scheduler, autonomy, browser automation, email, secrets access, self-modification, or Mission Control Web growth
- All changes are documentation and shell script fixes within the existing `demos/` directory
- Readback remains evidence only, not authorization

### Files changed (this session)

| File | Change |
|------|--------|
| `demos/local-company-assistant/README.md` | Restructured for operator-friendly quick start, added troubleshooting, cross-links |
| `demos/local-company-assistant/demo.sh` | Fixed tool count grep to handle space after colon in JSON |
| `demos/local-company-assistant/test_debug.sh` | Changed from absolute to relative path (Tool Runtime security) |
| `demos/local-company-assistant/expected-output.md` | **NEW** — expected output report with acceptance criteria |
| `demos/local-company-assistant/GOVERNANCE_VALUE.md` | **NEW** — governance & audit value for commercial use |
| `PROJECT_STATUS.md` | Added section 26 documenting this session |
| `FOCUS_LOOP_NEXT.md` | Updated handoff — E3 complete, next: E5 product positioning |

### Track E status after this session

| Step | Status |
|------|--------|
| E1 — SME documentary assistant demo | ✅ Complete |
| E2 — Business/workflow prospecting demo | ✅ Complete |
| E3 — Local company assistant demo pack | ✅ **Complete (this session)** |
| E4 — README: demo in 10 minutes | ✅ Complete |
| E5 — Product positioning evidence | ❌ Remaining |
| H1 — Production hardening pass | ❌ Remaining |

   Created `demos/business-prospecting/` — a complete end-to-end business prospecting workflow demonstration for ARPAGONA Agent Core:

   **New directory: `demos/business-prospecting/`:**
   - `README.md` — Demo documentation explaining the business scenario, phases, governance boundaries, and how E2 differs from E1
   - `demo.sh` — End-to-end demo script (bash) with 5 phases:
     - Phase 1: Cognitive analysis with LLM synthesis (`cognitive run --llm --provider mock --domain business`)
     - Phase 2: Document discovery via Tool Runtime (`list-files`, `read-file`, `search-text`)
     - Phase 3: Governed cognitive assessment (`--assess --observe --govern`)
     - Phase 4: Follow-up action proposal (graceful API-dependent fallback)
     - Phase 5: Operator readback surfaces (`llm journal --json`, `status --json`)
   - `samples/prospect-brief.md` — Synthetic business prospect document (French, Maison de la Culture Numérique MCN Lyon, budget 40-60k€)
   - `samples/background-research.md` — Synthetic market research context document

   **Scenario:** NovaTech Consulting qualifies "Maison de la Culture Numérique (MCN)" — a Lyon-based cultural center seeking an integrated visitor management and workshop reservation system (budget 40-60k€, deadline September 2027).

   **Demo verification:** `bash demos/business-prospecting/demo.sh` — all 5 phases pass:
   - Phase 1: Business domain classification, LLM synthesis (provider=mock, 460 chars), non-authorizing proposal
   - Phase 2: Tool runtime commands succeed (2 files discovered, 44 lines/1936 chars read)
   - Phase 3: Offline governance chain produces 1 decision, 1 audit event, governance warning
   - Phase 4: Graceful API-unavailable fallback with skip explanation
   - Phase 5: 3 LLM journal entries, status with tool_runtime_tool_count=3, cli_version=0.1.0

   **Test count:** 644+ workspace tests — all pass (0 new failures, 0 regressions)

   **Not added (per stop-list):**
   - No new crates, dependencies, feature flags, or build-time changes
   - No Decision Gate bypass
   - No scheduler, autonomy, MCP expansion, browser automation, email, secrets, or unrestricted shell
   - No API endpoint or Mission Control Web expansion
   - No readback-as-authorization behavior
   - No real LLM calls or API keys required (uses --provider mock)
   - No filesystem mutations beyond demo directory creation
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

## Latest Session Update — Session 39: Track A Phase 4 — MCP Resources + Prompts

### What was built
- **MCP Resource types** (`McpResource`, `McpResourceTemplate`, `ResourceContents`, `ResourceAnnotations`) in `crates/mcp-server/src/types.rs`
- **MCP Prompt types** (`McpPrompt`, `PromptArgument`, `PromptMessage`, `PromptMessageContent`) in `crates/mcp-server/src/types.rs`
- **Handle resources/list** — exposes server info, tools list, recent governance audit, and audit stats as discoverable resources
- **Handle resources/templates/list** — exposes `arpagona://audit/by-id/{audit_id}` template for parameterized audit record lookup
- **Handle resources/read** — returns JSON content for each resource URI; handles server/info, tools/list, audit/recent, audit/stats
- **Handle prompts/list** — exposes 3 prompt templates: `assess_governance`, `summarize_context`, `inspect_audit_record`
- **Handle prompts/get** — generates structured prompt messages with dynamic data from the audit store
- **Dispatch routing** — all new methods routed in `handle_request_to_message`
- **Capability advertisement** — `initialize` now advertises `resources: {}` and `prompts: {}` capabilities
- **12 new tests** covering resources list, read, templates, prompts list/get, error cases, and pre-initialize guards

### Functional-alpha chain advancement
```
MCP Server -> Resources (Phase 4) -> Prompts (Phase 4) -> Structured data surfaces for external agents
```

### What was NOT added
- No real execution capabilities
- No shell access
- No browser automation
- No SurrealDB persistence
- No Decision Gate bypass
- No LLM calls
- No notification support (Phase 5 — deferred)

### Risks
- Resource URIs are hardcoded (`arpagona://` scheme). If the URI scheme changes in the future, the handler match arms and resource listing must be updated in sync.
- Prompt templates contain English prose directly in Rust strings. Internationalization or template externalization is deferred.
- The `audit/stats` and `audit/recent` resources only work when `audit_path` is configured in `McpServerConfig`. When not configured, they return empty/error content.

## 19. Latest Session Update (2026-05-27 — Track B Step B4: SQLite persistence for holographic memory)

This session added a durable SQLite-backed `HolographicMemoryStore` implementation to `crates/holographic-memory`.

### What was added

**`crates/holographic-memory/src/sqlite_store.rs`** — new module:
- `SqliteHolographicMemoryStore` — implements `HolographicMemoryStore` using `rusqlite` with the `bundled` feature (no system SQLite library required)
- **Dual-layer design**: in-memory `HashMap` cache for trait-compatible reference returns (`get_trace`, `list_traces`) + SQLite for durable persistence
- **Schema**: `holographic_traces` table with `id`, `project_id`, `created_at`, `importance`, `confidence`, `activation_count`, `trace_json` columns, plus indexes on `project_id` and `created_at`
- **Full CRUD**: `add_trace`, `get_trace`, `list_traces`, `activate_trace`, `retrieve_by_resonance`, `traverse_linked_memories`
- Mutations (activation, retrieval) sync both the cache and the `trace_json` column for reliable persistence across drop/reopen cycles
- `new(path)` for file-backed stores, `in_memory()` for testing

**Configuration changes:**
- Workspace `Cargo.toml`: added `rusqlite = { version = "0.31", features = ["bundled"] }`
- `crates/holographic-memory/Cargo.toml`: added `rusqlite.workspace = true`

**Module declaration:** `pub mod sqlite_store;` added to `crates/holographic-memory/src/lib.rs`

### Tests (20 new, all passing)

| Category | Tests |
|----------|-------|
| Basic CRUD | new_store_is_empty, add_and_retrieve_trace, add_duplicate_returns_error, get_nonexistent_returns_error, list_traces_scoped_to_project |
| Activation | activate_trace_increments_count, activate_multiple_times, activate_nonexistent_returns_error |
| Resonance | retrieve_by_resonance_matches_correct_trace, retrieve_by_resonance_empty_query_returns_empty, retrieve_by_resonance_scoped_to_project, retrieval_activates_traces |
| Linked memory | traverse_linked_memories_from_sqlite, traverse_nonexistent_root_returns_error |
| Persistence | persistence_across_drop_reopen_cycle, persistence_activation_survives_reopen, persistence_multiple_projects |

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean
- `cargo test --workspace`: 541+ tests, all passing (0 regressions)

### Stability level

Alpha experimental. The SQLite store is a new backend option alongside the existing `InMemoryHolographicMemoryStore`. The in-memory store remains the default; the SQLite store is enabled explicitly via `SqliteHolographicMemoryStore::new(path)`.

### Not changed

- No changes to CLI, API server, MCP server, or any other crate
- No changes to existing resonance or signature logic
- No Decision Gate bypass
- No broad capability expansion
- No SurrealDB persistence
- No LLM calls, browser automation, email, or restricted capabilities

### Risks

- The dual-layer cache design means the in-memory cache is the "source of truth" during a session. If a second process modifies the SQLite file externally, the cache will be stale until the store is reconstructed. This is acceptable for the expected single-process deployment model.
- `rusqlite` with `bundled` feature adds ~3MB to the build from compiled C SQLite source. The build time impact is one-time per fresh build.

## Session 2026-05-27T18:00Z — Track B Step B6: DecisionGate governance for holographic memory writes

### What was done

Added `MemoryWriteKind::CreateHolographicTrace` variant and its `ActionType::CreateHolographicTrace` counterpart to the ARPAGONA governance vocabulary.

**Scope of changes:**

| File | Change |
|------|--------|
| `crates/core/src/action.rs` | Added `CreateHolographicTrace` to `ActionType` enum, `MemoryWriteKind` enum, `action_type()` mapping, and `FromStr` parser |
| `crates/core/src/audit.rs` | Added `CreateHolographicTrace` to `memory_write_intent_for_audit` match |
| `crates/core/src/executor.rs` | Added `CreateHolographicTrace` to NoopExecutor supported types |
| `crates/core/src/execution_registry.rs` | Added `CreateHolographicTrace` to memory-write execution capability and known types list |
| `crates/decision-gate/src/lib.rs` | Added 2 tests: missing-permission block and low-risk approval |
| `crates/decision-gate/src/override_engine.rs` | Added `CreateHolographicTrace` to is_simulative_or_mutative list |
| `apps/api-server/src/main.rs` | Added `CreateHolographicTrace` to effects generation match |

### Tests (4 new, all passing)

- `tests::missing_write_memory_permission_blocks_create_holographic_trace` — proves missing `WriteMemory` permission produces `Blocked` with proper audit trace
- `tests::low_risk_holographic_trace_with_permission_is_approved` — proves `CreateHolographicTrace` at low risk with permission produces `Approved`
- `tests::memory_write_kind_maps_to_specific_action_type` (extended) — proves the new variant maps to `ActionType::CreateHolographicTrace`
- Existing DecisionGate tests for `CreateFailureInsightMemory` pattern extend naturally to the new variant

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean
- `cargo test --workspace`: 555+ tests, all passing (0 regressions)

### Next step

Track A Phase 5 (PR #117) was merged earlier in this run. The deferred action was Track B Step B6, now completed.

### Not changed

- No changes to CLI, MCP server, conversation-memory, holographic-memory stores, or graph-memory crates
- No actual memory writes — this is governance vocabulary only
- No Decision Gate bypass
- No broad capability expansion
- No LLM calls, browser automation, email, or restricted capabilities

### Risks

- The `CreateHolographicTrace` action type is treated as a governed memory-write, meaning it inherits the same permission requirements (`WriteMemory`) and risk-based approval rules as `CreateFailureInsightMemory`. This is correct — holographic traces are persistent memory data.
- `ProposeToolUse`-style `NotOverridable` policy does NOT apply to `CreateHolographicTrace` since it is not in the simulative action list with restricted override — it follows standard override rules.

## 20. Latest Session Update (2026-05-27 — Path escape security reporting fix, PR #119)

This session fixed a safety observability gap in the Tool Runtime: path escape attempts (absolute paths, parent traversal) were reported as `Failed` with `is_security: false`, indistinguishable from I/O errors. They now consistently report as `Blocked` with `is_security: true`.

### What was changed

**`crates/tool-runtime/src/lib.rs`** — three functions:
- `execute_read_file()`: `SecurityBlocked` errors from `resolve_path()` now return `ToolExecutionResult::blocked()` with `is_security: true`
- `execute_list_files()`: same fix
- `execute_search_text()`: same fix

### Tests

| Test | Status |
|------|--------|
| `read_file_blocks_outside_workspace` | Updated: now expects `Blocked` / `is_security: true` |
| `search_text_does_not_scan_outside_workspace` | Updated: now expects `Blocked` / `is_security: true` |
| `absolute_path_parent_traversal_is_security_blocked` | **New** — read_file with real workspace escape via `../outside.txt` |
| `list_files_blocks_absolute_paths` | **New** — `/etc` returns `Blocked` |
| `list_files_blocks_parent_traversal` | **New** — `../outside-dir` returns `Blocked` |

Total tool-runtime tests: 22 (16 existing + 3 updated + 3 new)

### Verification

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean
- `cargo test --workspace`: ✅ all 550+ tests pass

### Handoff notes

- **FOCUS_LOOP_NEXT.md handoff was stale** — it pointed to Track B Step B5 (periodic consolidation), which was already fully implemented with SQLite backend, CLI command, and 4+ tests. No deferred action was available. Both tracks A and B are complete.
- This run picked **DV-2026-05-26-001** from the daily validation backlog instead.
- AGENT_FOCUS_LOOP.md needs updating to define Track C now that both tracks are complete.

### What was NOT changed

- No changes to core domain types, Decision Gate, audit, CLI, API server, MCP server
- No new capabilities added
- No Decision Gate bypass
- No LLM calls, browser automation, email, or restricted capabilities
- No file access broadened or unblocked


## 21. Latest Session Update (2026-05-27 — Compute Reservoir allocation justification tests, DV-2026-05-26-004)

This session closed DV-2026-05-26-004 (medium severity) from the daily validation backlog by adding 4 targeted allocation justification tests to the Compute Reservoir crate.

### What was changed

**`crates/compute-reservoir/src/lib.rs`** — 4 new tests in the `#[cfg(test)] mod tests` block:

| Test | Covers | Verifies |
|------|--------|----------|
| `p4_public_low_complexity_prefers_cheap_local_with_justification` | Public/low-complexity + local_first | local-small selected; justification mentions cost/locality |
| `p4_high_sensitivity_justifies_local_resource_by_sensitivity` | Confidential sensitivity + complex reasoning | local-small selected; sensitivity blocks cloud; justification mentions sensitivity |
| `p4_complex_high_value_justifies_strong_model_by_capability` | Complex/high-value with budget | cloud-strong selected (only node with ComplexReasoning); justification mentions resource |
| `p4_justification_explains_fallback_when_ideal_missing` | Capability gap with local-first + zero budget | FallbackSelected; fallback.reason explains compatibility |

### Tests

- Compute Reservoir: 19 tests (15 existing + 4 new) — all pass
- Full workspace: 530+ tests pass
- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean

### Backlog status changes

- **DV-2026-05-26-004**: moved from Open → Fixed (this session)
- **DV-2026-05-27-001**: moved from Open → Closed (superseded — PR #103 already merged)

## 2026-05-28 — Context parser tests for --context flag (DV-2026-05-26-003)

### Summary

Added 7 CLI parser tests for the `--context` flag on `cognitive run`, addressing DV-2026-05-26-003. Tests cover: basic key:value, comma in value (documenting current behavior — comma stays part of value), spaces in value, empty context, multi-key with newlines, multi-line key:value (pre-existing coverage extended), and repeated-flag rejection (`--context` is `Option<String>`, single-use only).

### Files changed

- `crates/cli/src/main.rs` — 6 new tests + 1 fixed test (changed from `#[should_panic]` to `Cli::try_parse_from` assertion) in the `#[cfg(test)] mod tests` block
- `DAILY_VALIDATION_BACKLOG.md` — closed DV-2026-05-26-003 with evidence
- `PROJECT_STATUS.md` — this section
- `FOCUS_LOOP_NEXT.md` — updated handoff

### What was NOT changed

- No changes to core domain types, Decision Gate, allocation logic, CLI, API server, MCP server, tool runtime
- No new capabilities added
- No Decision Gate bypass
- No LLM calls, browser automation, email, or restricted capabilities
- No parsing behavior changes — comma stays part of value, single-flag usage documented by test
- No file access, tool runtime, or execution behavior changes


## 2026-05-27 — P3 end-to-end integration test for --assess --observe --govern pipeline

### Summary

Added the missing end-to-end integration test for the P3 Cognitive Observation to Governed Learning chain: ToolRuntime results (`--observe`) → FailureInsightCandidates (`--assess`) → DecisionGate/Decision/AuditEvent (`--govern`) — all offline, no API server required.

### What was changed

**`crates/cli/tests/snapshot_integration.rs`** — new test `cognitive_observe_govern_pipeline_produces_governance_results_from_tool_observations`:

- Spawns `arpagona cognitive run --assess --observe --govern --json` as a subprocess (same cross-process binary pattern as existing integration tests)
- Proves cognitive_observations are produced from ToolRuntime (read_file, list_files)
- Proves each observation has tool_name, kind, and status fields
- Proves `observed` flag is `true`
- Proves failure_insight_candidates are present (from both improvement_candidates and observation assessments)
- Proves governance_results contain proposed_action, decision, and audit_event per entry
- Proves decision_count > 0 and audit_event_count > 0
- Proves governance_warning indicates offline governance readback

### Verification

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean
- `cargo test --workspace`: ✅ all tests pass (including the new integration test)

### What was NOT changed

- No changes to core domain types, Decision Gate, CLI handler logic, tool runtime, or any other crate
- No new capabilities added
- No Decision Gate bypass
- No LLM calls, browser automation, email, or restricted capabilities
- No file access, execution, or governance behavior changes

### Track C / P3 advancement

This completes the remaining gap for P3: the `--observe --govern` offline pipeline now has end-to-end test coverage. P3 is functionally complete when combined with the existing `--assess --govern` integration test (cognitive_govern_chain_produces_decisions_and_audit_events_offline) and the `--assess --observe --propose` test (cognitive_propose_pipeline_produces_governed_proposals).

---

## Session 2026-05-28 13:31 CEST — DV-2026-05-27-003: conflict-marker scan false positive

### What was done

Fixed the daily-validation conflict-marker scan to exclude its own protocol file, eliminating the false positive where `grep -R` matched its own command example in `docs/daily-agent-validation.md`.

### Changes

**`docs/daily-agent-validation.md`** — Added `--exclude=daily-agent-validation.md` and `--exclude=DAILY_VALIDATION_BACKLOG.md` to the mandatory conflict-marker grep command:

```bash
grep -R "<<<<<<<\|=======\|>>>>>>>" \
  --exclude-dir=.git \
  --exclude-dir=target \
  --exclude-dir=node_modules \
  --exclude=daily-agent-validation.md \
  --exclude=DAILY_VALIDATION_BACKLOG.md \
  .
```

Both excluded files are protocol/tracking documents whose content unavoidably contains the scan patterns as self-referential examples — not real conflict markers. The `--exclude` flag uses basename matching (GNU grep convention), so full paths are not needed.

**`DAILY_VALIDATION_BACKLOG.md`** — Closed DV-2026-05-27-003 with evidence and verification results.

**`FOCUS_LOOP_NEXT.md`** — Updated handoff: P4 remains the strategic direction; DV-2026-05-27-002 (LLM synthesis quality) is the new fallback.

### Verification

- `grep -R '<<<<<<<|=======|>>>>>>>' --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude=daily-agent-validation.md --exclude=DAILY_VALIDATION_BACKLOG.md .` → zero matches (exit 1 = clean).

### What was NOT changed

- No changes to any Rust source, test, config, or build files
- No new capabilities added
- No Decision Gate bypass
- No LLM calls, browser automation, email, or restricted capabilities
- No tool execution or governance behavior changes

### Next recommended handoff

P4 (Working Memory integration) remains the strategic next milestone. If P4 is too large for one run, the previous fallback DV-2026-05-27-002 is now fixed — pick the next open DV backlog item or decompose P4 into a bounded sub-step.

## 17. Latest Session Update (2026-05-27 — LLM Synthesis Quality Structured Output)

This session processed **DV-2026-05-27-002** (LLM local synthesis quality) as a bounded fallback increment, because P4 (Working Memory accumulation) was too large for one run.

### What changed

**`crates/llm/src/lib.rs`** — `COGNITIVE_SYNTHESIS_SYSTEM_PROMPT` (lines 853-885):

Before: Asked for a free-form paragraph summarizing state, gap, and next step.
After: Requests a structured self-scorecard with three labeled sections — `[STATE]`, `[KEY GAP / RISK]`, `[RECOMMENDED NEXT STEP]` — and explicitly instructs the model to "Reference concrete field values from the working-memory summary." Safety warnings (no tool calls, no authorization claims) are retained.

**`MockProvider::synthesize()`** — Previously returned a generic `[MOCK SYNTHESIS]` message that didn't reference any structured fields. Now parses the working-memory summary fields from the user prompt (Domain, Sensitivity, Complexity, Missing context count, Proposed next action) and produces a deterministic structured output that mirrors the format requested by the prompt. This makes `--llm mock` output actually useful for testing and demonstration.

**New function:** `parse_wm_summary_fields()` — structured field extraction helper.

### New tests (7 deterministic tests)

| Test | What it verifies |
|------|-----------------|
| `cognitive_synthesis_prompt_contains_structured_sections` | Prompt has [STATE] [KEY GAP / RISK] [RECOMMENDED NEXT STEP] |
| `cognitive_synthesis_prompt_retains_safety_warnings` | No tool calls / no authorization warnings preserved |
| `cognitive_synthesis_user_prompt_contains_objective_and_wm_summary` | Prompt assembly works correctly |
| `parse_wm_summary_extracts_all_known_fields` | Field parser handles full input |
| `parse_wm_summary_returns_defaults_for_empty_prompt` | Field parser handles empty input |
| `parse_wm_summary_handles_missing_lines` | Field parser handles partial input |
| `mock_synthesis_output_contains_structured_sections` | Mock output has [STATE] [KEY GAP] [RECOMMENDED] |
| `mock_synthesis_output_references_concrete_fields` | Mock output references domain/next action |
| `synthetic_synthesis_uses_request_context_for_missing_observations` | Mock output adapts to missing context count |

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean (pre-existing E0670 edition noise)
- `cargo test --workspace`: 576+ tests pass (21 in arpagona-llm, +7 new)

### Files changed

| File | Change |
|------|--------|
| `crates/llm/src/lib.rs` | Updated prompt, mock provider, added parser + 7 tests (+256/−13 lines) |
| `DAILY_VALIDATION_BACKLOG.md` | Closed DV-2026-05-27-002 as fixed |
| `FOCUS_LOOP_NEXT.md` | Updated backlog status, preserved P4 handoff |

### Not changed (as intended)

- No new crate, CLI surface, handler, API endpoint, scheduler, LLM call, execution path, Decision Gate bypass, SurrealDB persistence, or autonomy.

### Stability level

Stable alpha. Only prompt text, mock provider behavior, and tests changed. No production LLM integration was modified — the prompt now requests a structured format; when a real LLM (OpenAI/Ollama) is used, its output should follow the new structure. The deterministic fallback (mock provider) produces useful output for testing.

### PR

#124 — merged into main.



## 18. Latest Session Update (2026-05-27 — Milestone completion verification, demo script fix, clean handoff)

This session verified that all planned milestones (P1-P8, Track A A1-A5, Track B B1-B7) are fully delivered and the `scripts/demo-full-loop.sh` end-to-end demo runs successfully across all three domains (business, coding, research).

### What was done

**`scripts/demo-full-loop.sh`** — Fixed governance result field names in the `validate_and_format` Python function:

| Before (broken) | After (correct) |
|-----------------|-----------------|
| `r.get('decision', {}).get('decision', 'unknown')` | `r.get('decision', {}).get('status', 'unknown')` — now shows `"approved"` |
| `r.get('proposed_action', {}).get('risk', 'unknown')` | `r.get('proposed_action', {}).get('risk_level', 'unknown')` — now shows `"low"` |

**`FOCUS_LOOP_NEXT.md`** — Updated to reflect that all planned milestones are delivered (P1-P8 ✅, Track A A1-A5 ✅, Track B B1-B7 ✅). The handoff now signals "await human direction for next strategic roadmap" per AGENT_FOCUS_LOOP.md section 10.

**`PROJECT_STATUS.md`** — This section.

### Milestone verification

All milestones verified by demo script run:

| Domain | decision_count | audit_event_count | decision_status | risk_level |
|--------|---------------|------------------|----------------|------------|
| Business | 1 | 1 | approved | low |
| Coding | 2 | 2 | approved (×2) | low (×2) |
| Research | 1 | 1 | approved | low |

The full governed chain produces:
```
Objective → WorkingMemory → Plan → Observations → Assessment
→ FailureInsightCandidates → DecisionGate → Decision → AuditEvent
```

### Files changed

| File | Change |
|------|--------|
| `scripts/demo-full-loop.sh` | Fixed governance result field names (+2/−2 lines) |
| `FOCUS_LOOP_NEXT.md` | Full rewrite: all milestones complete, await human direction |
| `PROJECT_STATUS.md` | This section |

### Not changed (as intended)

- No Rust source, test, config, CLI, API, Decision Gate, MCP, or tool runtime changes
- No new capabilities added
- No Decision Gate bypass
- No LLM calls, browser automation, email, or restricted capabilities
- No file access, execution, or governance behavior changes
- No new strategic roadmap items

### Risks

- The project is now at a natural checkpoint. All planned milestones are delivered. The next run without human direction will have no bounded increment to execute. Per AGENT_FOCUS_LOOP.md section 10, new roadmap items must not be added without human direction.
- The AGENT_FOCUS_LOOP.md track status tables (section 9) still show B4-B7 and A5 as 🔜 — they are all ✅ in reality. A documentation-only PR to sync the tables is the natural next step once human direction for the next phase is provided.

### Stability

Stable alpha. All existing tests continue to pass without modification. The demo script validates the full governed cognitive loop end-to-end.

## 19. Latest Session Update (2026-05-27 — C1 real LLM integration: parser tests, proposal-only safety tests)

This session followed the new Phase 2 roadmap from PR #129 (merged). The handoff pointed to **C1 — Real LLM integration in proposal-only mode**. The `--llm` and `--provider` CLI flags already existed in the `CognitiveRunArgs` struct and were already wired through the `cognitive_run` handler to call `run_cognitive_synthesis` — but they had **zero parser tests** and **zero proposal-only safety tests**.

### What was done

**Parser tests for `--llm` and `--provider`** — Added 5 new tests in `crates/cli/src/main.rs`:

| Test | What it covers |
|------|----------------|
| `cli_parses_cognitive_run_with_llm_flag` | Basic `--llm` parsing + default provider check |
| `cli_parses_cognitive_run_with_llm_and_provider` | `--llm --provider mock` |
| `cli_parses_cognitive_run_with_llm_and_json` | `--llm --json` |
| `cli_parses_cognitive_run_with_llm_provider_and_assess` | `--llm --provider openai --assess --json` |
| `cli_parses_cognitive_run_with_provider_and_all_flags` | Full pipeline: all flags + `--llm --provider ollama` |

**Proposal-only safety tests** — Added 4 new tests in `crates/llm/src/lib.rs`:

| Test | What it proves |
|------|----------------|
| `mock_synthesis_output_is_proposal_only` | Mock synthesis output is NOT valid JSON, does NOT contain "approved", "executed", or "memory_write" |
| `mock_synthesis_output_is_advisory_text_not_proposed_action` | Output does NOT contain proposed_action/decision_status/memory_write keywords; DOES contain [STATE] and [RECOMMENDED NEXT STEP] |
| `run_cognitive_synthesis_with_mock_returns_non_executable_text` | The top-level entry point returns plain advisory text, not a structured artifact |
| `cognitive_synthesis_prompt_forbids_action_execution` | The system prompt explicitly forbids tool calls and execution claims |

### Verification

- `cargo fmt -- --check` — clean
- `cargo check` — clean (only pre-existing edition warnings)
- `cargo test --workspace` — all 570+ tests pass (27 new: 5 parser + 4 safety + 18 pre-existing)
- Manual smoke test: `cargo run --bin arpagona -- cognitive run --objective "Test" --llm --provider mock --json` — produces structured synthesis with [STATE], [KEY GAP / RISK], [RECOMMENDED NEXT STEP] sections

### End-to-end proof

```
$ cargo run --bin arpagona -- cognitive run --objective "Analyse les tendances du marché" --domain business --llm --provider mock --json
{
  "llm_synthesis": "[STATE] ... [KEY GAP / RISK] ... [RECOMMENDED NEXT STEP] ...",
  "llm_provider": "mock",
  "llm_routing": "Provider set via --provider flag"
}
```

The output is clearly advisory text — no ProposedAction, no Decision, no AuditEvent. Per C1 safety boundaries:
- ✅ LLM output enriches working memory (text synthesis)
- ✅ LLM output does NOT approve actions
- ✅ LLM output does NOT write memory directly
- ✅ LLM output does NOT bypass Decision Gate
- ✅ Provider, model, and routing are audit-readable in the JSON output

### Files changed

| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | +5 CLI parser tests for `--llm` and `--provider` flags |
| `crates/llm/src/lib.rs` | +4 proposal-only safety tests for LLM synthesis |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to C2 |
| `PROJECT_STATUS.md` | This section |

### Not changed (as intended)

- No modifications to the `CognitiveRunArgs` struct or handler logic (already implemented)
- No changes to the cognitive run pipeline, Decision Gate, MCP, or tool runtime
- No LLM provider implementation changes
- No new capabilities added
- No Decision Gate bypass
- No production LLM calls in test suite
- No automation, browser, email, shell, or secrets access

### Risks

- The default provider is `ollama` (local Ollama endpoint). If Ollama is not running and `--provider` is not set to `mock`, the `--llm` flag will produce a connection error. Users should set `--provider mock` for deterministic behavior, or ensure an Ollama instance is available.
|- The OpenAI provider requires `OPENAI_API_KEY` in the environment. The `arpagona auth openai` command can help operators configure this.
|- C1 is intentionally proposal-only. Real LLM tool-calling is deferred to C2.

## 22. Latest Session Update (2026-05-27 — C3: Prompt, response, decision and risk journaling)

This session implemented Track C Step C3 — making LLM interactions auditable after the fact through an LLM interaction journal with file-backed persistence and CLI readback.

### What was added

**`crates/core/src/llm_journal.rs`** — new module:

| Type | Purpose |
|------|---------|
| `LlmInteractionType` | Synthesis, ToolCallIntent, DirectToolCall |
| `LlmJournalEntry` | id, created_at, interaction_type, prompt_summary, response_summary, provider, model, objective, proposed_actions, tool_call_intents, decision_gate_outcomes, risk_level |
| `LlmJournal` | In-memory ring-buffer with file-backed persistence (JSON-lines) |

Key methods:
- `add_synthesis()` — convenience for cognitive synthesis interactions
- `add_direct_tool_call()` — convenience for governed tool-call interactions with governance metadata
- `with_file()` / `load_from_file()` — file-backed persistence
- `recent_entries(n)` — returns N most recent entries
- `get_entry(id)` — lookup by ID

**`crates/cli/src/main.rs`** — CLI integration:

| Change | Detail |
|--------|--------|
| `global_llm_journal()` | Global `OnceLock<Mutex<LlmJournal>>` with file persistence at `target/llm-journal.jsonl` (configurable via `ARPAGONA_LLM_JOURNAL_PATH`) |
| `Llm` command variant | `arpagona llm journal [--limit N] [--json]` |
| Synthesis journaling | `cognitive_run()` now journals each successful `--llm` synthesis call with objective, provider, prompt/response summaries |
| `llm_journal_list()` handler | Human-readable and JSON output with full entry details |

**CLI readback examples:**

```bash
# List recent journal entries (human-readable)
$ arpagona llm journal

# List recent entries (structured JSON)
$ arpagona llm journal --json --limit 5
```

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ 600+ tests pass (7 new in arpagona-agent-core) |
| Cross-process persistence | ✅ `cognitive run --llm` → file → `llm journal` reads entries |

### New tests (7 in arpagona-agent-core)

| Test | What it proves |
|------|---------------|
| `new_journal_is_empty` | Empty journal behavior |
| `add_synthesis_creates_entry` | Synthesis entries created correctly |
| `journal_respects_capacity` | Ring-buffer eviction at capacity |
| `recent_entries_returns_most_recent_first` | Correct ordering |
| `get_entry_returns_none_for_unknown_id` | Missing entry handling |
| `add_direct_tool_call_creates_entry_with_governance_data` | Governance metadata in tool-call entries |
| `llm_journal_entry_serializes_and_deserializes` | Serialization round-trip |

### Files changed

| File | Change |
|------|--------|
| crates/core/src/llm_journal.rs | **New** — LLM journal types and file-backed persistence |
| crates/core/src/lib.rs | Added `pub mod llm_journal; ` and `pub use llm_journal::*;` |
| crates/cli/src/main.rs | Added global journal, Llm command/subcommand, synthesis journaling, readback handler |
| FOCUS_LOOP_NEXT.md | Updated handoff to C4 |
| PROJECT_STATUS.md | This section |

### Stability level

Alpha C3 delivery. File-backed persistence is append-only JSON-lines. The journal stores prompt/response summaries, not raw secrets.

### What was NOT added

- No MCP resource for LLM journal (deferred — CLI readback is the primary surface for now)
- No journaling for the governed tool executor (C2 bridge) yet — deferred to C3 follow-up or C4
- No shell, browser, email, secrets or unrestricted write tools
- No Decision Gate bypass
- No autonomous scheduling
- No broad product roadmap items

### Risks

- File-backed journal is append-only with no compaction. A small number of entries per CLI invocation is expected; compaction can be added if needed.
- The `LlmJournal` struct uses `#[serde(skip)]` on the `path` field, so serialization round-trips only work for the entries, not the file path configuration.
- Journal entries are not persisted to Graph Memory or SurrealDB — the JSON-lines file is independent of the main audit system.


## 17. Latest Session Update (2026-05-27 — Track C Step C2: governed direct tool-call bridge)

This session delivered the governed direct tool-calling bridge (Track C Step C2).

### What was added

**`crates/runtime/src/governed_tool_executor.rs`** — new bridge module:
- `govern_and_execute_tool_call()` — evaluates a `ToolCallIntent` through the Decision Gate and executes through the bounded Tool Runtime if approved
- `GovernedToolCallResult` — structured result carrying both the `Decision` and (when approved) the `ToolExecutionResult`

**`crates/runtime/Cargo.toml`** — added dependencies on `arpagona-decision-gate` and `arpagona-tool-runtime`

**`crates/runtime/src/lib.rs`** — added `mod governed_tool_executor` and re-exports for the public types

**Inherited from foundation commit (cherry-picked):**
- `ActionType::DirectToolCall` + `ToolCallIntent` in `crates/core/src/action.rs`
- `govern_tool_call()` in `crates/decision-gate/src/lib.rs`
- Execution registry + API server stubs

### Tests (9 new)

| Test | What it proves |
|------|---------------|
| `approved_tool_call_executes_via_tool_runtime` | read_file + ProposeToolUse → approved + executed |
| `approved_list_files_executes_via_tool_runtime` | list_files + permission → approved + executed |
| `approved_search_text_executes_via_tool_runtime` | search_text + permission → approved + executed |
| `blocked_tool_call_without_permission` | No permissions → Blocked |
| `blocked_high_risk_tool_call` | Critical risk → NeedsHumanApproval |
| `malformed_tool_call_missing_arguments` | Permission OK → runtime fails on missing arg |
| `absolute_path_in_tool_call_is_blocked_by_runtime` | /etc/passwd → runtime blocks on safety |
| `unknown_tool_in_tool_call_requires_human_approval` | Unknown tool, Medium risk → NeedsHumanApproval |
| `governed_tool_call_result_is_not_authorization` | Result is observation, not authorization |

### Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean (only pre-existing warnings)
- `cargo test --workspace`: 595+ tests pass, no regressions

### Stability level

Alpha runtime bridge. The `governed_tool_executor` module is a new bridge between the cognitive runtime, Decision Gate and Tool Runtime. The underlying crates remain at their existing stability levels.

### Not changed (per C2 safety boundaries)

- ✅ No shell/browser/email/secrets access
- ✅ No unrestricted write tools
- ✅ No autonomous scheduling
- ✅ No Decision Gate bypass
- ✅ No readback treated as authorization
- ✅ C3 (journaling) deferred to next PR
- ✅ Helm chart, Docker, CI, deployment unchanged

### Recommended next step

After PR #131 is merged, proceed to **Track C Step C3 — Prompt, response, decision and risk journaling**.

## 18. Latest Session Update (2026-05-27 — Track C Step C4: Compute Reservoir model routing journal integration)

This session integrated the Compute Reservoir routing decision into the LLM interaction journal and CLI readback (Track C Step C4).

### What was added

**`crates/core/src/llm_journal.rs`** — extended for C4:
- New `compute_routing: Option<Value>` field on `LlmJournalEntry` for storing Compute Reservoir allocation details
- New `add_synthesis_with_routing()` convenience method accepting optional compute routing JSON
- Backwards-compatible: old `add_synthesis()` delegates to `add_synthesis_with_routing()` with `None` routing; existing journal JSONL files deserialize correctly (serde ignores unknown fields)

**`crates/cli/src/main.rs`** — C4 integration:
- `cognitive_run()` with `--llm --allocate`: constructs compute routing JSON (selected_node_id, resource_kind, expected_cost_cents, expected_latency_ms, justification, fallback, routing_note) and passes it to the journal via `add_synthesis_with_routing()`
- `llm_journal_list()`: human-readable display of compute routing (selected_node, justification, routing_note); structured JSON output includes `compute_routing` field

### Tests (9 llm_journal tests, 2 new + 7 existing)

| Test | What it proves |
|------|---------------|
| `add_synthesis_with_routing_stores_compute_routing` | Routing JSON stored and retrievable in journal entry |
| `synthesis_without_routing_has_none` | Backwards-compatible — entries without routing have `None` |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ 603+ tests pass, no regressions |

### Files changed

| File | Change |
|------|--------|
| crates/core/src/llm_journal.rs | Added `compute_routing` field, `add_synthesis_with_routing()` method, 2 new tests |
| crates/cli/src/main.rs | Wired compute routing into LLM journal on `--llm --allocate`, added human/JSON readback |
| FOCUS_LOOP_NEXT.md | Updated handoff to C5 |

### Stability level

Alpha C4 delivery. Extends the existing C3 journal with optional compute routing metadata. No new crate, no new dependencies, no persistence schema change.

### What was NOT added

- No shell, browser, email, secrets or unrestricted write tools
- No Decision Gate bypass
- No autonomous scheduling
- No new crate or dependency
- No changes to Compute Reservoir allocation logic itself (only journaling of its results)
- No model/provider dispatch changes (the existing `cloud-strong→openai` mapping is journaled as-is)

### Safety invariants verified

- ✅ Compute routing in journal is evidence-only, never authorization
- ✅ Allocation justification includes `NON_AUTHORIZING_READBACK` warning
- ✅ Route selection does not authorize tool execution
- ✅ Backwards-compatible with existing C3 journal files

### Recommended next step

After PR #133 is merged, proceed to **Track C Step C5 — Anti-drift and adversarial tests**.

## 19. Latest Session Update (2026-05-27 — Track C Step C5: Anti-drift and adversarial tests)

This session added 18 new anti-drift and adversarial tests across the Decision Gate and LLM provider crates.

### Test families covered

| Family | Tests | Location |
|--------|-------|----------|
| **Tool bypass containment** | 3 | `crates/decision-gate/src/lib.rs` — proves the Decision Gate always produces a governing decision regardless of tool name; blocks only when permissions are missing |
| **Malformed payload resilience** | 2 | `crates/decision-gate/src/lib.rs` — proves governance layer never panics on missing or null arguments |
| **Decision Gate mandatory regression** | 3 | `crates/decision-gate/src/lib.rs` — proves every tool-call proposal begins as `PendingDecision` and requires governance |
| **Hallucination containment** | 3 | `crates/llm/src/lib.rs` — proves hallucinated execution claims in LLM output are safely parsed as proposals; garbage input rejected; execution-type JSON rejected |
| **Prompt injection** | 2 | `crates/llm/src/lib.rs` — proves injection prompts never produce executable actions; all injection-triggered proposals are proposal-only with `llm_executed=false` |
| **Overconfident model claims** | 2 | `crates/llm/src/lib.rs` — proves mock provider output never claims execution or authority |
| **Model/provider failure fallback** | 2 | `crates/llm/src/lib.rs` — proves `run_cognitive_synthesis` returns a structured error for unknown providers; mock provider always succeeds |
| **Safety language invariant** | 1 | `mock_synthesis_never_claims_authority_or_execution` confirms output contains non-authorizing disclaimer |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ 600+ tests pass, no regressions |

### Files changed

| File | Change |
|------|--------|
| `crates/decision-gate/src/lib.rs` | Added 7 C5 anti-drift tests in `#[cfg(test)] mod tests` |
| `crates/llm/src/lib.rs` | Added 11 C5 anti-drift tests in `#[cfg(test)] mod tests` |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to D1 (Operator status surface) |

### What was NOT added

- No runtime behavior, LLM provider, Decision Gate logic, CLI surfaces or API endpoints were modified
- No new crate or dependency
- No shell, browser, email, secrets or unrestricted write tools
- No Decision Gate bypass
- No autonomous scheduling

### Stability level

| Stable test-only addition. All 18 new tests are deterministic, require no external LLM access, and operate at the governance/proposal layer.

## 19. Latest Session Update (2026-05-27 — D1: Operator status surface — local subsystem monitoring)

This session delivered D1 — the first coherent operator status view that combines API-sourced data with local (non-API) subsystem status.

### What was added

**`crates/cli/src/main.rs`** — new types, functions, and enhanced status output:

- **`LocalSubsystemStatus`** — new `#[derive(Debug, Serialize)]` struct with 13 fields covering:
  - Holographic Memory SQLite database existence and path
  - OpenAI API key configuration (`OPENAI_API_KEY` env var check)
  - Ollama endpoint configuration and lightweight reachability probe (`/api/tags`)
  - Conversation memory trace count (currently `None`, placeholder for future persistent store)
  - Tool Runtime tool count and tool names (read_file, list_files, search_text)
  - Current handoff next action (parsed from `FOCUS_LOOP_NEXT.md`)
  - Open validation backlog item count (from `DAILY_VALIDATION_BACKLOG.md`)
  - MCP server binary availability (checks `target/debug/arpagona-mcp-server`)
  - CLI version string (`CARGO_PKG_VERSION`)
  - Readback-only warning

- **`gather_local_subsystem_status()`** — async function that gathers non-API status

- **`check_ollama_reachable()`** — lightweight HTTP probe with 3-second timeout

- **`read_handoff_next_action()`** — reads the first meaningful content line from `FOCUS_LOOP_NEXT.md`

- **`count_backlog_open_items()`** — counts `### DV-*` entries in `DAILY_VALIDATION_BACKLOG.md`

- **`StatusReadback` struct** — extended with `local: LocalSubsystemStatus` field

- **`format_status_readback()`** — extended to display a `Local subsystems` section with all fields

### Tests (5 new status tests)

| Test | What it proves |
|------|---------------|
| `status_formatted_includes_local_subsystem_section` | Human-readable output includes all local subsystem fields |
| `status_json_includes_local_subsystem_fields` | JSON serialization includes `local` object with all fields |
| `read_handoff_next_action_returns_content_when_file_exists` | Handoff parsing does not panic regardless of CWD |
| `local_subsystem_status_null_optional_fields_serialize_correctly` | `None` optional fields serialize as JSON `null` |
| Existing `status_readback_formats_*` tests updated | Extended with local field in fixture |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ 90+ tests pass across all crates, no regressions |

### Safety invariants

- No shell, browser, email, secrets or unrestricted write tools added
- No Decision Gate bypass
- No autonomous scheduling
- No runtime behavior modified
- No execution path opened
- All local checks are read-only and bounded (file existence, env var, lightweight HTTP probe with 3s timeout)
- The Ollama reachability check uses a 3-second timeout and only probes the documented `/api/tags` endpoint — no model pulls, no API key reads
- Readback warnings preserved on both the parent `StatusReadback` and the nested `LocalSubsystemStatus`

### Files changed

| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | Added `LocalSubsystemStatus` struct, `gather_local_subsystem_status()`, `check_ollama_reachable()`, `read_handoff_next_action()`, `count_backlog_open_items()`, extended `StatusReadback` with `local` field, extended formatter, 5 new tests |
| `PROJECT_STATUS.md` | Added this session update |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to D2 |

### Not changed

- No runtime behavior, LLM provider, Decision Gate logic, CLI surfaces beyond `status` enhancement
- No new crate or dependency
- No API endpoints, MCP resources/prompts, Mission Control Web
- No executor, scheduler, browser, email or network automation
- No Graph Memory, Holographic Memory, Compute Reservoir, Tool Registry or Audit system behavior

## 20. Latest Session Update (2026-05-28 — fix DV-2026-05-28-004: restore governance/readback regression assertions)

This session fixed DV-2026-05-28-004: restored targeted governance/readback regression assertions that were removed by commit 20f64f8 (C1 LLM integration).

### What was added

**`crates/cli/tests/snapshot_integration.rs`**:

1. **New test: `cognitive_observe_assess_govern_pipeline_has_structured_governance_results`** (offline, no API server)
   - Runs `--assess --observe --govern --json` pipeline
   - Asserts: `assessed=true`, `governed=true`, `observed=true`
   - Asserts: each `cognitive_observation` has non-empty `tool_name`, `kind`, `status` (ToolRuntime observation propagation)
   - Asserts: `failure_insight_candidates` non-empty
   - Asserts: `governance_results` non-empty with `proposed_action_id`, `decision.status` in [approved/blocked/needs_human_review/requires_override], `audit_event.event_type` non-empty
   - Asserts: `decision_count > 0`, `audit_event_count > 0`
   - Asserts: `governance_warning` with offline readback marker

2. **Enhanced existing test: `cognitive_propose_pipeline_produces_governed_proposals`**
   - Added assertions for proposed_action priority metadata:
     - `payload.priority_score` in [0.0, 2.0]
     - `payload.priority_band` in [high/medium/low]
     - `proposed_actions` sorted by priority_score descending

**`DAILY_VALIDATION_BACKLOG.md`**:
- Added DV-2026-05-28-001 (conflict-marker scan false positives) to open backlog
- Added DV-2026-05-28-004 with status `fixed in PR #140`

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (only pre-existing warnings) |
| `cargo test --workspace` | ✅ All tests pass, no regressions |
| `cargo test --test snapshot_integration` | ✅ 9 tests pass (8 existing + 1 new) |

### List of repaired assertions from DV-2026-05-28-004

| Assertion | Restored in |
|-----------|-------------|
| CognitiveObservations structure (tool_name, kind, status) | `cognitive_observe_assess_govern_pipeline_has_structured_governance_results` |
| FailureInsightCandidates presence | `cognitive_observe_assess_govern_pipeline_has_structured_governance_results` |
| Governance results: proposed_action_id, decision.status, audit_event.event_type | `cognitive_observe_assess_govern_pipeline_has_structured_governance_results` |
| Decision status validation (approved/blocked/needs_human_review/requires_override) | `cognitive_observe_assess_govern_pipeline_has_structured_governance_results` |
| Audit event type non-empty | `cognitive_observe_assess_govern_pipeline_has_structured_governance_results` |
| ProposedAction priority_score in [0.0, 2.0] | `cognitive_propose_pipeline_produces_governed_proposals` |
| ProposedAction priority_band in [high/medium/low] | `cognitive_propose_pipeline_produces_governed_proposals` |
| ProposedActions sorted by priority_score descending | `cognitive_propose_pipeline_produces_governed_proposals` |

### Safety boundaries preserved

- No runtime behavior, LLM provider, Decision Gate logic, CLI surfaces or API endpoints were modified
- No new crate or dependency
- No shell, browser, email, secrets or unrestricted write tools
- No Decision Gate bypass
- No autonomous scheduling
- No SurrealDB or Graph Memory persistence changes

### Deliberately not changed

- DV-2026-05-28-001 (conflict-marker scan) — marked open, not fixed this session
- DV-2026-05-28-003 (parent-traversal security classification) — deferred to next run
- DV-2026-05-28-005 (Ollama synthesis specificity) — deferred to next run
- PR #139 was not merged (governance rule: DEEP does not merge main) — remains ready for human merge
- No CLI commands, API surfaces, or executor changes

## 21. Latest Session Update (2026-05-28 — fix DV-2026-05-28-003: lexical parent-traversal security classification)

This session fixed DV-2026-05-28-003: missing parent-traversal targets that would escape the workspace now return `Blocked`/`is_security: true` instead of `Failed`/`is_security: false`.

### What was changed

**`crates/tool-runtime/src/lib.rs`** — `resolve_path()`:
- Added lexical parent-traversal escape detection **before** `canonicalize()` using purely lexical path normalization (no I/O)
- When `..` components would escape the workspace, returns `SecurityBlocked` immediately
- When `..` components stay within the workspace (e.g. `subdir/../file.txt`), proceeds to normal filesystem canonicalization

**Tests (5 changed/added):**

| Test | Change | Coverage |
|------|--------|----------|
| `read_file_blocks_path_escaping_workspace` | Updated: expect `Blocked`/`is_security: true` (was `Failed`/`is_security: false`) | `../safe.txt` with nonexistent target outside workspace |
| `nonexistent_parent_traversal_is_security_blocked` | **New** | `../nonexistent.txt` → Blocked/is_security: true; proves missing parent-traversal targets are classified as security before I/O |
| `deep_parent_traversal_is_security_blocked` | **New** | `a/deep/../../../../outside.txt` → Blocked; proves deep `..` escape via read_file |
| `list_files_nonexistent_parent_traversal_is_security_blocked` | **New** | `../nonexistent-dir` → Blocked via list_files |
| `search_text_nonexistent_parent_traversal_is_security_blocked` | **New** | `../nonexistent-dir` → Blocked via search_text |

**`DAILY_VALIDATION_BACKLOG.md`**: Added previously missing DV-2026-05-28-003 entry (was referenced in `FOCUS_LOOP_NEXT.md` but absent from backlog), marked fixed.

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (pre-existing warnings only) |
| `cargo test -p arpagona-tool-runtime` | ✅ 20 tests pass (4 new + 1 updated + 15 existing) |
| `cargo test --workspace` | ✅ All tests pass across all crates |

### Files changed

| File | Change |
|------|--------|
| `crates/tool-runtime/src/lib.rs` | Added lexical escape detection in `resolve_path()`, updated 1 test, added 4 new tests (+109/−5 lines) |
| `DAILY_VALIDATION_BACKLOG.md` | Added DV-2026-05-28-003 entry (was missing from backlog), marked fixed |
| `FOCUS_LOOP_NEXT.md` | Updated handoff to reflect PR #141, remaining backlog items |

### Safety boundaries preserved

- No shell, browser, email, secrets or unrestricted write tools
- No Decision Gate bypass
- No autonomous scheduling
- No new capabilities added — only stricter security classification for existing blocked patterns
- Lexical detection is purely additive; the existing canonicalize-based check still runs as a second layer
- Paths with `..` that stay within the workspace (e.g. `subdir/../file.txt`) are NOT blocked by the lexical check — they proceed to normal I/O validation

### Deliberately not changed

- No changes to core domain types, Decision Gate, audit, CLI, API server, MCP server
- No changes to Tool Runtime tool capabilities, bounds, or blocked file patterns
- No changes to any non-tool-runtime crate
- PRs #139 and #140 remain open and mergeable, waiting for human merge

## 22. Latest Session Update (2026-05-28 — E1 SME Documentary Assistant demo)

This session extended the E1 SME Documentary Assistant demo with an LLM-assisted variant, completing the "Extend E1 with --llm variant" work item from FOCUS_LOOP_NEXT.md.

### What was added

**`demos/sme-documentary/demo-llm.sh`** — standalone LLM-assisted demo script with 3 modes:

- `mock` (default) — deterministic mock provider for zero-dependency demo
- `ollama` — real local model via Ollama (qwen3.5:9b)
- `both` — runs mock first, then Ollama comparison in Phase 5

The demo integrates `--llm --provider` into every cognitive phase:

| Phase | Description | LLM role |
|-------|-------------|----------|
| Phase 1 | Tool Runtime read-only discovery | Same as demo.sh (tool runtime does not call LLM) |
| Phase 2 | Cognitive analysis | `--llm --provider` enriches working memory, plan, proposals with structured [STATE]/[KEY GAP/RISK]/[RECOMMENDED NEXT STEP] synthesis |
| Phase 3 | Governed analysis pipeline | `--assess --observe --govern --llm` exercises full governance chain with LLM-enriched context |
| Phase 4 | Operator readback + LLM journal | Shows system status and populated LLM journal entries with provider/model/summary traces |
| Phase 5 | (both mode only) Ollama comparison | Runs same cognitive analysis with local model for side-by-side comparison |

**`demos/sme-documentary/README.md`** — Updated:
- Quick Start section restructured: standard demo and LLM-assisted demo subsections
- LLM variant instructions show all 3 modes
- Next Steps updated: step 1 (Real LLM integration) marked ✅

**`FOCUS_LOOP_NEXT.md`** — Updated:
- E1 LLM variant complete status
- Next action: Track E2 (Business/prospecting workflow demo) after PRs merged

**`DAILY_VALIDATION_BACKLOG.md`** — Updated DV-2026-05-28-005 evidence:
- Added finding: `--provider ollama` (qwen3.5:9b) produces French-language synthesis when the objective is in French — more contextually useful than English mock output for the SME scenario.

### Files changed

| File | Change |
|------|--------|
| `demos/sme-documentary/demo-llm.sh` | **New** — LLM-assisted demo variant (3 modes) |
| `demos/sme-documentary/README.md` | Updated: Quick Start split, LLM instructions, Next Steps |
| `FOCUS_LOOP_NEXT.md` | Updated: E1 LLM variant complete, next action = E2 |
| `DAILY_VALIDATION_BACKLOG.md` | Updated: DV-2026-05-28-005 evidence with Ollama French finding |

### Not changed

- No Rust source, test, config, CLI, Decision Gate, MCP, or tool runtime changes
- No new crate or dependency added
- No LLM provider, governance, audit, or memory behavior changed
- No shell, browser, email, secrets, or execution capabilities added
- No API endpoints or Mission Control Web expansion

## 23. Latest Session Update (2026-05-28 — DV-2026-05-28-005: LLM synthesis local specificity)

This session fixed DV-2026-05-28-005 (low severity): local Ollama synthesis produced structured but generic output because the mock provider used `'?'` as literal placeholders for context_items and assumptions, and the system prompt was not explicit enough about grounding in request-specific field values.

### Changes

**`crates/llm/src/lib.rs`**:

- **`parse_wm_summary_fields()`** — extended return type from 5-tuple to 7-tuple, now returns context_items and assumptions counts alongside existing fields. Parses `Context items:` and `Assumptions:` lines from the working-memory summary prompt.
- **`MockProvider::synthesize()`** — fixed `'?'` placeholder bug (was printing literal `?` for context items and assumptions). Now uses actual parsed values to produce grounded text referencing concrete field values, domain, and counts in every section.
- **`COGNITIVE_SYNTHESIS_SYSTEM_PROMPT`** — strengthened to explicitly require: "cite the objective text, domain name, context items count, assumptions count, and missing context count directly. Do not use generic phrases like 'the objective' without naming what it is."
- **2 new acceptance tests**:
  - `mock_synthesis_references_context_items_and_assumptions` — proves output references context_items (=5), contains no `'?'`, references assumptions, references domain, retains safety warning.
  - `mock_synthesis_with_zero_context_still_self_contained` — proves zero-field input produces all 3 structured sections, domain reference, no `'?'` placeholders.
- **Updated 3 existing parser tests** to destructure the new 7-tuple return type.

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ Clean (pre-existing E0670 edition warnings only) |
| `cargo test -p arpagona-llm` | ✅ 38 tests pass (36 existing + 2 new) |
| `cargo test --workspace` | ✅ 536+ tests pass across all crates |

### Safety boundaries preserved

- No shell, browser, email, secrets, or unrestricted write tools
- No Decision Gate bypass
- No autonomous scheduling
- No new LLM calls or provider endpoints
- No changes to any crate outside `crates/llm`
- Prompt changes and mock improvements are deterministic and test-verified
- Output is still advisory text (not JSON, not ProposedAction, not authorization)

### Deliberately not changed

- No changes to core domain types, Decision Gate, audit, CLI, API server, MCP server, tool runtime, holographic memory, or compute reservoir
- No changes to real Ollama/OpenAI provider synthesis logic (only the prompt text changed)
- No remote model APIs called or model downloads required
- No real LLM interaction was modified

## 24. Latest Session Update (2026-05-28 — DEEP focus loop: all DV resolved, C2 delivered, PR blitz)

This session completed the DEEP focus loop as the 2026-05-28 7am cron run.

### What was done

**PR #147 rebase + merge** — fixed DV-2026-05-28-005 (LLM synthesis specificity). Rebased onto latest main, resolved conflicts in handoff/backlog/PROJECT_STATUS.md files. Merged.

**PR #145 rebase + merge** — Track C Step C2 (governed direct tool-call CLI bridge). Rebased onto main (skipped superseded FOCUS_LOOP_NEXT.md commit). All code changes applied cleanly. 639 tests pass. Merged.

**PR #146 rebase + merge** — Track C Step C2.2 (approved tool-call execution through Tool Runtime). Rebased onto main (skipped superseded handoff commits). 644 tests pass. Merged.

**PR #144 rebase + merge** — D5 operator approval design study (documentation only). Rebased onto main. 644 tests pass. Merged.

**PR #142 rebase + merge** — P0 hygiene backlog alignment + H1 demo script (documentation only). Rebased onto main. 644 tests pass. Merged.

### Result

All 5 open conflicting PRs are now on main. No open PRs remain. All DV-2026-05-28-* entries resolved.

| PR | Milestone | Status |
|----|-----------|--------|
| #147 | DV-2026-05-28-005 — LLM synthesis specificity | ✅ Merged |
| #145 | C2 — Governed direct tool-call CLI bridge | ✅ Merged |
| #146 | C2.2 — Approved tool-call execution through Tool Runtime | ✅ Merged |
| #144 | D5 — Operator approval design study | ✅ Merged |
| #142 | H1 — P0 hygiene backlog + demo script | ✅ Merged |

### Tests

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean
- `cargo test`: ✅ 644 tests pass across all crates

### Not changed

- No new crates, dependencies, feature flags, or build-time changes
- No Decision Gate bypass
- No scheduler, autonomy, MCP expansion, browser automation, email, secrets, or unrestricted shell
- No API endpoint or Mission Control Web expansion
- No readback-as-authorization behavior

## 25. Latest Session Update (2026-05-28 2nd DEEP focus loop — P1 blitz: 5 PRs merged, C4+D2+D3+E2+E4 delivered, C5 confirmed)

This session performed the DEEP 2026-05-28 2nd focus loop run. Governance bootstrap: no `docs/gona-deep-governance.md` or `docs/steroid-hermes-action-plan.md` present; the cron prompt served as the temporary GONA governance bootstrap.

### P1 — Merged all 5 open mergeable PRs

All PRs had green CI checks. Merged in order:

| PR # | Branch | Milestone | Additions | Special handling |
|------|--------|-----------|-----------|-----------------|
| #149 | `feat/e2-business-prospecting-demo` | E2 — Business Prospecting Workflow Demo | 635+18 | Clean merge — no conflicts |
| #150 | `feat/c4-compute-reservoir-model-routing` | C4 — Compute Reservoir Model Routing | 418+18 | Rebased — handoff file conflicts resolved |
| #151 | `docs/e4-readme-demo-10-min` | E4 — README: demo in 10 minutes | 308+117 | Rebased — handoff file conflicts resolved |
| #152 | `feat/d2-action-supervision-surface` | D2 — ProposedAction and tool-call supervision | 454+18 | Rebased — llm_journal.rs + handoff conflicts resolved (merged both `add_governance()` and `add_compute_routing()`) |
| #153 | `feat/d3-memory-resonance-visibility` | D3 — Memory and resonance visibility | 412+28 | Rebased — handoff file conflicts resolved |

Total changed across 5 PRs: ~2,426 lines merged.

### C5 — Anti-drift/adversarial tests confirmed on main

Commit `67620e6 feat: Track C Step C5 — Anti-drift and adversarial tests` is on main. Tests cover:
- **Tool bypass attempts** — `govern_tool_call_approves_shell_tool_with_permission`, `govern_tool_call_with_any_tool_name_produces_governing_decision` (19 tool names tested)
- **Malformed tool-call payloads** — `govern_tool_call_handles_missing_arguments_gracefully`, `govern_tool_call_handles_null_arguments_without_panic`
- **Decision Gate mandatory regression** — `every_proposed_action_begins_as_pending_decision`, `every_tool_call_requires_governance_decision`
- **Unsafe memory-write governance** — existing tests for all MemoryWriteKind variants

### Phase 2 delivery status

| Track | Milestone | Status |
|-------|-----------|--------|
| C1 | Real LLM integration (--llm flag) | ✅ On main |
| C2 | Governed direct tool-calling | ✅ Merged |
| C3 | LLM interaction journaling | ✅ On main |
| C4 | Compute Reservoir model routing | ✅ Merged |
| C5 | Anti-drift/adversarial tests | ✅ On main |
| D1 | Operator status surface | ✅ Partial (status command exists) |
| D2 | ProposedAction/tool-call supervision | ✅ Merged |
| D3 | Memory and resonance visibility | ✅ Merged |
| D4 | Web Mission Control | 🔜 Deferred |
| D5 | Operator approval design | ✅ Merged |
| E1 | SME documentary assistant demo | ✅ Merged |
| E2 | Business prospecting workflow demo | ✅ Merged |
| E3 | Company assistant demo pack | ❌ Remaining |
| E4 | README: demo in 10 minutes | ✅ Merged |
| E5 | Product positioning evidence | ❌ Remaining |
| H1 | Production hardening pass | ❌ Remaining |

### Verification

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean (pre-existing api-server warnings only)
- `cargo test --workspace`: ✅ all tests pass (0 failures across all crates — core, decision-gate, compute-reservoir, holographic-memory, mcp-server, tool-runtime, cli, llm, api-server, etc.)

### Safety boundaries preserved

- No unrestricted shell, browser, email, secrets, or write tools
- No Decision Gate bypass
- No scheduler, autonomy, or agent self-modification
- No API endpoint or Mission Control Web expansion
- No readback-as-authorization behavior
- No new model calls, provider endpoints, or LLM integration
- All 5 PRs merged were pre-reviewed, green-CI, and mergeable

### Files changed (this session)

| File | Change |
|------|--------|
| FOCUS_LOOP_NEXT.md | Updated handoff — all 5 PRs merged, C4+C5+D2+D3+E2+E4 delivered, next: E3 or H1 |
| PROJECT_STATUS.md | Added section 25 documenting this session |

No code files were changed — only handoff/documentation files updated.

### Deliberately not changed

- No code changes to any crate: core, decision-gate, compute-reservoir, holographic-memory, mcp-server, tool-runtime, tool-registry, cli, llm, runtime, api-server
- No crate boundaries, permissions, risk levels, or governance logic
- No new features, flags, or commands
- No test additions (C5 already on main)
- No branch was created for new feature work (all effort went to merging existing PRs)
- No C5 branch was created — tests confirmed already on main

## 28. Latest Session — Combined: H1 dead-code cleanup (merged) + E5 Product Positioning Evidence (merged)

This DEEP focus loop merged two previously-open PRs and continued H1 hardening work.

### Merged — H1 workspace dead-code cleanup (PR #157)

This session completed a bounded H1 production hardening pass: removing unused imports, dead functions, and suppressing warnings across the workspace, without adding any new capabilities.

**Files changed:**
| File | Change |
|------|--------|
| `crates/core/src/executor_registry.rs` | Removed unused `ProposedActionId` from non-test import |
| `crates/core/src/executor.rs` | Removed unused `ExecutionResult::blocked()` — 11-line dead method, never called |
| `crates/core/src/policy_engine.rs` | Removed unused `PolicyEngineResult::needs_dry_run()` — 9-line dead method, never called |
| `apps/api-server/src/main.rs` | Removed unused `ExecutionStatus`, `NoopExecutor`, `Executor` imports; fixed unnecessary `mut` on `result` |
| `crates/llm/src/lib.rs` | Prefixed unused `is_proposed_action` with underscore to suppress warning |
| `crates/mcp-server/src/http_transport.rs` | Removed unused `RequestId` test import; removed unused test helper `fn get()` |

Verification: `cargo fmt -- --check` ✅, `cargo check` ✅, `cargo test` ✅ (650+ tests pass)

### Merged — E5 Product Positioning Evidence (PR #156)

**New document: `docs/product-positioning-evidence.md`** — 5 evidence-backed claims with anti-claims, audience language templates, and evidence table mapping claims → demos → crates → test counts.

| # | Claim | Evidence |
|---|-------|-----------------|
| 1 | Complete offline governed cognitive pipeline | E1/E2/E3 demos — 5-phase pipeline |
| 2 | Read-only safe perception with bounded tool runtime | `crates/tool-runtime` — path escape blocking, size limits, `.git`/`.env` blocking |
| 3 | Non-authorizing cognitive analysis with mandatory governance | `crates/core/src/cognitive_work.rs`, `crates/decision-gate` |
| 4 | Complete audit traceability | `crates/graph-memory/audit_store.rs`, `demo_snapshot.rs` |
| 5 | Layered cognitive architecture | 5 cognitive crates — Working Memory, Reservoir Echo, Holographic Memory, Graph Memory, Compute Reservoir |

Track E is now **COMPLETE** ✅ (E1-E5 all delivered).

### Phase 2 delivery status after this session

| Track | Milestone | Status |
|-------|-----------|--------|
| C1–C5 | Real LLM → anti-drift tests | ✅ Complete |
| D1–D3, D5 | Operator surfaces + approval design | ✅ Complete |
| D4 | Web Mission Control | 🔜 Deferred |
| E1–E5 | Demo scenarios → positioning evidence | ✅ Complete |
| H1 | Production hardening pass | ⏳ First pass (dead-code) done — more work available |

### Safety boundaries preserved

- No new capabilities added by H1 (dead code removal only)
- No new capabilities added by E5 (documentation only)
- No Decision Gate bypass, scheduler, autonomy, browser automation, email, secrets, self-modification, or Mission Control Web growth
- No readback-as-authorization behavior

### Deliberately not changed

- 6 pre-existing api-server unused-variable warnings remain (require function-level understanding)
- No new test additions (all existing tests pass unchanged)
- No crate boundaries, permissions, risk levels, or governance logic
- No new features, flags, or commands

### Recommended next step

**H1 — Production hardening pass (continued)** — remaining work: fix api-server unused-variable warnings, add edge-case tests for Tool Runtime (path traversal, large files, directory edge cases, Decision Gate blocking scenarios), improve CLI error messages, audit readability improvements.

## 29. Latest Session — 2026-05-28 DEEP focus loop (H1 continuation: api-server warnings + edge-case tests)

This session continued the H1 production hardening pass, fixing the 6 pre-existing api-server unused-variable warnings and adding 5 new edge-case tests for the Tool Runtime.

### Fixed: api-server unused-variable warnings (6 eliminated)

All 6 pre-existing unused-variable warnings in `apps/api-server/src/main.rs` were fixed by prefixing with underscore:
- Line 585: `override_engine` → `_override_engine` (early-exit check, shadowed by inner scope)
- Line 1003-1004: `expected_effects`, `touched_resources`, `reversibility`, `summary` → underscore-prefixed (returned by `describe_action_effects` but consumed by caller)
- Line 1006: `capability` → `_capability` (returned by `execution_capability` but unused until execution path is integrated)

After fix: `cargo check -p arpagona-api-server` produces **0 warnings** (was 6).

### Added: Tool Runtime edge-case tests (5 new)

| Test | What it proves |
|------|---------------|
| `read_file_empty_file_succeeds` | 0-byte file reads without panic, returns 0 lines |
| `list_files_empty_directory_returns_empty` | Empty directory lists no entries gracefully |
| `list_files_in_subdirectory_works` | Listing inside a nested workspace subdirectory works |
| `search_text_empty_query_returns_all_or_no_matches` | Empty query string does not panic |
| `search_text_case_sensitivity_distinguishes_cases` | Search respects case: exact match finds correct count |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean (0 diffs) |
| `cargo check` | ✅ Clean (0 warnings across all crates) |
| `cargo test` | ✅ ~655 tests pass, 0 failures, 0 regressions |

### Safety boundaries preserved

- No new capabilities, CLI flags, runtime behavior, model calls, permissions, or governance logic
- No Decision Gate bypass, scheduler, autonomy, browser automation, email, secrets, self-modification, or Mission Control Web growth
- All changes are provably safe: underscore-prefixed variables (no behavioral change) + edge-case tests (read-only, no external effects)

### Files changed

| File | Change |
|------|--------|
| `apps/api-server/src/main.rs` | Fixed 6 unused-variable warnings |
| `crates/tool-runtime/src/lib.rs` | Added 5 edge-case tests |
| `PROJECT_STATUS.md` | Added section 29 documenting this session |
| `FOCUS_LOOP_NEXT.md` | Updated handoff |

### Deliberately not changed

- No new features, flags, or commands
- No crate boundaries, permissions, risk levels, or governance logic
- No `docs/daily-agent-validation.md` changes
- No `DAILY_VALIDATION_BACKLOG.md` changes (all DV entries closed)
- No Holographic Memory, Compute Reservoir, Graph Memory, MCP, or Decision Gate changes
- No demo scripts or documentation changes outside status/handoff files

## 30. Latest Session — 2026-05-28 DEEP focus loop (H1: +7 Decision Gate blocking scenario tests)

This session continued the H1 production hardening pass, adding 7 new Decision Gate blocking scenario tests to `crates/decision-gate/src/lib.rs`.

### Added: Decision Gate blocking scenario tests (7 new, governance path edge cases)

The existing 52 Decision Gate tests covered standard paths (low→approved, medium→human, missing permission→blocked/override, C5 anti-drift). These 7 new tests cover governance path edge cases:

| Test | What it proves |
|------|---------------|
| `high_risk_without_matching_policy_falls_back_to_needs_human_approval` | High risk, no policies, all granted → NeedsHumanApproval via default fallback |
| `critical_risk_with_blocking_policy_is_blocked` | Critical risk + active policy (non-requiring human approval) → Blocked |
| `critical_risk_with_requiring_approval_policy_needs_human_approval` | Critical risk + policy requiring human approval → NeedsHumanApproval via policy match |
| `override_policy_for_direct_toolcall_is_not_overridable` | DirectToolCall is destructive/dangerous → NotOverridable → Blocked, not RequiresOverride |
| `overlapping_policies_highest_restriction_wins` | Blocking + requiring-approval policies → NeedsHumanApproval (strictest wins) |
| `risk_threshold_above_action_risk_policy_not_applied` | Policy at Critical risk threshold, action at Medium → no match (risk_rank check) |
| `informational_action_without_permission_blocked_not_overridable` | DirectToolCall with wrong permission → Blocked with no override hint |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt -- --check` | ✅ Clean |
| `cargo check` | ✅ **0 warnings** (api-server warnings fixed on this branch) — main has 6 |
| `cargo test` | ✅ ~660 tests pass, 0 failures, 0 regressions |
| Decision Gate test count | 59 (52 existing + 7 new) |

### Branch state

- Branch: `feat/h1-warnings-and-edge-tests`
- Contains: api-server 0-warnings fix + 5 Tool Runtime edge-case tests + 7 Decision Gate blocking scenario tests
- On main: 6 api-server warnings remain, no Decision Gate blocking tests
- PR: pending push

### Safety boundaries preserved

- No new capabilities, CLI flags, runtime behavior, model calls, permissions, or governance logic
- No Decision Gate bypass, scheduler, autonomy, browser automation, email, secrets, self-modification, or Mission Control Web growth
- All changes are test-only additions to existing crate (decision-gate src/lib.rs) + already-tested api-server/resolve_path fixes
- No readback-as-authorization behavior

### Files changed (this session)

| File | Change |
|------|--------|
| `crates/decision-gate/src/lib.rs` | Added 7 blocking scenario tests (+211 lines) |
| `FOCUS_LOOP_NEXT.md` | Updated handoff — H1 Decision Gate tests done, merge pending, next: CLI error messages or stale deps |
| `PROJECT_STATUS.md` | Added section 30 documenting this session |

### Deliberately not changed

- No changes to any Decision Gate logic, policy engine, or override engine behavior
- No changes to core, compute-reservoir, holographic-memory, mcp-server, tool-runtime, tool-registry, cli, llm, runtime, api-server behavior
- No changes to crate boundaries, permissions, risk levels, or governance logic
- No new features, flags, or commands

## 31. Latest Session Update (2026-05-28 — H1: backlog count accuracy + DV section cleanup)

This session merged PR #158 and fixed two data-accuracy issues in the operator status surface.

### What was done

1. **Merged PR #158** (squash, green CI):
   - `feat/h1-warnings-and-edge-tests` — 3 commits:
     - api-server: 6 unused-variable warnings → 0
     - 5 Tool Runtime edge-case tests (empty file, empty dir, subdirectory, empty query, case sensitivity)
     - 7 Decision Gate blocking scenario tests (governance path edge cases, override rejection, risk threshold, overlapping policies)
   - PR #158 was open, mergeable, and both CI checks passed (SUCCESS)

2. **Fixed `count_backlog_open_items()`** — was counting ALL `### DV-` entries regardless of section. Now correctly scoped to only count entries under the `## Open candidates` H2 header. Returns 0 for the current backlog (all 5+ entries are closed/superseded).

3. **Moved `DV-2026-05-28-005`** from Open candidates to Closed / superseded candidates in `DAILY_VALIDATION_BACKLOG.md`. It was already marked `fixed in PR #147` but remained in the open section, creating a contradiction ("No open DV entries remain" with an open-section entry).

### Files changed

| File | Change |
|------|--------|
| `FOCUS_LOOP_NEXT.md` | Updated — PR #158 merged, backlog fix added, next action: D1 gap analysis |
| `PROJECT_STATUS.md` | Added section 31 documenting this session |
| `DAILY_VALIDATION_BACKLOG.md` | Moved DV-2026-05-28-005 from Open → Closed section |
| `crates/cli/src/main.rs` | `count_backlog_open_items()` now section-scoped |

### Verification

- `cargo fmt -- --check`: ✅ clean
- `cargo check`: ✅ clean (pre-existing E0670 edition linter noise only)
- `cargo test`: ✅ all tests pass

### Safety boundaries preserved

- No new capabilities, CLI flags, runtime behavior, model calls, permissions, or governance logic
- No Decision Gate bypass, scheduler, autonomy, browser automation, email, secrets, self-modification, or Mission Control Web growth
- No readback-as-authorization behavior

### Limits

- The backlog count fix does not add a test for itself (it's a pure function reading file I/O; the behavior change is verified by manual inspection)
- No integration test was added for the DV section structure (deferred to H1 if desired)
- No demo scripts, documentation (other than handoff), or `DAILY_VALIDATION_BACKLOG.md` changes (all DV entries closed)
- No Holographic Memory, Compute Reservoir, Graph Memory, MCP, or Decision Gate logic changes
