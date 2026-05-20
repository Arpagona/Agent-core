# ARPAGONA Agent Core — Project Objectives

This document is the canonical objective file for ARPAGONA Agent Core. It defines what we are building, why we are building it, and the principles that must guide every implementation decision. It must be read together with `PROJECT_STATUS.md`, `docs/operating-doctrine.md`, `docs/development-acceleration.md` and `docs/failure-to-insight.md` before modifying the repository.

## 1. Purpose

ARPAGONA Agent Core aims to become a professional, local-first, graph-native and compute-aware agentic runtime.

It is not a chatbot, not a fork of an existing framework, and not a loose collection of automation scripts. It is a governed runtime where AI agents can reason, remember, plan and propose actions, while the system remains controlled, auditable, secure and cost-aware.

The project exists to address weaknesses observed in current agentic systems: noisy context growth, poor long-term memory, weak invalidation of obsolete information, insufficient auditability, excessive cloud model usage, unsafe tool execution patterns, and lack of compute-resource awareness.

## 2. Foundational Principle

No agent executes directly. Agents only propose actions. Every action passes through the Decision Gate. Every important decision is recorded in the graph. Every sensitive action requires human approval.

In French: aucun agent n'agit directement. Un agent propose une action. Le Decision Gate évalue l'action. L'action est approuvée, bloquée, reroutée ou soumise à validation humaine. Toute décision importante est tracée dans le graphe.

This principle must never be bypassed for convenience.

## 3. Core Objectives

ARPAGONA Agent Core must provide the following core systems:

1. Core Domain: stable typed vocabulary for agents, humans, workspaces, tasks, actions, decisions, facts, tools, policies, risks, permissions, sources, observations, episodes and compute resources.

2. Graph Memory: structured memory based on typed entities, relations, facts, sources, provenance, temporal validity, confidence and invalidation.

3. Decision Gate: deterministic pre-execution control layer that evaluates proposed actions before they affect files, APIs, tools, users or external systems.

4. Compute Reservoir: resource-awareness layer that chooses the best cognitive or computational resource for a task: cloud LLM, local LLM, worker, deterministic tool, GPU, CPU or deferred job.

5. Tool Registry: declarative catalogue of allowed tools with schemas, permissions, risk levels and enabled/disabled states.

6. Neutral Orchestrator: general-purpose coordinator able to receive objectives, create tasks, recall context, request compute allocation, propose actions, pass them through the Decision Gate and record outcomes.

7. Audit System: traceability layer for requests, recalled context, compute choices, proposed actions, decisions, approvals, tool calls, observations, memory updates, errors, invalidations and explicit decision reasons.

8. Mission Control: web dashboard that makes the runtime observable and controllable.

9. Failure-to-Insight: first-order learning layer that turns execution failures, blocked decisions, incorrect proposals, bad routing, missing context, policy gaps and human corrections into durable insights. These insights must improve future documentation, tests, audit conventions, memory, policies and routing decisions. They inform future decisions, but they are never approval, authorization or permission to execute.

## 4. Rippletide-Inspired Method

ARPAGONA Agent Core is conceptually inspired by the runtime-enforcement approach promoted by systems such as Rippletide, without copying their implementation.

The key lesson is that production agents need a deterministic decision layer between the agent and the real world. Better reasoning is not enough. The runtime must verify whether an action is allowed, supported by valid context, compliant with policy and traceable before it executes.

The design must therefore include pre-execution enforcement: an agent proposes an action, the runtime intercepts it, checks applicable context and policies, then approves, blocks, escalates, reroutes or requests more context.

The system must also implement a decision context graph. The goal is not merely to retrieve semantically similar text. The goal is to answer: which context is allowed, valid, relevant and applicable to this action right now?

The core graph-memory primitives should be:

- remember: store structured facts with provenance;
- relate: create typed relationships between entities and facts;
- recall: retrieve applicable context for a task or action;
- invalidate: expire, supersede, revoke or mark facts as unreliable.

Every important decision must produce a causal audit trace: proposed action, context used, valid facts, applied policies, decision result, approval source and reason.

When a decision, proposal, routing choice, recalled context or human correction reveals a failure, the system must preserve enough evidence to transform that failure into a durable Failure-to-Insight artifact. The correction may target code, tests, policy, memory, documentation or operating doctrine, but the insight itself remains informational and non-authorizing.

Decision audit traces must be explanatory, not merely referential. A decision audit event must be sufficient for a human supervisor or later agent to understand why a decision was approved, blocked, escalated, rerouted or left pending without inspecting hidden runtime state. At minimum, decision audit events must include:

- proposed_action_id;
- decision_id;
- action_type;
- risk level;
- decision outcome;
- explicit human-readable reason;
- matched policy rule, fallback rule or `deny_by_default` marker;
- required permission when relevant;
- actor/source that triggered the evaluation;
- timestamp;
- suggested next action when blocked or incomplete.

A `Decision: blocked` result without an explicit reason in the audit trace is considered an alpha defect. For example, an attempted `read_memory` action that is blocked must explain whether the cause is missing policy, unavailable memory backend, missing permission, deny-by-default behavior or required human confirmation.

## 5. Compute Reservoir Objective

The Compute Reservoir is a first-class architectural pillar.

Its role is to make the runtime aware of available resources and able to route tasks intelligently. It must manage strong cloud models, local LLMs, embedding models, OCR/parsing workers, deterministic tools, local GPU, local CPU, remote machines, deferred jobs and fallback resources.

It must answer: what resource should process this task, why this resource, what expected cost, what expected latency, whether the resource is compatible with the data sensitivity level, whether a local model is sufficient, whether a stronger model is justified, and what fallback exists.

Design goals:

- cost control: avoid sending large contexts blindly to expensive cloud models;
- local-first confidentiality: sensitive data stays local by default;
- capability routing: select resources according to task type and model/tool capability;
- budget awareness: track token, cost, latency and retry budgets;
- performance memory: remember which resources work well or fail for specific task types.

This layer is different from the Decision Gate. Compute Reservoir chooses how to think or process. Decision Gate decides whether an action may happen. Tool Registry declares available tools. Orchestrator coordinates the loop.

## 6. Target Architecture

The target runtime loop is:

User or human supervisor -> Mission Control -> Neutral Orchestrator -> Graph Memory recall -> Compute Reservoir allocation -> Tool Registry lookup -> Proposed Action -> Decision Gate -> approved, blocked, needs human approval or needs more context -> controlled execution or no execution -> observation, audit and graph update.

The architecture must remain modular, testable and observable.

## 7. Technical Direction

Current technical choices:

- Backend language: Rust;
- Backend framework: Axum;
- Frontend: Next.js and TypeScript;
- Dashboard: Mission Control web app;
- Main database: SurrealDB;
- Architecture: monorepo;
- Main agent: neutral orchestrator;
- API style: REST first, WebSocket later;
- Direction: local-first, graph-native and compute-aware.

Python workers may later be used for ingestion, OCR, document processing, embeddings experiments and AI/data tasks where Python is more productive.

## 8. What V0 Must Prove

V0 does not need to be fully autonomous or production-ready. V0 must prove that the architecture works.

A successful V0 demonstrates that a user can create an objective, the orchestrator creates a task, Graph Memory recalls applicable context, Compute Reservoir selects a resource, an agent proposes an action, Decision Gate evaluates it, the decision is recorded, audit shows the causal trace, Failure-to-Insight captures what was learned from non-trivial failures or corrections, and Mission Control makes the chain visible.

The audit proof is not satisfied by IDs alone. V0 must show at least one approved, one blocked and one pending or human-confirmation decision with complete explanatory audit fields, including explicit reasons and matched policy/fallback rules.

V0 must be ambitious but controlled. Governance comes before autonomy.

## 9. Development Priorities

Recommended implementation order:

1. Core Domain Types;
2. Decision Gate;
3. Compute Reservoir;
4. Tool Registry;
5. Graph Memory with SurrealDB;
6. Audit System;
7. Failure-to-Insight doctrine and bounded domain vocabulary;
8. Neutral Orchestrator;
9. API Server Axum;
10. Mission Control Web;
11. Scheduler and controlled autonomous loops;
12. LLM Provider abstraction;
13. End-to-end demo;
14. Security hardening.

## 10. Success Criteria

ARPAGONA Agent Core will be considered successful when it can maintain structured and invalidable memory, select the right compute resource for a task, prevent unsafe action execution, expose clear and explanatory decision traces, keep humans in control of sensitive actions, reduce unnecessary cloud LLM usage, combine local and cloud intelligence intelligently, make agent behavior observable in Mission Control, and support future professional deployments.

A clear decision trace means that `/evaluate action-x` followed by `/audit audit-decision-action-x` must expose the decision outcome, reason, policy/fallback rule and next recommended step. Audit entries that only show `decision_created`, `proposed_action_id` and `decision_id` are insufficient for governed alpha use.

## 11. Long-Term Direction

The long-term direction is to build a professional agentic operating layer usable as a personal/professional operating assistant, local enterprise AI runtime, document intelligence system, engineering assistant, R&D assistant, business workflow automation system, controlled coding agent runtime and eventually a multi-agent local-first company brain.

Autonomy must be earned, not assumed.

## 12. Guiding Sentence

ARPAGONA Agent Core is a local-first, graph-native, compute-aware runtime for controlled AI agents: agents may reason and propose, but the runtime governs what can actually happen.
