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

## Current storage boundary

`crates/core` owns the pure domain types and the synchronous `GraphMemoryStore` contract.

`crates/graph-memory` is the experimental SurrealDB adapter. It may persist audit events and list them by workspace, but it must not become an execution layer or orchestration layer.

Graph Memory and Audit are allowed to stabilize persistence, contracts, queryability and trace semantics. They are not allowed to execute tools or bypass the Decision Gate.

## Event conventions

Recommended audit event usage:

- `ActionProposed`: record that an agent produced a proposed action.
- `DecisionCreated`: record that the Decision Gate produced a decision for a proposed action.
- `HumanApprovalRequested`: record an escalation to human validation.
- `HumanApproved` / `HumanRejected`: record human validation outcomes.
- `ExecutionStarted` / `ExecutionSucceeded` / `ExecutionFailed`: reserved for future controlled execution after explicit validation. They must not be used to imply current execution capability.
- `PolicyChanged`: record changes to policy context.

## Alpha limits

Current limits:

- no stable causal trace schema yet;
- no dedicated query by proposed action or decision yet;
- no durable graph relation automatically linking audit events to decisions;
- no execution events should be produced by runtime/tool code because real execution is still deferred.

Recommended next stabilization steps:

1. Keep tests proving `DecisionCreated` audit events retain `proposed_action_id` and `decision_id` through persistence.
2. Clarify whether audit event query APIs should include lookup by proposed action and decision.
3. Decide whether causal trace relations should be represented as `GraphRelation` objects or remain embedded in `AuditEvent` fields for alpha.
4. Stabilize Audit before expanding Runtime/API/CLI surfaces.
