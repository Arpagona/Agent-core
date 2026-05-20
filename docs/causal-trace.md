# Causal Trace Conventions

This document defines the current alpha conventions for linking proposed actions, decisions and audit events in ARPAGONA Agent Core.

The goal is traceability only. These conventions do not authorize execution, tool calls, scheduler autonomy, API/CLI orchestration or runtime growth.

## Governed flow

The non-negotiable flow remains:

```text
Agent -> ProposedAction -> DecisionGate -> Decision -> Audit
```

Future controlled execution, when explicitly validated later, must remain downstream of this chain:

```text
ProposedAction -> ToolRegistry lookup -> DecisionGate -> Human approval if needed -> Controlled execution -> Audit -> Graph update
```

## Minimal causal trace

A causal trace should make the following links explicit:

- the proposed action being evaluated;
- the decision produced by the Decision Gate;
- the audit event recording that decision;
- the workspace and task context when known;
- the actor that produced the trace;
- the reason and policies or context used when available.

Current alpha representation:

- `ProposedAction.id` identifies the proposed action.
- `Decision.proposed_action_id` links a decision to the proposed action it evaluates.
- `AuditEvent.proposed_action_id` links an audit event to the proposed action.
- `AuditEvent.decision_id` links an audit event to the decision.
- `AuditEvent.workspace_id` and `AuditEvent.task_id` preserve workspace/task scope when known.
- `AuditEvent.payload` may carry structured alpha metadata such as reason, policy ids, context refs or a `causal_trace` object.
- `AuditEvent::decision_created` is the Rust-first helper for constructing canonical alpha `DecisionCreated` events with queryable `proposed_action_id`, `decision_id`, workspace/task scope and a structured `payload.causal_trace`.

## Current storage boundary

`crates/core` owns the pure domain types and the synchronous `GraphMemoryStore` contract.

`crates/graph-memory` is the experimental SurrealDB adapter. It may persist audit events and list them by workspace, task, proposed action or decision, but it must not become an execution layer or orchestration layer.

Graph Memory and Audit are allowed to stabilize persistence, contracts, queryability and trace semantics. They are not allowed to execute tools or bypass the Decision Gate.

## Event conventions

Recommended audit event usage:

- `ActionProposed`: record that an agent produced a proposed action.
- `DecisionCreated`: record that the Decision Gate produced a decision for a proposed action.
- `HumanApprovalRequested`: record an escalation to human validation.
- `HumanApproved` / `HumanRejected`: record human validation outcomes.
- `ExecutionStarted` / `ExecutionSucceeded` / `ExecutionFailed`: reserved for future controlled execution after explicit validation. They must not be used to imply current execution capability.
- `PolicyChanged`: record changes to policy context.

## Alpha decision: embedded audit links first

For the alpha, causal trace links remain embedded in `AuditEvent` fields and are queryable through audit-event lookup methods.

The alpha canonical links are:

- `Decision.proposed_action_id` for the decision-to-proposed-action link;
- `AuditEvent.proposed_action_id` for the audit-event-to-proposed-action link;
- `AuditEvent.decision_id` for the audit-event-to-decision link;
- `AuditEvent.workspace_id` and `AuditEvent.task_id` for scope.

Do not automatically mirror these links as durable `GraphRelation` objects yet.

Rationale:

- the current query need is already covered by dedicated audit-event queries;
- `GraphRelation` semantics are still broad and should not be overloaded before the graph schema stabilizes;
- automatic mirroring would create duplicate sources of truth between Audit fields and graph edges;
- the alpha priority is traceability and governance stabilization, not graph expressiveness;
- keeping the links embedded avoids accidental orchestration or authorization logic being built on graph edges too early.

A future stabilization pass may introduce explicit `GraphRelation` mirrors only after these questions are answered:

- which relation names are canonical for audit causality;
- whether graph edges are derived projections or independent durable records;
- how duplication, deletion, migration and reindexing are handled;
- which component owns creation of those relations;
- whether Mission Control needs graph-edge traversal beyond audit-event queries.

## Alpha limits

Current limits:

- no stable causal trace schema yet;
- dedicated query by task, proposed action and decision exists for audit events, but remains alpha;
- no durable graph relation automatically linking audit events to decisions, proposed actions or tasks;
- no execution events should be produced by runtime/tool code because real execution is still deferred.

Recommended next stabilization steps:

1. Keep tests proving `DecisionCreated` audit events retain `proposed_action_id` and `decision_id` through persistence.
2. Keep tests proving audit events can be queried by task, proposed action and decision without introducing execution.
3. Stabilize Audit field semantics and payload conventions while keeping causal trace links embedded for alpha.
4. Revisit `GraphRelation` mirroring only after the audit schema and graph relation vocabulary are stable.
5. Stabilize Audit before expanding Runtime/API/CLI surfaces.
