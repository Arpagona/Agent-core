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
- `docs/compute-reservoir.md` now frames the future Compute Reservoir without implementing it.
- `crates/core` exists and contains the core domain vocabulary: agents, workspaces, tasks, goals, proposed actions, decisions, policies, permissions, risks, graph primitives, audit events, memory concepts and cognitive primitives.
- `Decision Gate` currently exists as alpha logic inside `crates/core`.
- `Reservoir Echo` currently exists inside the Cognitive Runtime primitives as short-term volatile cognitive continuity.
- `crates/graph-memory` exists as an experimental SurrealDB adapter for Graph Memory persistence.
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
| `docs/compute-reservoir.md` | Stable foundation | Future Compute Reservoir framing | Documentation only; no crate implemented yet. |
| `crates/core` | Stable foundation | Domain vocabulary and pure types | Must not become a catch-all crate. Governance logic should be extracted when safe. |
| Core domain types | Stable foundation | Shared typed language | Should remain pure, serializable and dependency-light. |
| Decision Gate | Alpha | Pre-execution governance | Currently inside `crates/core`; should become `crates/decision-gate`. |
| Reservoir Echo | Alpha | Short-term cognitive continuity | Volatile traces only. Not persistent memory. Not model routing. Not Compute Reservoir. |
| Compute Reservoir | Not implemented | Compute/model/resource routing | Must come next after Decision Gate extraction. Do not implement in this recentering pass. |
| Tool Registry | Not implemented | Declarative catalogue of tools and permissions | Must exist before any real tool execution. |
| `crates/graph-memory` | Experimental | SurrealDB Graph Memory adapter | Needs persistence conventions and graph schema stabilization. |
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
- audit persistence and causal trace design;
- exact crate boundaries for governance layers.

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
- API, CLI, LLM and runtime layers are advancing before Tool Registry and Compute Reservoir.
- Decision Gate should become a dedicated crate to prevent `core` from taking governance responsibility.
- Reservoir Echo must not be confused with Compute Reservoir.
- No tool execution must be introduced before Tool Registry + Decision Gate + Audit are stable.
- API server and CLI could accidentally become privileged orchestration layers if responsibilities are not constrained.
- LLM provider abstraction could drift toward tool-calling unless explicitly kept proposal-only.
- Runtime loops could drift toward autonomy before human-governed control paths exist.
- Graph Memory and Audit could diverge unless important decisions produce durable, queryable traces.

## 7. Next Recommended Work

Recommended sequence from the current state:

1. Extract Decision Gate into `crates/decision-gate`.
2. Create minimal `crates/compute-reservoir`.
3. Create `crates/tool-registry`.
4. Stabilize Graph Memory persistence.
5. Stabilize Audit.
6. Only then continue API/CLI/Runtime integration.

The extraction of Decision Gate should be done carefully and only if tests remain green. If the extraction touches too many imports or breaks downstream crates, first add boundary documentation and migration notes, then extract in a dedicated change.

## 8. Target Architectural Order

The target consolidation order is:

1. Core Domain Types
2. Decision Gate separated
3. Compute Reservoir minimal
4. Tool Registry
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

This session performed a documentation-only architectural re-centering.

Changed:

- created `PROJECT_STATUS.md` as the canonical operational status file;
- updated `README.md` to require reading `PROJECT_OBJECTIVES.md` and `PROJECT_STATUS.md` before modifications;
- updated `README.md` to state that the immediate objective is consolidation, not feature expansion;
- updated `docs/roadmap.md` to clarify that some bricks were prototyped out of order and must now return to the target architectural sequence;
- updated `docs/architecture.md` with an `Architectural Re-Centering` section;
- created `docs/compute-reservoir.md` as a documentation-only framing file.

Not changed:

- no new crate was created;
- no runtime feature was added;
- no endpoint was added;
- no provider was added;
- no tool execution was introduced;
- Decision Gate was not moved yet;
- Compute Reservoir was not implemented yet;
- Tool Registry was not implemented yet.

Next recommended action remains: extract Decision Gate into `crates/decision-gate` in a dedicated safe change, then create a minimal `crates/compute-reservoir`.
