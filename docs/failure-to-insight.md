# Failure-to-Insight Loop

This document defines the canonical failure-to-insight mechanism for ARPAGONA Agent Core.

The goal is to make failures useful. A failure must not only be logged; it must become structured learning that can improve future decisions, routing, documentation, tests and operating doctrine.

## 1. Purpose

ARPAGONA Agent Core must progressively transform execution failures, blocked decisions, incorrect proposals, missing context, bad routing, tool mismatch, policy gaps and human corrections into durable insights.

A failure-to-insight loop is the bridge between:

- raw audit events;
- human-readable post-mortems;
- graph memory updates;
- regression tests;
- policy improvements;
- better Compute Reservoir routing;
- safer Decision Gate behavior;
- better future agent loops.

This is a first-class requirement, not a later analytics feature.

## 2. Definition

A `FailureInsight` is a structured record extracted from a failed, blocked, degraded, ambiguous or corrected agent loop.

It should answer:

- what failed?
- where did it fail?
- why did it fail?
- what signal revealed the failure?
- what was the impact?
- what should be changed?
- which layer owns the correction?
- how can we detect the same class of failure earlier next time?

## 3. Failure classes

The system should distinguish at least these classes:

| Class | Meaning | Typical owner |
|---|---|---|
| `missing_context` | The agent lacked necessary context or recalled the wrong context. | Graph Memory / Recall |
| `stale_context` | The agent used obsolete, superseded or invalid context. | Graph Memory / Invalidation |
| `bad_action_type` | The agent proposed the wrong action type or too generic a proposal. | LLM provider / Router |
| `policy_gap` | The Decision Gate lacked a clear policy for the situation. | Decision Gate / Policy |
| `blocked_without_explanation` | A decision was blocked or escalated without enough human-readable reason. | Audit / Decision Gate |
| `wrong_compute_choice` | The selected model, worker or resource was too weak, too expensive or unsuitable. | Compute Reservoir |
| `tool_mismatch` | The selected tool was inappropriate, disabled, under-specified or missing permissions. | Tool Registry |
| `unsafe_drift` | A component moved toward execution, autonomy or authorization outside the governed path. | Architecture / Review |
| `insufficient_observability` | The human cannot reconstruct what happened from CLI/Audit/Mission Control. | Audit / CLI |
| `test_gap` | A failure class exists without a regression test. | Tests / CI |
| `documentation_gap` | Contributors or agents lack a written rule to avoid repeating the error. | Docs / Doctrine |

## 4. Required lifecycle

Every significant failure or correction should follow this lifecycle:

```text
Failure observed
-> audit event or human correction
-> classify failure
-> extract root cause
-> create FailureInsight
-> decide correction target
-> update code, test, policy, memory or documentation
-> link back to audit/PR/session
-> make the next loop aware of the insight
```

The loop is not complete until at least one durable artifact has changed or a deliberate `no_change` reason has been recorded.

## 5. Minimum fields

A future domain type should include fields equivalent to:

```text
id
source_event_id
workspace_id
task_id
proposed_action_id
decision_id
failure_class
severity
summary
root_cause
impact
corrective_action
owner_layer
confidence
created_at
status
linked_pr
linked_test
linked_doc
```

This document does not require the type to exist immediately, but future implementation should align with this schema.

## 6. Integration requirements

### Audit

Audit must preserve enough context for later failure extraction. A failure insight must be traceable back to the event, decision, action, task or human correction that caused it.

### Graph Memory

Durable insights should eventually be stored in Graph Memory as structured knowledge with provenance, confidence and invalidation rules.

### Decision Gate

Repeated policy gaps or unclear blocks must become explicit Decision Gate rules, better reasons or stronger fallback behavior.

### Compute Reservoir

Wrong resource choices should update performance memory: model/tool suitability, cost, latency, quality and failure patterns by task type.

### Tool Registry

Tool mismatch should update tool metadata, required permissions, risk levels, enabled/disabled state or schema clarity.

### CLI / Mission Control

Human supervision surfaces should expose not only what failed, but what was learned and what changed afterward.

### Focus Loop

Every non-trivial focus loop report should include:

```text
Failures observed: yes/no
Failure insights created: yes/no
If no, why not?
Correction target: code/test/policy/memory/docs/none
Next detection signal:
```

## 7. Near-term implementation path

Do not start with autonomous self-improvement.

Recommended sequence:

1. document the doctrine;
2. add a simple `FailureInsight` domain vocabulary;
3. add audit event conventions for failure extraction;
4. expose read-only CLI summaries of failures and insights;
5. add regression tests for recurring failure classes;
6. connect insights to Graph Memory only after the vocabulary is stable;
7. later use insights to influence Compute Reservoir routing and Decision Gate policy refinement.

## 8. Non-goals for alpha

Do not implement yet:

- automatic code self-modification;
- autonomous policy rewrites;
- unreviewed memory mutation by LLM;
- automatic merge based on a failure insight;
- hidden optimizer loops that change behavior without audit;
- treating a failure insight as authorization.

A failure insight informs future behavior. It does not approve action.

## 9. Success criterion

A human or future agent should be able to ask:

```text
What failed in the last loop, what did we learn, what was changed, and how will we detect the same failure earlier next time?
```

If the system cannot answer that from durable artifacts, the failure-to-insight loop is not yet implemented.
