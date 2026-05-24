# ARPAGONA Agent Core — Project Objectives

This document is the canonical objective file for ARPAGONA Agent Core. It defines what we are building, why we are building it, and which principles must guide implementation. It must be read together with `WHITEPAPER.md`, `PROJECT_STATUS.md`, `docs/operating-doctrine.md`, `docs/development-acceleration.md` and `docs/failure-to-insight.md` before modifying the repository.

## 1. Primary Purpose

ARPAGONA Agent Core aims to become a **Rust-based cognitive agent runtime**.

The project is inspired by practical agent systems such as Hermes/OpenClaw in terms of local agent ergonomics, CLI usage, providers, tasks, inspectable state and future autonomous loops. However, ARPAGONA's ambition is deeper: build a local-first cognitive runtime with memory, continuity, compute-aware reasoning, reflection and controlled self-improvement.

The project is not only a governance layer. Governance is necessary, but it is not the central identity. The central identity is cognitive capability.

The system exists to address weaknesses observed in current agentic systems:

- poor long-term memory;
- excessive context stuffing;
- weak continuity between cycles;
- insufficient learning from failures;
- limited routing between cloud models, local models and workers;
- lack of cost and data-sensitivity awareness;
- weak distinction between working memory, durable memory and temporary salience;
- premature tool execution before cognition and reflection are stable.

## 2. Guiding Vision

ARPAGONA Agent Core must become a local-first runtime where agents can:

- receive objectives;
- maintain working context;
- use a Reservoir Echo for short-term cognitive continuity;
- recall structured Graph Memory;
- route tasks through a Compute Reservoir;
- propose actions;
- reflect on outcomes;
- turn failures into durable insights;
- progressively improve memory, routing, policies, tools and tests;
- remain observable and human-governed.

The core sentence is:

```text
Cognitive ambition first. Governance as the immune system.
```

## 3. Foundational Safety Principle

The safety principle remains non-negotiable:

```text
No agent executes directly.
Agents only propose actions.
Every sensitive action passes through the Decision Gate.
Every important decision is recorded in the graph or audit trail.
Every structural improvement remains controlled.
```

In French:

```text
Aucun agent n'agit directement.
Un agent propose une action.
Le Decision Gate évalue les actions sensibles.
Toute décision importante est tracée.
Toute amélioration structurelle sensible reste contrôlée.
```

This principle enables autonomy. It must not reduce the project to audit/governance only.

## 4. Core Cognitive Objectives

ARPAGONA Agent Core must provide the following core systems.

### 4.1 Core Domain

Stable typed vocabulary for agents, humans, workspaces, tasks, goals, actions, proposed actions, decisions, facts, tools, policies, risks, permissions, sources, observations, episodes, memory, compute resources and failure insights.

### 4.2 Working Memory

Active context for the current cycle: objective, task, recalled context, recent observations, selected compute resource, constraints and active proposal.

### 4.3 Reservoir Echo

Short-term volatile cognitive continuity layer: pulses, activations, decay, reinforcement and salience across cycles. Reservoir Echo is not durable memory and not model routing.

### 4.4 Graph Memory

Structured durable memory with facts, entities, relations, sources, episodes, observations, decisions, failures, validity, confidence, provenance and invalidation.

Core primitives:

- remember;
- relate;
- recall;
- invalidate;
- consolidate.

### 4.5 Compute Reservoir

Cognitive resource router. It selects which resource should think, read, summarize, parse or draft: strong cloud LLM, local LLM, embedding model, deterministic worker, GPU, CPU, deferred job or fallback.

It considers capability, cost, latency, sensitivity, local-first constraints, quality requirements and past performance.

### 4.6 Reflection and Failure-to-Insight

The system must observe its own cycles and convert failures, bad proposals, missing context, poor routing, blocked decisions and human corrections into durable, non-authorizing insights.

These insights can propose improvements to:

- memory;
- policies;
- tests;
- prompts;
- documentation;
- compute routing;
- tools;
- operating doctrine.

They do not authorize execution or self-modification.

### 4.7 Decision Gate

Deterministic boundary for risky or external actions. It evaluates proposed actions against context, risk, permissions and policies. It is a guardrail, not the core cognitive engine.

### 4.8 Tool Registry

Declarative catalogue of tools, schemas, capabilities, permissions and risks. Tool lookup is not authorization.

### 4.9 Neutral Orchestrator

The coordinator that turns objectives into tasks, recalls context, requests compute allocation, asks for proposals, routes decisions and records outcomes.

### 4.10 Audit and Mission Control

Audit and Mission Control make the cognitive runtime observable: what happened, why, what was recalled, what was proposed, what was decided, what failed and what was learned.

## 5. Rippletide-Inspired Method

ARPAGONA Agent Core is conceptually inspired by Rippletide-like runtime enforcement, but this is a supporting safety layer.

The useful lesson is:

```text
A capable agent needs a deterministic boundary between intention and effect.
```

Relevant patterns:

- pre-execution enforcement;
- context graph;
- applicable context rather than merely similar context;
- explicit policies;
- causal audit;
- invalidation of obsolete information.

The goal is not to build only a compliance runtime. The goal is to let a more cognitive agent become useful without becoming unsafe or opaque.

## 6. Compute Reservoir Objective

The Compute Reservoir is central because ARPAGONA Agent Core must not depend on one abstract model.

It must manage and route between:

- GPT-class cloud models;
- local Ollama models;
- embedding models;
- OCR/parsing workers;
- deterministic tools;
- local GPU/CPU;
- deferred jobs;
- fallback paths.

It must answer:

- what resource should process this task?
- why this resource?
- expected cost?
- expected latency?
- data sensitivity compatibility?
- local model sufficient?
- strong model justified?
- fallback available?

This is a cognitive routing layer, not just a cost optimization layer.

## 7. Target Cognitive Runtime Loop

Target loop:

```text
User objective
-> intent parsing
-> working memory
-> Reservoir Echo
-> Graph Memory recall
-> Compute Reservoir allocation
-> agent proposal
-> Decision Gate if needed
-> human boundary if needed
-> observation
-> audit
-> reflection
-> Failure-to-Insight / memory consolidation
```

The architecture must remain modular, testable and observable.

## 8. Technical Direction

Current direction:

- Backend language: Rust;
- Backend framework: Axum;
- Frontend: Next.js and TypeScript;
- Dashboard: Mission Control web app;
- Main database: SurrealDB;
- Architecture: monorepo;
- Main agent: neutral orchestrator;
- API style: REST first, WebSocket later;
- Direction: local-first, graph-native, cognitive-runtime-first and compute-aware.

Python workers may later be used for ingestion, OCR, document processing, embeddings experiments and AI/data tasks where Python is more productive.

## 9. What V0 Must Prove

V0 does not need to be production-ready. It must prove the cognitive architecture in miniature.

A successful V0 demonstrates:

1. a user creates an objective;
2. the runtime creates or updates a task;
3. working memory is formed;
4. Reservoir Echo preserves short-term salience;
5. Graph Memory recalls applicable context;
6. Compute Reservoir selects a resource;
7. an agent proposes an action;
8. Decision Gate evaluates sensitive/risky actions;
9. audit records the chain;
10. Failure-to-Insight captures what was learned;
11. the operator can inspect the chain locally.

The key proof is cognitive continuity plus controlled progression.

## 10. Development Priorities

Recommended consolidation order:

1. Core cognitive/domain vocabulary;
2. Reservoir Echo and cognitive cycle primitives;
3. Graph Memory and readback;
4. Decision Gate as safety boundary;
5. Compute Reservoir;
6. Tool Registry;
7. CLI supervision as first Mission Control;
8. Reflection / Failure-to-Insight;
9. Neutral Orchestrator;
10. API server integration;
11. Mission Control Web;
12. Scheduler and controlled loops;
13. Tool Runtime and sandbox;
14. Security hardening;
15. End-to-end cognitive alpha.

## 11. Success Criteria

ARPAGONA Agent Core will be successful when it can:

- maintain working memory and short-term cognitive continuity;
- maintain structured and invalidable Graph Memory;
- route cognition across local/cloud/workers intelligently;
- propose useful actions without direct execution;
- learn from failures and human corrections;
- expose clear causal traces;
- stay local-first where appropriate;
- become progressively more useful without unsafe self-modification.

## 12. Long-Term Direction

The long-term direction is to build a local-first cognitive operating layer usable as:

- personal/professional operating assistant;
- AI research assistant;
- engineering assistant;
- development assistant;
- document intelligence system;
- business workflow assistant;
- controlled coding runtime;
- multi-agent cognitive workspace;
- local company brain.

Autonomy must be earned, not assumed.

## 13. Guiding Sentence

ARPAGONA Agent Core is a Rust-based cognitive agent runtime: a Hermes-like local-first system designed for memory, continuity, reflection, compute-aware reasoning and controlled self-improvement — with governance as the safety layer that makes this ambition usable.
