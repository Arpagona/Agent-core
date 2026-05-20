# ARPAGONA Agent Core — Project Status

This document is the canonical operational status file for ARPAGONA Agent Core.

It describes the current implementation state, stability level, architectural risks, explicit stop-list, and the recommended next sequence of work.

Every future contributor or agent must read this file together with `PROJECT_OBJECTIVES.md` before modifying the repository.

## 1. Current State

The repository currently contains a fast-moving alpha foundation with several experimental building blocks already present.

Current observed state:

- `PROJECT_OBJECTIVES.md` exists and defines the canonical vision of the project.
- `PROJECT_STATUS.md` exists and defines the canonical operational status of the repository.
- `README.md` now points contributors and agents to both canonical project files before any modification.
- `docs/roadmap.md` now distinguishes the target architectural order from experimental work already prototyped out of order.
- `docs/architecture.md` now includes explicit architectural re-centering guidance.
- `docs/compute-reservoir.md` now frames the alpha minimal Compute Reservoir crate and its non-goals.
- `docs/tool-registry.md` now frames the alpha minimal Tool Registry crate, its declarative role and its explicit non-goals.
- `docs/causal-trace.md` now documents alpha conventions for linking proposed actions, tasks, decisions and audit events.
- `crates/core` exists and contains the core domain vocabulary: agents, workspaces, tasks, goals, proposed actions, decisions, policies, permissions, risks, graph primitives, audit events, memory concepts and cognitive primitives.
- `Decision Gate` now exists as alpha governance logic inside `crates/decision-gate`.
- `crates/compute-reservoir` now exists as an alpha minimal pure Rust crate with compute inventory/allocation types and a deterministic `allocate_compute` function.
- `crates/tool-registry` now exists as an alpha minimal declarative catalogue for tool definitions, capabilities, schemas, permissions, risk levels and enabled/disabled states, without execution.
- `Reservoir Echo` currently exists inside the Cognitive Runtime primitives as short-term volatile cognitive continuity.
- `crates/graph-memory` exists as an experimental SurrealDB adapter for Graph Memory persistence and alpha audit trace lookup by workspace, task, proposed action and decision.
- `crates/llm` exists as an experimental provider abstraction that can produce `ProposedAction` objects with `PendingDecision`, without executing tools.
- `crates/runtime` exists as an experimental cognitive runtime loop that stops at action proposal.
- `apps/api-server` exists as an alpha Axum API server.
- `crates/cli` exists as an alpha terminal interface.
- `apps/mission-control` exists only as a placeholder and must remain deferred.
- `workers/python-ingestion` exists only as a placeholder and must remain deferred.

The implementation already demonstrates the founding rule:

```text
Agent -> ProposedAction -> DecisionGate -> Decision -> Audit
```

However, the repository must now be re-centered around governance layers before adding visible features.

## 2. Stability Matrix

| Component | Status | Role | Notes |
|---|---|---|---|
| `PROJECT_OBJECTIVES.md` | Stable foundation | Canonical project vision | Must be read before every significant change. |
| `PROJECT_STATUS.md` | Stable foundation | Canonical operational status | Must be updated after every significant change. |
| `README.md` | Stable foundation | Entry point for contributors | Points to canonical objective/status files and consolidation priority. |
| `docs/roadmap.md` | Stable foundation | Architectural implementation order | Re-centered around governance-first consolidation. |
| `docs/architecture.md` | Stable foundation | Target architecture and boundaries | Includes Architectural Re-Centering section. |
| `docs/compute-reservoir.md` | Stable foundation | Compute Reservoir framing | Documents the alpha minimal crate and the boundary with Decision Gate, Graph Memory and Tool Registry. |
| `docs/tool-registry.md` | Stable foundation | Tool Registry framing | Documents the declarative registry boundary, explicit non-goals and alpha surface. |
| `docs/causal-trace.md` | Alpha foundation | Causal trace conventions | Documents current links and alpha audit trace queries for tasks, proposed actions, decisions and audit events without adding execution. |
| `crates/core` | Stable foundation | Domain vocabulary and pure types | Must not become a catch-all crate. Governance logic should be extracted when safe. |
| Core domain types | Stable foundation | Shared typed language | Should remain pure, serializable and dependency-light. |
| Decision Gate | Alpha | Pre-execution governance | Extracted into `crates/decision-gate`; `crates/core` no longer reexports the Decision Gate logic. |
| Reservoir Echo | Alpha | Short-term cognitive continuity | Volatile traces only. Not persistent memory. Not model routing. Not Compute Reservoir. |
| Compute Reservoir | Alpha minimal | Compute/model/resource routing | `crates/compute-reservoir` provides serializable types and pure allocation only; no model calls, execution, I/O, persistence or Decision Gate replacement. |
| Tool Registry | Alpha minimal | Declarative catalogue of tools and permissions | `crates/tool-registry` declares tools, capabilities, schemas, governance notes and lookup/status changes only; no execution path. |
| `crates/graph-memory` | Experimental | SurrealDB Graph Memory adapter | Adds alpha audit-event queries by task, proposed action and decision; broader persistence conventions and graph schema still need stabilization. |
| Graph Memory domain port | Alpha | Memory contract | Useful foundation, but persistence and audit coupling are not final. |
| Audit System | Alpha | Trace important events and decisions | Needs stabilization before execution layers grow. |
| `crates/llm` | Experimental | LLM provider abstraction | Must remain limited to proposals. No tool execution by provider. |
| `crates/runtime` | Experimental | Cognitive runtime loop | Must remain proposal-only until governance layers are stable. |
| `apps/api-server` | Alpha | REST access to alpha objects | Must not take business governance responsibility. |
| `crates/cli` | Alpha | Terminal interface | Must remain a control/testing interface, not an execution bypass. |
| Neutral Orchestrator | Not implemented | Coordination layer | Deferred until governance, compute and tool layers exist. |
| Mission Control Web | Deferred | Human supervision UI | Do not expand yet. Governance first. |
| Scheduler / autonomous loops | Deferred | Controlled recurring work | Must wait for Decision Gate, Tool Registry, Audit and human approval path. |
| MCP integration | Deferred | External tool ecosystem | Must wait for Tool Registry and security hardening. |
| Browser automation | Deferred | Controlled web interaction | Must wait for governance, audit and security hardening. |
| Security hardening | Deferred | Production-grade protection | Final V0 hardening stage, not a reason to bypass governance now. |

## 3. What Is Stable

Stable foundations:

- the founding principle: no direct execution by agents;
- the canonical objective document;
- the canonical operational status document;
- the monorepo direction;
- Rust as backend foundation;
- local-first, graph-native, compute-aware, auditable and human-governed architecture;
- `ProposedAction -> DecisionGate -> Decision -> Audit` as the mandatory control path;
- separation between domain vocabulary and adapters as an architectural rule;
- documentation-level separation between Reservoir Echo and Compute Reservoir.

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
- exact crate boundaries for remaining governance layers.

Experimental means: useful for learning and integration tests, but not stable enough to justify feature expansion around it.

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

These capabilities are explicitly blocked until Decision Gate, Compute Reservoir, Tool Registry, Graph Memory persistence and Audit are stabilized in the correct order.

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

## 7. Next Recommended Work

Recommended sequence from the current state:

1. Keep `crates/tool-registry` as a declarative catalogue only and harden it if gaps appear.
2. Stabilize Graph Memory persistence and Audit causal trace conventions.
3. Stabilize `crates/compute-reservoir` only as needed for future governed integration.
4. Only then continue API/CLI/Runtime integration.

The Decision Gate extraction is complete, the Compute Reservoir exists as alpha minimal, and the Tool Registry now exists as alpha minimal declarative catalogue. Keep `crates/core` limited to domain vocabulary, keep governance logic in `crates/decision-gate`, and do not treat compute allocation or tool lookup as action approval.

## 8. Target Architectural Order

The target consolidation order is:

1. Core Domain Types
2. Decision Gate separated
3. Compute Reservoir minimal
4. Tool Registry minimal
5. Graph Memory + SurrealDB stabilized
6. Audit System stabilized
7. Neutral Orchestrator
8. API Server Axum
9. Mission Control Web
10. Scheduler / controlled autonomous loops
11. LLM Provider abstraction stabilized
12. End-to-end demo
13. Security hardening

Some components already exist experimentally outside this order. They must not be treated as permission to expand features. They should be stabilized or constrained according to this target sequence.

## 9. Explicit Stop-List for Feature Expansion

Stop feature expansion until the governance layers are stabilized.

Do not add:

- new runtime capabilities;
- new API endpoints;
- new LLM providers;
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
- multi-agent autonomous execution.

Allowed work during the recentering phase:

- documentation cleanup;
- crate boundary clarification;
- tests that protect existing behavior;
- Decision Gate extraction planning;
- Compute Reservoir design documentation;
- Tool Registry design documentation;
- audit and graph persistence stabilization work that does not introduce execution.

## 10. Session Update Rule

Every future agent must update `PROJECT_STATUS.md` after every significant modification.

A significant modification includes:

- adding, removing or renaming a crate;
- changing the responsibility of a crate;
- adding a new API surface;
- changing Decision Gate behavior;
- changing Graph Memory or Audit persistence;
- adding a provider, runtime loop, worker or interface;
- changing security assumptions;
- changing the project roadmap or implementation order.

The update must clearly state whether the change is stable, alpha, experimental, deferred or not implemented.

## 11. Latest Session Update

This session stabilized the canonical alpha `AuditTraceSummary` shape as a Rust-first human-supervision readback object before exposing it through CLI/API.

Changed:

- `AuditTraceSummary` now includes `first_event_at` and `last_event_at` alongside first/last audit event ids, preserving chronological boundary timestamps from already-ordered audit-event lists;
- summary tests now protect causal link preservation, decision scope, event count, chronological boundary ids/timestamps, workspace/task scope, proposed action id and the non-approval/non-execution interpretation of readback markers;
- `docs/causal-trace.md` now documents the canonical alpha summary shape with temporal boundary fields and reiterates chronological ordering assumptions.

Stability level: alpha Audit readback shape stabilization.

Limits:

- no real tool execution was introduced;
- no API endpoint was added;
- no CLI behavior was expanded;
- no Decision Gate behavior was changed;
- no Graph Memory persistence schema or migration was changed;
- no graph-edge mirroring was introduced;
- no Runtime, scheduler, MCP, browser automation, provider or Mission Control growth was introduced;
- the summary remains readback only and must not be treated as approval, authorization, orchestration or execution state.

Architectural risk:

- low, provided downstream consumers treat the summary as a typed supervision readback and keep execution/authorization decisions inside the governed Decision Gate path.

Recommended next step: expose the summary through a tiny CLI/API inspection path only if explicitly chosen as the next small increment; otherwise continue Graph Memory persistence convention stabilization.
