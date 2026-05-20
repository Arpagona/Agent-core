# ARPAGONA Agent Core — Operating Doctrine

This document defines the practical operating mode for human and agent contributors working on ARPAGONA Agent Core.

It complements `PROJECT_OBJECTIVES.md` and `PROJECT_STATUS.md`.

## 1. Current Mode: Controlled Fast Iteration

ARPAGONA Agent Core is moving from strict governance-first, test-only stabilization toward controlled fast iteration.

The project should now prioritize small usable increments, while preserving the safety invariants that make semi-autonomous development auditable and recoverable.

The intended rhythm is:

```text
small branch -> small implementation -> tests -> PR -> merge -> observe -> correct
```

The project should behave more like an iterative engineering lab than a frozen specification. Bounded technical risk is acceptable when the change is reversible, observable and easy to correct.

## 2. Core Principle

Move fast, but keep failures survivable.

Speed is encouraged when each change is:

- bounded to one clear objective;
- small enough to review;
- reversible;
- traceable through Git history and PR discussion;
- covered by tests where practical;
- aligned with the current roadmap;
- honest about risks and limitations;
- free of new unsafe external side effects.

A PR should deliver one of the following:

1. a small usable capability;
2. a bug fix;
3. a test that protects an important invariant;
4. a traceability or observability improvement;
5. a documentation update that changes how contributors or agents should operate.

Do not mix several unrelated objectives inside the same PR.

## 3. Rust-First Engineering Policy

Rust is the default implementation language for ARPAGONA Agent Core wherever it is technically practical.

The project should maximize Rust usage for:

- core domain types;
- governance logic;
- Decision Gate behavior;
- Audit and causal trace handling;
- Graph Memory interfaces and persistence adapters;
- Tool Registry definitions;
- Compute Reservoir logic;
- runtime orchestration primitives;
- API server backend logic;
- CLI tooling;
- workers that require reliability, concurrency, auditability or strong typing.

Rust is preferred because ARPAGONA Agent Core needs:

- memory safety;
- explicit error handling;
- predictable performance;
- strong typing for governance boundaries;
- safe concurrency;
- maintainable long-term systems code;
- clear separation between domain logic and adapters.

Non-Rust components are allowed only when they provide a clear practical advantage, such as:

- frontend user interfaces;
- quick experiments;
- ecosystem-specific integrations;
- machine-learning tooling where Python remains materially more practical;
- glue code that should later be replaced or wrapped by Rust if it becomes central.

When adding a non-Rust component, contributors should document why Rust was not chosen and whether the component is temporary, experimental or intentionally external.

Default heuristic:

```text
If it is core, persistent, safety-relevant, auditable, concurrent or long-lived, prefer Rust.
```

## 4. What Is Now Allowed

Small production code changes are allowed when they unlock concrete project progress.

Allowed work includes:

- small internal production-code changes;
- small API or CLI improvements when already implied by the roadmap;
- Graph Memory persistence improvements;
- Audit causal-trace improvements;
- Decision Gate hardening;
- focused bug fixes;
- small readback/query capabilities that improve human supervision;
- documentation updates when behavior, implementation status or operating mode changes.

This replaces the previous expectation that most work should be test-only.

## 5. Hard Safety Boundaries

Controlled fast iteration does not mean unrestricted autonomy.

The following remain blocked unless explicitly approved in a dedicated issue or PR:

- new privileged system operations outside the repository work scope;
- credential handling changes;
- browser or external automation expansion;
- MCP expansion;
- external side effects without Decision Gate review;
- destructive data operations;
- destructive persistence migrations;
- scheduler or autonomous-loop expansion;
- unrestricted provider/tool execution;
- broad refactors disguised as small changes;
- Decision Gate bypass;
- agent self-modification.

If a change touches one of these areas, the PR must stop and ask for explicit human approval.

## 6. Local Model Delegation Policy

Cloud orchestration should be conserved for judgment, planning, safety review and final integration.

Local models such as LOCO/Ollama should be used as local subcontractors for high-volume, low-risk repository work.

Default division of work:

- Hermes/cloud: orchestration, final judgment, architectural trade-offs, safety review, patch approval, PR creation and merge recommendations.
- LOCO/Ollama: reading, summarizing, extracting, comparing, drafting, log summarization and first-pass pattern scanning.

Hermes should delegate to LOCO/Ollama when a task involves:

- reading or comparing more than two files;
- summarizing repository structure or module responsibilities;
- extracting Rust types, functions, traits or conventions;
- finding repeated patterns or possible convention drift;
- drafting documentation sections;
- drafting tests;
- summarizing `cargo` or CI output;
- producing a first-pass risk checklist;
- comparing ARPAGONA structure with Hermes-inspired patterns.

Hermes should not delegate final authority for:

- architectural decisions;
- safety boundary decisions;
- final patch approval;
- PR creation or merge decisions;
- external side-effect decisions;
- credential-related decisions;
- destructive operations.

Every non-trivial loop should attempt at least one bounded LOCO/Ollama delegation step unless there is a clear reason not to.

A non-trivial loop includes:

- more than one file inspected;
- more than one design option considered;
- production code changed;
- project-level documentation changed;
- long logs or CI output interpreted.

Loop reports should include:

- `Delegated to LOCO/Ollama: yes/no`;
- `If no, why not?`;
- `Local task summary`;
- `Hermes final decision`.

Default heuristic:

```text
If the work is high-volume, local, low-risk and reversible, delegate the first pass locally.
If the work requires final judgment, safety reasoning or architectural commitment, Hermes decides.
```

The goal is to reduce cloud-token usage while making the system more distributed, local-first and resilient.

## 7. CLI Supervision First Bias

The current near-term product bias is CLI supervision first.

Recent Audit and Graph Memory guards have already established a useful readback foundation. Future loops should not keep adding test-only audit guards by default. Test-only work remains valid when it protects a newly discovered risk, but it should not become the default next step once a readback invariant is already covered.

The CLI is the first local Mission Control surface. It should become the practical way to inspect what the agent did, why it did it and what evidence exists, before Mission Control Web is expanded.

Preferred next CLI-oriented increments:

1. improve `arpagona audit decision-summary <decision-id>` usability, errors and tests;
2. add `arpagona audit task-summary <task-id>` when the data path is clean;
3. add `arpagona audit workspace-summary <workspace-id>` only after task-level readback is useful;
4. extract audit readback formatting from `main.rs` if the CLI file becomes too large;
5. add server-side readback endpoints only when client-side filtering becomes a real limitation.

Default selection rule:

```text
If the next proposed work is another test-only Audit/Graph Memory guard, first ask whether a small read-only CLI supervision increment would be more useful.
Choose the CLI increment unless the test protects a concrete uncovered regression risk.
```

CLI supervision remains strictly read-only. It must not approve, reject, execute, schedule, mutate external state, bypass the Decision Gate or treat readback as authorization.

## 8. Contributor / Agent Loop Protocol

Each work loop should follow this protocol:

1. Read `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` and this document.
2. Sync with `main`.
3. Select one bounded useful unit of work.
4. Prefer CLI supervision increments over additional test-only Audit/Graph Memory guards unless a concrete uncovered regression risk is identified.
5. Create a focused branch.
6. Implement only that unit.
7. Prefer Rust for all durable backend, governance, persistence, runtime, API and CLI work unless a documented exception applies.
8. Delegate at least one bounded first-pass analysis step to LOCO/Ollama for non-trivial loops, or report why delegation was not useful.
9. Run the relevant checks, at minimum:
   - `cargo fmt -- --check`
   - `cargo check`
   - `cargo test`
10. Open a PR with:
   - summary;
   - files changed;
   - tests run;
   - risk assessment;
   - explicit statement of what was not changed;
   - language choice note when non-Rust code is added;
   - LOCO/Ollama delegation note for non-trivial loops;
   - why a CLI supervision increment was or was not chosen.
11. Update `PROJECT_STATUS.md` only when implementation status, behavior, architecture, roadmap or safety assumptions actually changed.

## 9. Preferred Near-Term Direction

The next useful increments should bias toward making the core usable and inspectable, not merely adding abstract structure.

Preferred sequence:

1. keep Audit causal trace semantics coherent;
2. keep Graph Memory persistence conventions stable;
3. expand read-only CLI supervision paths for human inspection;
4. improve Decision Gate invariants only when a concrete uncovered risk is found;
5. only then expand Runtime/API surfaces in small Rust-first increments.

The system should become easier to ask:

- what happened?
- why did it happen?
- which input caused which proposed action?
- which decision approved or rejected it?
- what should be done next?

## 10. Anti-Patterns

Avoid:

- large speculative refactors;
- feature expansion without auditability;
- adding execution paths before the governance path is clear;
- making the CLI/API a hidden privileged control layer;
- treating Graph Memory as authorization logic;
- treating Tool Registry lookup as approval;
- adding autonomy before observability;
- changing `PROJECT_STATUS.md` for trivial or test-only changes;
- adding long-lived backend logic in a non-Rust language without documenting the reason;
- using cloud reasoning for large low-risk reading or summarization tasks that could be delegated locally;
- choosing another test-only Audit/Graph Memory guard when a bounded CLI supervision improvement would deliver more usable value.

## 11. Working Heuristic

When choosing between two tasks, prefer the one that makes the system more usable while preserving traceability.

A good change should make the next loop easier, safer or more productive.

When choosing between languages, prefer Rust unless the non-Rust option has a clear tactical advantage.

When choosing between cloud and local analysis, prefer local analysis for high-volume low-risk work and reserve cloud reasoning for final judgment.

When choosing between another readback guard and a read-only supervision surface, prefer the supervision surface unless the guard protects a real uncovered risk.
