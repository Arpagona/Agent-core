# General Cognitive Work Loop V0

## Why Agent Core Is Not Only a Developer Agent

Agent Core is designed to become a general-purpose professional agentic nucleus. A
pure "developer agent" writes code, runs tests, and pushes commits. A
general-purpose professional agent can also prepare business strategies, design
courses, conduct research, organize administration, improve personal
productivity, escalate to humans, and — crucially — identify its own
improvement candidates.

The General Cognitive Work Loop is the first step toward that vision.

## What Is a General Cognitive Work Loop?

A cognitive work loop is the minimal cycle an agent runs to turn an objective
into actionable structure. It mirrors how a human professional approaches a new
assignment: understand the goal, recall relevant context, note assumptions and
constraints, identify what is missing, plan a sequence of work, determine the
next step, and reflect on what could be improved.

The loop is:

```
Objective
  → WorkingMemory (context, assumptions, constraints, gaps)
  → CognitivePlan (ordered steps)
  → RequiredObservations (what needs to be checked)
  → ProposedNextAction (what to do next)
  → ImprovementCandidates (learning signals)
```

## What V0 Does

- Defines all pure domain types (Objective, WorkingMemory, CognitivePlan,
  RequiredObservation, ProposedNextAction, ImprovementCandidate).
- Provides a heuristic, deterministic engine that processes any text objective
  and optional context with zero LLM calls.
- Classifies domain heuristically (Business, Teaching, Coding, Research, etc.).
- Detects missing context based on domain heuristics.
- Generates a minimal multi-step plan.
- Proposes a next action (RequestContext or StopWithReport).
- Collects improvement candidates (missing context, weak plan, etc.).
- Is fully serializable to JSON via serde.
- Ships as a CLI command: `arpagona cognitive run --objective <TEXT>`.

## What V0 Does Not Do

- ❌ Call any LLM or provider.
- ❌ Execute tools or shell commands.
- ❌ Read or write files.
- ❌ Persist or query any database.
- ❌ Authorize, approve, or execute side effects.
- ❌ Self-modify or mutate governance structures.
- ❌ Replace the Decision Gate or action proposal path.
- ❌ Operate autonomously without human review.

Every `CognitiveCycleResult` carries an explicit `warning` field stating that
the output is evidence-only and non-authorizing.

## How This Prepares the Generalist Professional Agent

The V0 skeleton is designed to be consumed by a future LLM or orchestrator:

1. **Domain routing**: Future LLM integration can refine domain classification
   and route objectives to the right tools, memory stores, or providers.
2. **Plan elaboration**: The heuristic plan is a seed that an LLM can expand
   into detailed sub-tasks with dependencies, evidence sources, and timelines.
3. **Observation fulfillment**: Detected missing context becomes explicit
   queries to external sources (Graph Memory, web search, human input).
4. **Next action dispatch**: `ProposedNextAction` can drive the orchestrator's
   decision loop without requiring the orchestrator to re-parse raw text.
5. **Learning accumulation**: `ImprovementCandidate`s can be promoted to
   `FailureInsight` objects and persisted in Graph Memory for cross-session
   learning.

## Why Self-Improvement Remains Controlled

All improvement signals in V0 are **candidates** — they describe what could be
improved but never mutate state. A future orchestrator must:

- Review each candidate against current policies.
- Decide whether to promote it to a `FailureInsight` (governed path).
- Route the insight through the Decision Gate before any durable memory write.
- Respect the `non_authorizing` invariant on every `ProposedNextAction`.

This ensures that self-improvement remains human-governed and auditable, never
autonomous or opaque.
