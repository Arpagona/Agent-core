# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical instruction file for the scheduled ARPAGONA focus loop.

The project has reached the first governed alpha checkpoint. The previous P1-P8 queue, Track A MCP milestones and Track B Holographic Memory milestones are considered delivered unless a regression is found. The focus loop must now advance the next strategic layer: real model integration, operator control and product-ready demonstrations, while preserving the founding safety rule.

```text
Build hard on internal cognitive architecture. Gate every external effect.
```

## 1. Mission

ARPAGONA Agent Core is not a coding assistant project. It is a local-first professional cognitive runtime and agentic orchestration kernel.

The target runtime chain remains:

```text
Objective -> Working Memory -> Observations -> Plan -> Tool Use -> Assessment -> ProposedAction -> Decision Gate -> Audit -> Reflection -> Governed Learning
```

Each run must either:

1. finish, merge or clean work that blocks this chain;
2. implement one substantial milestone from the active Phase 2 roadmap;
3. or report `NO-OP` with the precise blocker.

## 2. Current strategic posture

The Phase 1 alpha foundation is complete:

- governed cognitive loop demo exists;
- MCP server foundation exists;
- Holographic Memory has durable and governed integration paths;
- Working Memory and Compute Reservoir are integrated enough for alpha scenarios;
- CLI and MCP readback surfaces exist for local supervision.

The active Phase 2 objective is now:

```text
A real-model, proposal-only cognitive runtime with operator visibility and product-grade demo scenarios.
```

The current acceleration pillars are:

1. **Real LLM integration under governance** — LLMs may enrich working memory, observations and proposals, but must never execute, approve, write memory directly or bypass Decision Gate.
2. **Operator control** — CLI/MCP first, then minimal Web Mission Control only after the control surfaces are clear.
3. **Product demonstration** — ARPAGONA Agent must become demonstrable through concrete SME/business scenarios, not only internal tests.
4. **Production hardening** — tests, edge cases, failure behavior and audit readability must improve before broader autonomy.
5. **Safety boundary preservation** — no shell, no unrestricted write, no secrets access, no browser automation, no hidden autonomy.

## 3. Files to read first

Every run must read:

1. `AGENT_CONTEXT.md`
2. `PROJECT_OBJECTIVES.md`
3. `PROJECT_STATUS.md`
4. `AGENT_FOCUS_LOOP.md`
5. `FOCUS_LOOP_NEXT.md`
6. `DAILY_VALIDATION_BACKLOG.md`
7. `docs/daily-agent-validation.md`
8. files directly required by the selected milestone

If files conflict:

```text
safety/governance > AGENT_FOCUS_LOOP.md > DAILY_VALIDATION_BACKLOG.md > FOCUS_LOOP_NEXT.md > PROJECT_STATUS.md > PROJECT_OBJECTIVES.md > local opportunity
```

If an older file describes the previous P1-P8 roadmap as still pending, treat that as stale unless code or tests prove a regression.

## 4. Operating mode: major bounded increments

One run may create at most one PR, and that PR should be a coherent runtime brick, not a tiny cosmetic change.

A good PR may touch several crates, CLI, docs and tests if all changes serve one milestone.

Avoid:

- isolated flags without loop-level proof;
- demo-only wrappers before the underlying runtime exists;
- readback fields that do not improve supervision or auditability;
- duplicate branches for the same idea;
- opportunistic work not tied to the queue below.

Prefer:

- end-to-end cognitive loops;
- bridges between existing bricks;
- structured runtime state;
- governed learning paths;
- durable memory and replayable traces;
- MCP resources/prompts that make the system inspectable by other agents;
- repeatable local alpha scenarios;
- proposal-only LLM integration with audit traces.

## 5. P0 — Hygiene before acceleration

Before new development:

- `main` must be green or clearly reported blocked;
- no unresolved conflict markers;
- no duplicate open PRs for the same topic;
- no stale branch should be reused unless its commits are explicitly relevant;
- one topic should have one active PR.

If duplicate or superseded PRs exist, clean them before feature work.

When uncertain, do not delete. Report.

## 6. Morning use of the daily validation backlog

At the 7am focus-loop run, before selecting a new runtime milestone, inspect `DAILY_VALIDATION_BACKLOG.md`.

If it contains an open validation item that is:

- evidenced by a midnight daily validation run;
- safe and bounded;
- directly related to runtime correctness, safety boundaries, CLI/MCP observability, model interaction quality, tests or documentation required for operator use;
- small enough to complete as one coherent PR;

then prefer fixing that item before starting new feature work, unless P0/P1 hygiene or an already-open major PR blocks it.

Rules:

- Process at most one backlog item per run.
- If several items qualify, choose the highest severity, then the one most directly tied to safety or broken documented behavior.
- If an item is fixed, update `DAILY_VALIDATION_BACKLOG.md` in the same PR with the fix evidence and status.
- If an item is not chosen, briefly explain why in the report.
- Do not treat backlog entries as authorization to add real execution, unrestricted shell, hidden autonomy, external effects or broad new capabilities.

## 7. Merge & run cycle

This rule governs every cron run. It keeps the loop productive while avoiding uncontrolled branch sprawl.

### 7.1 At the start of every cron run

1. Fetch `origin/main` and check for new commits.
2. Fetch the PR created by the previous run, if any.
3. Determine the state:

| Previous PR state | Action |
|---|---|
| Already merged | Continue to new feature. |
| Open, mergeable, and the previous push had green checks | Merge according to repository policy, then start new feature. |
| Open, mergeable, but checks failed | Do not merge. Report blocker. |
| Open, not mergeable | Rebase/resolve, rerun checks, then merge only if green. |
| No previous PR | Start new feature directly. |

### 7.2 After merge — new feature

1. Rebase onto `origin/main`.
2. Create a new branch named `feat/<milestone-key>` or `docs/<bounded-topic>`.
3. Implement one bounded increment from the milestone queue.
4. Run verification before commit.
5. Push and create a PR.

### 7.3 If checks fail before push

- Do not push.
- Do not create a PR.
- Report the failure with the relevant output.
- Set `FOCUS_LOOP_NEXT.md` to the same handoff.
- The next run must retry or explicitly report why it is blocked.

## 8. Phase 2 runtime milestone queue

Choose the first safe actionable milestone.

### P1 — Finish open major PRs before starting new work

Goal: prevent branch sprawl and finish active runtime bricks.

Rules:

- if a major PR is open and mergeable, verify and merge when policy allows;
- if a major PR is open but conflicted, rebase/resolve before starting another brick;
- if a PR is superseded, close it only with clear evidence;
- do not open a duplicate branch for the same milestone.

### C1 — Real LLM integration in proposal-only mode

Goal: connect `arpagona cognitive run --llm` to the existing LLM/provider abstraction without granting execution power.

Target chain:

```text
Objective -> WorkingMemory -> LLM-assisted reasoning -> ProposedAction -> Decision Gate -> Audit
```

Required properties:

- LLM output may enrich working memory, observations, plans and proposals;
- LLM output must never execute tools directly;
- LLM output must never approve actions;
- LLM output must never write memory directly;
- LLM output must never bypass Decision Gate;
- provider, model, prompt summary and response summary should be audit-readable where practical;
- CLI smoke test with `--llm` must demonstrate proposal-only behavior;
- tests must prove tool-call/direct-execution attempts remain blocked or represented as proposals.

### C2 — LLM governance hardening

Goal: make model interaction safe enough for repeated local alpha usage.

Required properties:

- prompt-injection resistance tests for model outputs;
- structured parsing failure behavior;
- fallback when provider is unavailable;
- token/cost metadata where practical;
- clear distinction between model confidence and authorization.

### C3 — Compute-aware model selection

Goal: use Compute Reservoir to decide whether a local, cloud, small or large model should be proposed for a task.

Required properties:

- selection may be deterministic or simulated first;
- selection must be explainable;
- no provider call is made merely because a resource is selected;
- decision context remains audit-ready.

### D1 — Operator status surface

Goal: expose one coherent operator status view before building a full UI.

Target surfaces:

- CLI status command;
- MCP resource status;
- optional JSON endpoint if already aligned with the API server.

Required properties:

- show runtime health;
- show last decisions/audit summaries;
- show memory store status;
- show MCP capabilities;
- show current handoff/backlog status;
- read-only only.

### D2 — Minimal Web Mission Control skeleton

Goal: start Web Mission Control only after D1 has a clear read-only status contract.

Required properties:

- read-only dashboard;
- no execution buttons;
- no approval buttons unless governance semantics are explicitly designed;
- display current runtime state, audit events, proposed actions and memory status.

### E1 — SME documentary assistant demo

Goal: produce a product-facing ARPAGONA Agent scenario.

Scenario:

```text
User objective -> ingest/read bounded documents -> extract observations -> propose next actions -> Decision Gate -> audit -> summary
```

Required properties:

- useful to a small business / local SME;
- demoable from CLI first;
- no uncontrolled document ingestion;
- no hidden memory write;
- no external effects.

### E2 — Business/prospecting workflow demo

Goal: demonstrate how ARPAGONA Agent can help with a real business workflow.

Candidate scenarios:

- qualification of a client need;
- preparation of a proposal outline;
- analysis of a project brief;
- follow-up action suggestions.

Required properties:

- proposal-only;
- traceable;
- explainable;
- usable in a sales/product demonstration.

### H1 — Production hardening pass

Goal: stabilize existing alpha behavior before adding broader autonomy.

Allowed work:

- tests for edge cases;
- error handling;
- regression tests;
- audit readability;
- documentation of existing behavior;
- dependency and feature-flag cleanup.

Not allowed:

- new broad capabilities disguised as hardening;
- shell/browser/secrets/email/scheduler expansion.

## 9. Completed Phase 1 checkpoint

The following roadmap is treated as delivered unless a regression is found.

### Phase 1 priority queue

| Milestone | Status |
|-----------|--------|
| P1 — Open PR cleanup | ✅ Complete |
| P2 — Holographic Memory persistence | ✅ Complete |
| P3 — Governed MCP observability | ✅ Complete |
| P4 — General Cognitive Work Loop V0 | ✅ Complete |
| P5 — Cognitive Observation to Governed Learning | ✅ Complete |
| P6 — Working Memory integration | ✅ Complete |
| P7 — Compute Reservoir integration | ✅ Complete |
| P8 — End-to-end governed alpha demo | ✅ Complete |

### Track A — MCP Server / external agent integration

| Phase | Action |
|-------|--------|
| A1 | ✅ Phase 1 — crate + stdio transport + `tools/list` + `tools/call` |
| A2 | ✅ Phase 2 — DecisionGate governance before `tools/call` |
| A3 | ✅ Phase 3 — HTTP/SSE transport via Axum endpoint `/mcp` |
| A4 | ✅ Phase 4 — Resources + Prompts |
| A5 | ✅ Phase 5 — notifications / `tools/list_changed` / protocol hardening |
| A6 | 🔜 Operator-readiness deep dive remains available as Phase 2 D/E support work |

### Track B — Holographic Memory / internal cognitive memory

| Step | Action |
|------|--------|
| B1 | ✅ Conversation-memory bridge — encode turns as `HolographicTrace` |
| B2 | ✅ Recursive memory graph traversal via `linked_memory_ids` |
| B3 | ✅ Optional local embeddings / semantic generalization |
| B4 | ✅ SQLite persistence for holographic memory |
| B5 | ✅ Consolidation and duplicate trace fusion |
| B6 | ✅ Governed writes via DecisionGate, e.g. `MemoryWriteKind::HolographicTrace` |
| B7 | ✅ Cognitive-loop recall hints from resonance matches |

## 10. Forbidden work

Do not add:

- unrestricted shell;
- arbitrary command execution;
- file deletion;
- unrestricted write tools;
- secrets access;
- browser automation;
- network automation beyond explicitly bounded protocol surfaces;
- email sending;
- scheduler/autonomy expansion beyond the existing external cron mechanism;
- hidden prompt/context injection;
- broad user-memory ingestion;
- readback-as-authorization behavior;
- Decision Gate bypass;
- self-modification without explicit governed proposal;
- direct LLM tool execution;
- direct LLM approval of actions;
- direct LLM memory writes.

MCP itself is not forbidden. Unsafe MCP capabilities are forbidden.

LLM integration itself is not forbidden. Unsafe LLM authority is forbidden.

## 11. Required verification

For any code change:

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI changes, run the affected commands manually.

For MCP changes, include protocol-level tests or client smoke tests where practical.

For LLM/provider changes, include tests or smoke tests proving proposal-only behavior and safe provider failure behavior.

For Tool Runtime or Observation changes, include safety-boundary tests for `.git`, `.env`, absolute paths and parent traversal.

For documentation-only changes, scan for conflict markers and explain why code tests were not required.

## 12. Reporting format

Every run must report:

```text
Focus Loop Report
- trigger:
- selected priority item:
- why this item was chosen:
- PR/branch handled:
- runtime chain advanced:
- track: C, D, E, H, or cross-cutting
- track phase/step:
- work completed:
- tests run:
- merge/auto-merge status:
- blockers:
- risks:
- deliberately not changed:
- next recommended handoff:
```

If no safe action is available, report `NO-OP` and explain the blocker.

## 13. Handoff rule

At the end of every successful run, update `FOCUS_LOOP_NEXT.md` with one concrete next action only.

The handoff must target the next highest-priority runtime milestone, not a convenience script or cosmetic cleanup unless P0/P1 requires it.

The current preferred next handoff, unless blocked, is:

```text
Track C Step C1 — Real LLM integration in proposal-only mode.
```
