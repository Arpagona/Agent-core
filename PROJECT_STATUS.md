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
- `crates/graph-memory` exists as an experimental SurrealDB adapter for Graph Memory persistence and alpha audit trace lookup by workspace, task, proposed action and decision.
- `crates/llm` exists as an experimental provider abstraction that can produce `ProposedAction` objects with `PendingDecision`, without executing tools.
- `crates/runtime` exists as an experimental cognitive runtime loop that stops at action proposal.
- `apps/api-server` exists as an alpha Axum API server.
- `crates/cli` exists as an alpha terminal interface and now provides the first read-only local supervision surface for decision-scoped audit readback.
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
| Compute Reservoir | Alpha minimal | Compute/model/resource routing | `crates/compute-reservoir` provides serializable types and pure allocation only; no model calls, execution, I/O, persistence or Decision Gate replacement. |
| Tool Registry | Alpha minimal | Declarative catalogue of tools and permissions | `crates/tool-registry` declares tools, capabilities, schemas, governance notes and lookup/status changes only; no execution path. |
| `crates/graph-memory` | Experimental | SurrealDB Graph Memory adapter | Adds alpha audit-event queries by task, proposed action and decision; broader persistence conventions and graph schema still need stabilization. |
| Graph Memory domain port | Alpha | Memory contract | Useful foundation, but persistence and audit coupling are not final. |
| Audit System | Alpha | Trace important events and decisions | Has usable decision-scoped readback summaries; must remain non-authorizing. |
| `crates/llm` | Experimental | LLM provider abstraction | Must remain limited to proposals. No tool execution by provider. |
| `crates/runtime` | Experimental | Cognitive runtime loop | Must remain proposal-only until governance layers are ready for controlled integration. |
| `apps/api-server` | Alpha | REST access to alpha objects | Must not take business governance responsibility. |
| `crates/cli` | Alpha supervision surface | Local Mission Control precursor | Preferred near-term product surface for read-only audit/task/action supervision. Must not become an execution bypass. |
| Neutral Orchestrator | Not implemented | Coordination layer | Deferred until governance, compute and tool layers are coherent enough for controlled integration. |
| Mission Control Web | Deferred | Human supervision UI | Do not expand yet. CLI supervision comes first. |
| Scheduler / autonomous loops | Deferred | Controlled recurring work | Must wait for Decision Gate, Tool Registry, Audit and human approval path. |
| MCP integration | Deferred | External tool ecosystem | Must wait for Tool Registry and security hardening. |
| Browser automation | Deferred | Controlled web interaction | Must wait for governance, audit and security hardening. |
| Security hardening | Deferred | Production-grade protection | Final V0 hardening stage, not a reason to bypass governance now. |

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
- Compute Reservoir allocation heuristics and telemetry shape;
- audit persistence and causal trace design;
- future Failure-to-Insight audit conventions, CLI readback and Graph Memory integration;
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

This session added the minimal `FailureInsight` domain vocabulary to `crates/core`.

Changed:

- added pure serializable Failure-to-Insight domain types in `crates/core/src/failure_insight.rs`;
- added `FailureInsightId` to the existing core ID vocabulary;
- exported the new vocabulary from `crates/core`;
- added unit tests for construction, trace links, status handling and serde roundtrip;
- updated this status document to reflect that the minimal domain vocabulary now exists.

Stability level: stable core-domain vocabulary; no persistence or runtime behavior added.

Limits:

- no persistence was added;
- no CLI command or API endpoint was added;
- no runtime behavior was added;
- no real tool execution was introduced;
- no destructive capability was added;
- no approval, rejection or authorization behavior was added;
- no Decision Gate behavior was changed;
- no Graph Memory integration, schema, migration or runtime behavior was changed;
- no scheduler, autonomy, MCP, browser automation, credential handling or Mission Control Web growth was introduced;
- `FailureInsight` remains learning and observability vocabulary only, never approval, authorization, orchestration, self-modification or execution state.

Architectural risk:

- low for alpha use. The change is bounded to pure Rust domain types and optional trace links, preserving the separation between vocabulary, persistence, runtime, governance and execution.

Recommended next step: add a separate bounded PR for audit conventions around when a `FailureInsight` should be referenced or extracted, still without Graph Memory persistence, CLI readback, Decision Gate influence or autonomous generation.
