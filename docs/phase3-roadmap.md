# Phase 3 Roadmap — Neutral Orchestrator

Phase 3 starts after Phase 2 full delivery. Its job is not to add more raw capability. Its job is to coordinate the existing cognitive/runtime bricks into a governed, inspectable work cycle.

Founding rule remains unchanged:

```text
Build hard on internal cognitive architecture. Gate every external effect.
```

## GONA arbitration decision

GONA selected **Phase 3 roadmap definition first**, with **Neutral Orchestrator V0** as the first implementation milestone.

Rationale:
- Starting with broad Neutral Orchestrator implementation would be too large and risks becoming implicit autonomy.
- Integrating `compressed-cognitive-attention` directly into the runtime loop is premature until memory/context semantics are defined.
- A bounded roadmap/design PR gives DEEP a concrete queue and prevents Phase 3 from drifting into opportunistic feature work.

## Phase 3 objective

```text
A bounded Neutral Orchestrator that coordinates objective intake, context assembly, compute routing, proposal routing, Decision Gate outcomes and audit linkage without becoming an execution, approval or scheduler layer.
```

## Non-negotiable boundaries

The Neutral Orchestrator must not add:
- unrestricted shell;
- browser automation;
- email sending;
- secrets access;
- unrestricted file writes;
- hidden autonomy;
- scheduler expansion;
- Decision Gate bypass;
- readback-as-authorization behavior;
- direct memory writes from LLM output;
- direct approval from model output, memory recall, compute allocation or operator readback.

## Phase 3 milestone queue

### P3-0 — Roadmap and contract definition

Goal: document the Phase 3 queue and the Neutral Orchestrator V0 boundary.

Acceptance criteria:
- This roadmap exists.
- `AGENT_FOCUS_LOOP.md` points DEEP at Phase 3 instead of stale Phase 2 instructions.
- `FOCUS_LOOP_NEXT.md` names one concrete next action.
- No code changes are required.

### P3-1 — Neutral Orchestrator V0 domain contract

Goal: add the smallest pure domain contract for orchestrated work cycles.

Expected shape:

```text
ObjectiveInput
  -> OrchestratorContextRequest
  -> ContextBundle(advisory)
  -> ComputeRouteRequest
  -> ProposalRequest
  -> ProposedAction or ToolCallIntent
  -> Decision Gate
  -> Audit-linked OrchestratorOutcome
```

Required properties:
- pure serializable types first;
- no execution;
- no provider calls;
- no scheduler;
- no approval semantics;
- explicit IDs linking objective, context bundle, compute route, proposal, decision and audit event;
- tests proving that context, memory recall and compute route are advisory only.

### P3-2 — Neutral Orchestrator V0 deterministic loop skeleton

Goal: implement a deterministic in-process skeleton that wires existing bricks without external effects.

Required properties:
- accepts a bounded objective input;
- assembles a synthetic/advisory context bundle;
- requests or simulates compute route advice;
- creates a proposal request;
- sends any proposed action/tool-call intent through Decision Gate;
- records an audit-linked outcome;
- exposes readback data for CLI/MCP later;
- tests prove blocked, allowed-simulation and malformed paths.

### P3-3 — Read-only CLI/MCP readback for orchestration state

Goal: make orchestrator cycles inspectable by the operator and by external agent clients.

Required properties:
- read-only status/readback first;
- show objective ID, context bundle summary, compute route summary, proposal summary, Decision Gate result and audit IDs;
- no approve/reject buttons or mutation commands;
- no Web Mission Control expansion yet.

### P3-4 — Memory-aware context integration design

Goal: design how Graph Memory, Holographic Memory, Reservoir Echo and compressed-cognitive-attention contribute advisory context.

Required properties:
- recall/context is advisory only;
- no memory result authorizes action;
- compressed-cognitive-attention integration remains behind explicit contract boundaries;
- include tests or fixtures proving misleading recall cannot bypass governance.

### P3-5 — First product-facing orchestrated scenario

Goal: demonstrate a useful SME/business workflow through the Neutral Orchestrator, still locally and safely.

Candidate scenario:

```text
User objective -> bounded local documents/context -> advisory context bundle -> proposal/tool-call intent -> Decision Gate -> audit/readback -> operator-facing summary
```

Required properties:
- demoable from CLI first;
- no uncontrolled ingestion;
- no external effects;
- clear business value;
- clear audit/governance explanation.

## Deferred until after P3-1/P3-2

- Runtime integration of `compressed-cognitive-attention`.
- Mission Control Web expansion.
- Scheduler/autonomy work.
- Human approval semantics beyond design/readback.
- Any new external-effect capability.
