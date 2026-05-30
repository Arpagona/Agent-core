# ARPAGONA — Steroid Hermes Roadmap

Status: GONA strategic reset, 2026-05-30.

## Product thesis

ARPAGONA is not a pretty CLI and not a governance toy. It must become a local-first cognitive runtime: Hermes/OpenClaw ergonomics, but with deeper memory, continuity, compute routing, reflection, auditability and controlled self-improvement.

Core sentence:

```text
Premium operator experience on top of a real governed cognitive loop.
```

## Non-negotiables

1. No façade PRs.
   - A UX surface must expose or activate real runtime state.
   - Pretty output without `CycleTrace`, `WorkingMemory`, `Decision`, `AuditEvent`, `ComputeDecision` or memory signal behind it is not enough.

2. Read/think/plan freely; gate effects.
   - Read-only inspection is default.
   - Mutation, publication, external sending, destructive commands and hidden autonomy require explicit human/Decision Gate approval.

3. One coherent runtime brick at a time.
   - No branch sprawl.
   - No duplicate PRs for the same idea.
   - Fix mergeability/checks before starting new feature work.

4. Operator trust comes from proof.
   - Every major feature needs tests and a reproducible smoke command.
   - The operator should see: what was understood, what context was used, why the resource was chosen, what was proposed, what was decided, what was learned.

## Current reality check

The project is not hollow. Existing objectives and docs define real subsystems:

- Neutral Orchestrator
- Working Memory
- Compute Reservoir
- Decision Gate
- Tool Registry / Tool Runtime / MCP
- Audit and Mission Control
- Holographic Memory
- Compressed Cognitive Attention retrieval
- Failure-to-Insight reflection
- CLI/MCP readback surfaces

But the danger is real: if we keep adding attractive CLI wrappers before tightening the end-to-end loop, we create a façade. The next work must prove runtime depth.

## Immediate P0 — stabilize before acceleration

Observed on 2026-05-30:

- PR #220 (`feat/steroid-hermes-ux-alpha`) exists and is `UNSTABLE` on GitHub checks.
- PR #211-#216 have `SUCCESS` checks but are currently `DIRTY` against `main`.
- `FOCUS_LOOP_NEXT.md` still says the P3 stack awaits GONA merge, but current GitHub mergeability says the stack now needs rebase/repair before it can be merged cleanly.

P0 order:

1. Inspect and fix PR #220 check status.
2. Re-evaluate/rebase PR #211 -> #216 in stack order, or mark superseded with evidence.
3. Only then start P3-27.

## Roadmap: four execution lanes

### Lane A — Mission Loop V1

Goal: make `arpagona run` / `arpagona chat` a real local mission loop, not just output formatting.

Target chain:

```text
Objective
-> WorkingMemory
-> context assembly
-> Compute Reservoir route
-> model/proposal/tool-intent
-> Decision Gate
-> observation/audit
-> concise operator readback
-> trace saved for replay
```

Acceptance criteria:

- works without API server for local alpha scenarios;
- emits/uses a real `CycleTrace`;
- shows concise operator output by default;
- `--json` or inspect command exposes structured internals;
- no direct mutation without gate.

### Lane B — Audit Spine

Goal: make every important cycle inspectable and replayable.

Next brick:

- `audit list-from-dir` for saved audit events;
- `orchestrator run --save-audit` connected to trace/audit persistence;
- `orchestrator cycles --json` includes audit event type breakdown.

Acceptance criteria:

- saved audit files can be listed independently;
- tests prove empty, malformed and populated audit dirs;
- readback says clearly what happened and what did not authorize execution.

### Lane C — Memory-aware Context

Goal: use memory as cognitive advantage, not as decorative text.

Integrations:

- Graph Memory for durable facts and provenance;
- Holographic Memory for pattern resemblance;
- Compressed Cognitive Attention for temporal-neighborhood recall;
- Failure-to-Insight for correction loops.

Acceptance criteria:

- each memory signal is advisory and non-authorizing;
- operator output says which memory/context influenced the plan;
- tests prove memory absence, stale memory and conflicting memory are handled safely.

### Lane D — Operator Mission Control

Goal: premium, high-trust operator surface.

CLI/MCP should answer:

- what mission is active?
- what did the system understand?
- what context did it use?
- which compute resource was chosen and why?
- what is blocked on approval?
- what is the next safe action?
- what did we learn from the last failure?

Acceptance criteria:

- no governance spam by default;
- detail available on demand;
- every displayed state has a real backend source;
- useful for Thibaud as an operator, not just for developers.

## Benchmark implications from adjacent tools

Publicly visible agent CLI patterns converge on:

- continuous progress feedback;
- explicit diff/preview before changes;
- dry-run/simulation before mutation;
- clear approval boundary;
- inspectable logs/traces;
- local-first operation when possible;
- strong defaults with expert detail available on demand.

Anti-patterns to avoid:

- hidden autonomy;
- model deciding authorization;
- vague “AI is thinking” UX with no trace;
- pretty banners without cycle substance;
- too much governance jargon in normal operator output;
- branch/PR sprawl that makes velocity look higher than it is.

## DEEP operating directive

DEEP must optimize for fewer, stronger PRs:

1. First stabilize current open work.
2. Then ship one runtime brick per cycle.
3. Every brick must have a CLI/MCP proof path and tests.
4. Every report must state: capability added, real subsystem touched, validation run, remaining risk.

Short version:

```text
No more cosmetic acceleration. Ship the spine.
```
