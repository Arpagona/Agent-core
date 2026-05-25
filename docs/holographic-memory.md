# Holographic Memory

Holographic Memory is an experimental cognitive memory layer for ARPAGONA Agent Core.

It complements Graph Memory and Reservoir Echo. It does not replace them.

## Role

Holographic Memory stores distributed pattern signatures of cognitive experience:

- episodes;
- tasks;
- action chains;
- failures;
- successes;
- conversations;
- compute routing choices;
- decision patterns;
- tool-use patterns;
- full cognitive cycles.

Its purpose is to help the runtime detect resonance, similarity and recurring structures across time.

It should answer questions such as:

```text
Does this task resemble a previous failure?
Does this action chain resemble a successful cycle?
Does this context resemble a situation where the Decision Gate blocked something?
Does this compute routing choice resemble a past poor model selection?
Does this conversation pattern indicate a known drift risk?
```

## Non-Authoritative Memory

Holographic Memory is not a source of truth.

It must never say:

```text
This is true.
This is authorized.
This action should execute.
```

It may only say:

```text
This resembles a pattern already encountered.
This context resonates with previous episodes.
This situation may deserve recall, caution, reflection or routing adjustment.
```

All authoritative facts remain in Graph Memory. All action authorization remains under the Decision Gate.

## Difference From Other Memory Layers

```text
Working Memory       = active context of the current cycle.
Reservoir Echo       = volatile short-term salience and activation decay.
Holographic Memory   = distributed signatures of recurring cognitive patterns.
Graph Memory         = explicit source-aware facts, relations, episodes and decisions.
Compute Reservoir    = cognitive resource routing.
Decision Gate        = action safety boundary.
```

## Intended Position in the Cognitive Loop

```text
Input
-> Intent Parsing
-> Working Memory
-> Reservoir Echo
-> Holographic Memory recall
-> Graph Memory recall
-> Compute Reservoir allocation
-> Agent Proposal
-> Decision Gate if needed
-> Observation
-> Audit
-> Reflection / Failure-to-Insight
-> Memory consolidation
```

Holographic Memory should influence recall, caution, routing and reflection. It should not directly produce actions.

## Future Domain Concepts

Potential future types:

```text
HolographicTrace
- id
- workspace_id
- source_episode_id
- trace_kind
- vector
- labels
- strength
- decay
- created_at
- updated_at

HolographicPattern
- id
- pattern_kind
- prototype_vector
- support_count
- confidence
- last_matched_at

HolographicQuery
- query_vector
- top_k
- min_similarity
- workspace_scope

HolographicMatch
- trace_id
- similarity
- matched_labels
- linked_episode_id
```

Potential trace kinds:

```text
TaskPattern
ActionChainPattern
FailurePattern
SuccessPattern
ConversationPattern
ComputeRoutingPattern
DecisionPattern
ToolUsePattern
```

## V0 Direction

A minimal future V0 should remain simple:

- encode an episode or cognitive cycle into a fixed-width vector;
- store the signature;
- compare by similarity;
- return non-authorizing resonance matches;
- link matches back to Graph Memory episodes, facts or audit events;
- expose matches through CLI/Mission Control as evidence only.

V0 must not add broad runtime behavior or action authority.

## V1 Direction

A later V1 may experiment with:

- role/value binding;
- superposition;
- sequence encoding;
- failure-pattern prototypes;
- success-pattern prototypes;
- links to Reflection and Failure-to-Insight;
- influence on Compute Reservoir routing.

## Safety Boundaries

Holographic Memory must not:

- replace Graph Memory;
- replace Reservoir Echo;
- replace Compute Reservoir;
- replace Decision Gate;
- authorize actions;
- store secrets;
- become hidden policy;
- become hidden self-modification;
- produce execution decisions.

It is a cognitive resonance layer, not a governance layer and not an execution layer.

## Guiding Sentence

```text
Graph Memory gives ARPAGONA explicit memory.
Reservoir Echo gives ARPAGONA short-term continuity.
Holographic Memory gives ARPAGONA pattern resonance.
Compute Reservoir gives ARPAGONA cognitive resource selection.
Reflection gives ARPAGONA self-improvement.
```
