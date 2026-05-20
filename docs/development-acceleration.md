# Development Acceleration — Hermes-like Alpha and Rippletide Direction

This document clarifies the current product-building bias for ARPAGONA Agent Core.

It complements `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md`, `docs/operating-doctrine.md`, `docs/architecture.md` and `docs/roadmap.md`.

## 1. Current intent

The project should now move aggressively toward a functional Hermes-like alpha while preserving ARPAGONA's own architecture.

The goal is not to clone Hermes. Hermes is an inspiration for practical agent-product ergonomics: CLI entrypoints, local developer workflow, explicit commands, scheduled loops, inspectable state and operator-friendly reporting.

ARPAGONA must remain distinct:

- Rust-first core;
- local-first supervision;
- graph-native memory;
- compute-aware routing;
- Rippletide-inspired runtime enforcement;
- Decision Gate before any external effect;
- audit and causal trace by default;
- human supervision for sensitive actions.

## 2. Product bias: CLI supervision first

The CLI is the first local Mission Control surface.

Near-term work should bias toward making the CLI useful for inspecting the system, rather than repeatedly adding more test-only guards around already-covered audit/readback behavior.

Test-only PRs remain valid when they protect a concrete uncovered regression risk. They should not be the default next step.

Preferred CLI progression:

1. strengthen `arpagona audit decision-summary <decision-id>`;
2. add `arpagona audit task-summary <task-id>`;
3. add `arpagona audit workspace-summary <workspace-id>`;
4. expose status/readback commands that help answer what happened, why it happened and what should happen next;
5. only then consider Mission Control Web expansion.

CLI commands must remain read-only unless a future command explicitly goes through the governed action path.

## 3. Aggressive but bounded development

The desired mode is aggressive iteration, not reckless expansion.

A good loop should ship one small usable increment, not just one more abstract guard.

Default preference order:

1. useful read-only CLI supervision increment;
2. small Rust abstraction that enables the next CLI/product step;
3. Graph Memory or Audit stabilization only when it unblocks product use or protects a real uncovered risk;
4. Runtime/API expansion only when it remains read-only or clearly governed;
5. execution/autonomy only after the governed path is ready.

## 4. Rippletide direction to preserve

The Rippletide-inspired direction is runtime enforcement.

The system must make it structurally difficult for an agent to act directly. Agents produce structured intent. The runtime evaluates intent against context, policies, permissions and risk before anything can affect the outside world.

Key requirements:

- every important action starts as a `ProposedAction`;
- applicable context should come from Graph Memory, not raw context stuffing;
- the Decision Gate decides approval, rejection, escalation or need for more context;
- important decisions produce causal audit traces;
- CLI/Mission Control surfaces inspect traces and decisions, but do not replace governance;
- readback is not approval;
- Tool Registry lookup is not approval;
- Graph Memory is not authorization.

## 5. Compute Reservoir direction

The system should also move toward real local/cloud delegation rather than using cloud reasoning for everything.

Hermes/cloud-like orchestration should be reserved for final judgment, planning and integration.

LOCO/Ollama/local models should handle first-pass reading, summarization, extraction, draft generation and log analysis whenever practical.

Future Compute Reservoir work should make this explicit in code: choosing which resource should think, read, summarize or draft based on capability, cost, latency and data sensitivity.

## 6. What to avoid now

Avoid:

- endless test-only stabilization when a product-visible readback surface is possible;
- growing API/Runtime before CLI supervision is useful;
- treating CLI as a debug toy rather than the first local Mission Control surface;
- copying Hermes without preserving ARPAGONA's governed architecture;
- implementing execution before the Tool Registry, Decision Gate and Audit chain are ready;
- letting Graph Memory, CLI, API or Runtime become hidden authorization layers.

## 7. Near-term success target

A useful near-term alpha should let a human operator run local CLI commands to understand:

- what tasks exist;
- what actions were proposed;
- what decisions were made;
- why they were made;
- which policies and risks were involved;
- what audit events support the answer;
- what remains pending.

That is the practical bridge between the current Rust foundation and a future Mission Control Web interface.
