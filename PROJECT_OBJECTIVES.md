# ARPAGONA Agent Core — Project Objectives

This document is the canonical objective file for ARPAGONA Agent Core. It defines what we are building, why we are building it, and which principles must guide implementation. It must be read together with `WHITEPAPER.md`, `PROJECT_STATUS.md`, `docs/operating-doctrine.md`, `docs/development-acceleration.md`, `docs/holographic-memory.md` and `docs/failure-to-insight.md` before modifying the repository.

## 1. Primary Purpose

ARPAGONA Agent Core aims to become a **Rust-based cognitive agent runtime**.

The project is inspired by practical agent systems such as Hermes/OpenClaw in terms of local agent ergonomics, CLI usage, providers, tasks, inspectable state and future autonomous loops. However, ARPAGONA's ambition is deeper: build a local-first cognitive runtime with memory, continuity, pattern resonance, compute-aware reasoning, reflection and controlled self-improvement.

The project is not only a governance layer. Governance is necessary, but it is not the central identity. The central identity is cognitive capability.

The system exists to address weaknesses observed in current agentic systems:

- poor long-term memory;
- excessive context stuffing;
- weak continuity between cycles;
- insufficient learning from failures;
- limited recognition of recurring cognitive patterns;
- flat vector recall that ignores temporal neighborhood and episode continuity;
- limited routing between cloud models, local models and workers;
- lack of cost and data-sensitivity awareness;
- weak distinction between working memory, durable memory, temporary salience and pattern resonance;
- premature tool execution before cognition and reflection are stable.

## 2. Guiding Vision

ARPAGONA Agent Core must become a local-first runtime where agents can:

- receive objectives;
- maintain working context;
- use a Reservoir Echo for short-term cognitive continuity;
- use Holographic Memory for non-authorizing pattern resonance;
- use compressed convolutional memory retrieval for temporally enriched recall;
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

Stable typed vocabulary for agents, humans, workspaces, tasks, goals, actions, proposed actions, decisions, facts, tools, policies, risks, permissions, sources, observations, episodes, memory, cognitive traces, compute resources and failure insights.

### 4.2 Working Memory

Active context for the current cycle: objective, task, recalled context, recent observations, selected compute resource, constraints and active proposal.

### 4.3 Reservoir Echo

Short-term volatile cognitive continuity layer: pulses, activations, decay, reinforcement and salience across cycles. Reservoir Echo is not durable memory and not model routing.

### 4.4 Holographic Memory

Experimental cognitive resonance layer. It stores distributed pattern signatures of episodes, tasks, action chains, failures, successes, conversations, routing choices and decisions.

Holographic Memory answers: what does this situation resemble?

It may influence recall, caution, compute routing and reflection. It must never authorize actions or replace Graph Memory.

### 4.5 Compressed Convolutional Memory Retrieval

Experimental temporally enriched recall layer inspired by Compressed Convolutional Attention, applied to agent memory rather than to a Transformer implementation.

It must project memory-event embeddings into a smaller latent space, enrich each memory event with a local temporal convolution over neighboring events, then compute attention-like similarity between the current query and the enriched latent memories.

Compressed Convolutional Memory Retrieval answers: what memories are relevant when each memory is interpreted with its local temporal neighborhood?

It must support:

- deterministic compressed projection;
- local temporal convolution over ordered memory events;
- attention-like scoring between a query and enriched memories;
- top-k retrieval with scores, ranks and minimal explanation;
- empty-memory and invalid-dimension safeguards;
- future integration with Graph Memory, Working Memory and Decision Gate readback.

It must remain non-authoritative:

- it does not authorize actions;
- it does not replace Graph Memory;
- it does not replace Holographic Memory;
- it does not execute tools;
- it does not require modifying a base LLM architecture.

The first implementation target is an isolated Rust crate, likely `crates/compressed-cognitive-attention`, proving the retrieval mechanism before integration.

### 4.6 Graph Memory

Structured durable memory with facts, entities, relations, sources, episodes, observations, decisions, failures, validity, confidence, provenance and invalidation.

Core primitives:

- remember;
- relate;
- recall;
- invalidate;
- consolidate.

### 4.7 Compute Reservoir

Cognitive resource router. It selects which resource should think, read, summarize, parse or draft: strong cloud LLM, local LLM, embedding model, deterministic worker, GPU, CPU, deferred job or fallback.

It considers capability, cost, latency, sensitivity, local-first constraints, quality requirements and past performance.

### 4.8 Reflection and Failure-to-Insight

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

### 4.9 Decision Gate

Deterministic boundary for risky or external actions. It evaluates proposed actions against context, risk, permissions and policies. It is a guardrail, not the core cognitive engine.

### 4.10 Tool Registry

Declarative catalogue of tools, schemas, capabilities, permissions and risks. Tool lookup is not authorization.

### 4.11 Neutral Orchestrator

The coordinator that turns objectives into tasks, recalls context, requests compute allocation, asks for proposals, routes decisions and records outcomes.

### 4.12 Audit and Mission Control

Audit and Mission Control make the cognitive runtime observable: what happened, why, what was recalled, what resonated, what was proposed, what was decided, what failed and what was learned.

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

## 7. Holographic Memory Objective

Holographic Memory is central to the long-term cognitive ambition because Graph Memory alone is explicit and source-aware, while Reservoir Echo is short-term and volatile.

Holographic Memory fills a different role: pattern resonance.

It must help the runtime detect when a current cycle resembles past episodes, failures, successes, action chains, conversation patterns or compute routing choices.

It must remain non-authoritative:

```text
Graph Memory gives explicit memory.
Reservoir Echo gives short-term continuity.
Holographic Memory gives pattern resonance.
Compressed Convolutional Memory Retrieval gives temporally enriched recall.
Compute Reservoir gives cognitive resource selection.
Reflection gives self-improvement.
```

## 8. Compressed Convolutional Memory Retrieval Objective

Compressed Convolutional Memory Retrieval is an experimental memory-selection objective for improving recall quality without stuffing the full historical context into the LLM.

The target mechanism is:

```text
Ordered memory events
-> compressed latent projection
-> local temporal convolution
-> query-to-memory attention-like scoring
-> top-k enriched memories
-> explicit readback and audit-friendly explanation
```

Its purpose is to make recall more episode-aware. A memory event should not be scored only as an isolated embedding: its neighboring events may change its meaning, importance or usefulness.

This layer must initially be implemented as a standalone, deterministic Rust experiment. It should prefer a simple, inspectable implementation over neural complexity:

- fixed or deterministic projection from embedding dimension to latent dimension;
- configurable latent dimension;
- configurable local window size;
- edge-safe convolution with weight renormalization;
- cosine scoring;
- sorted top-k retrieval;
- no GPU dependency;
- no LLM call;
- no persistent mutation;
- no authorization semantics.

The first useful proof is not intelligence by itself. The first useful proof is whether temporally enriched recall produces better context candidates than flat similarity search for agentic traces, failures, PR histories, decision chains and conversation episodes.

## 9. Target Cognitive Runtime Loop

Target loop:

```text
User objective
-> intent parsing
-> working memory
-> Reservoir Echo
-> Holographic Memory recall
-> Compressed Convolutional Memory Retrieval
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

## 10. Technical Direction

Current direction:

- Backend language: Rust;
- Backend framework: Axum;
- Frontend: Next.js and TypeScript;
- Dashboard: Mission Control web app;
- Main database: SurrealDB;
- Architecture: monorepo;
- Main agent: neutral orchestrator;
- API style: REST first, WebSocket later;
- Direction: local-first, graph-native, cognitive-runtime-first, holographic-memory-aware, compressed-retrieval-aware and compute-aware.

Python workers may later be used for ingestion, OCR, document processing, embeddings experiments and AI/data tasks where Python is more productive.

## 11. What V0 Must Prove

V0 does not need to be production-ready. It must prove the cognitive architecture in miniature.

A successful V0 demonstrates:

1. a user creates an objective;
2. the runtime creates or updates a task;
3. working memory is formed;
4. Reservoir Echo preserves short-term salience;
5. Holographic Memory can surface non-authorizing resonance patterns;
6. Compressed Convolutional Memory Retrieval can surface temporally enriched context candidates;
7. Graph Memory recalls applicable context;
8. Compute Reservoir selects a resource;
9. an agent proposes an action;
10. Decision Gate evaluates sensitive/risky actions;
11. audit records the chain;
12. Failure-to-Insight captures what was learned;
13. the operator can inspect the chain locally.

The key proof is cognitive continuity plus controlled progression.

## 12. Development Priorities

Recommended consolidation order:

1. Core cognitive/domain vocabulary;
2. Reservoir Echo and cognitive cycle primitives;
3. Graph Memory and readback;
4. Holographic Memory documentation and later experiments;
5. Compressed Convolutional Memory Retrieval experiment;
6. Decision Gate as safety boundary;
7. Compute Reservoir;
8. Tool Registry;
9. CLI supervision as first Mission Control;
10. Reflection / Failure-to-Insight;
11. Neutral Orchestrator;
12. API server integration;
13. Mission Control Web;
14. Scheduler and controlled loops;
15. Tool Runtime and sandbox;
16. Security hardening;
17. End-to-end cognitive alpha.

## 13. Success Criteria

ARPAGONA Agent Core will be successful when it can:

- maintain working memory and short-term cognitive continuity;
- detect non-authorizing pattern resonance through Holographic Memory;
- retrieve temporally enriched context through Compressed Convolutional Memory Retrieval;
- maintain structured and invalidable Graph Memory;
- route cognition across local/cloud/workers intelligently;
- propose useful actions without direct execution;
- learn from failures and human corrections;
- expose clear causal traces;
- stay local-first where appropriate;
- become progressively more useful without unsafe self-modification.

## 14. Long-Term Direction

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

## 15. Guiding Sentence

ARPAGONA Agent Core is a Rust-based cognitive agent runtime: a Hermes-like local-first system designed for memory, continuity, holographic resonance, compressed convolutional recall, reflection, compute-aware reasoning and controlled self-improvement — with governance as the safety layer that makes this ambition usable.
