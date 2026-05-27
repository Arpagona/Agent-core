# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical instruction file for the scheduled ARPAGONA focus loop.

The focus loop must now accelerate toward a usable local-first cognitive runtime while preserving the founding safety rule: build hard on internal cognitive architecture, and gate every external effect.

## 1. Mission

ARPAGONA Agent Core is not a coding assistant project. It is a local-first professional cognitive runtime and agentic orchestration kernel.

The focus loop exists to build the runtime bricks required for:

```text
Objective -> Working Memory -> Observations -> Plan -> Tool Use -> Assessment -> ProposedAction -> Decision Gate -> Audit -> Reflection -> Governed Learning
```

Each run must either:

1. finish, merge or clean work that blocks this chain;
2. implement one substantial milestone from the priority queue;
3. or report `NO-OP` with the precise blocker.

## 2. Current strategic posture

The project has moved beyond abstract stabilization. The active alpha objective is now:

```text
A governed, inspectable, MCP-compatible cognitive runtime with durable memory and local-first supervision.
```

The current acceleration pillars are:

1. **Governed MCP surface** — MCP is now an active integration layer, not a deferred topic. Only read-only or explicitly governed MCP capabilities are allowed.
2. **Holographic Memory** — associative memory must become durable, queryable and non-authorizing.
3. **Cognitive loop integration** — objectives, observations, working memory, planning, tool use and governed learning must converge into repeatable local alpha scenarios.
4. **Operator observability** — CLI and MCP resources/prompts are the near-term Mission Control precursor. Web Mission Control remains deferred.
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

If `PROJECT_STATUS.md` still says MCP integration is deferred, interpret that as stale wording. The current rule is: **MCP is allowed only as governed/read-only integration; unsafe execution remains forbidden.**

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
- repeatable local alpha scenarios.

Core rule:

```text
Build hard on internal cognitive architecture. Gate every external effect.
```

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

## 8. Runtime milestone queue

Choose the first safe actionable milestone.

### P1 — Finish open major PRs before starting new work

Goal: prevent branch sprawl and finish active runtime bricks.

Rules:

- if a major PR is open and mergeable, verify and merge when policy allows;
- if a major PR is open but conflicted, rebase/resolve before starting another brick;
- if a PR is superseded, close it only with clear evidence;
- do not open a duplicate branch for the same milestone.

### P2 — Holographic Memory persistence

Goal: make Holographic Memory durable enough to survive restarts and support real agent continuity.

Target chain:

```text
HolographicTrace -> PersistentStore -> Reload -> ResonanceSearch -> ReconstructedContext
```

Required properties:

- SQLite-backed store is preferred for the next bounded increment;
- in-memory store remains the default/simple path;
- persistence proves drop/reopen survival;
- no change to resonance scoring unless directly required;
- no LLM calls, no vector DB dependency, no authorization behavior;
- tests prove project isolation and persistence.

### P3 — Governed MCP observability

Goal: make MCP a safe, inspectable integration surface for external agents.

Target chain:

```text
MCP Client -> initialize -> resources/prompts/tools -> Decision Gate -> Audit -> read-only/gated response
```

Allowed MCP work:

- resources exposing server info, tool catalogue, audit summaries and safe status surfaces;
- prompts that help external agents inspect, summarize or assess governance state;
- notifications such as `tools/list_changed` if they are informational only;
- HTTP/SSE transport hardening;
- protocol correctness tests;
- audit and Decision Gate proof tests.

Forbidden MCP work:

- shell tools;
- network/browser tools;
- unrestricted filesystem access;
- write/delete tools outside an explicitly governed patch flow;
- secrets access;
- any MCP path that treats readback as approval.

### P4 — General Cognitive Work Loop V0

Goal: create the first general-purpose work loop.

Target chain:

```text
Objective -> WorkingMemory -> Plan -> RequiredObservations -> ProposedNextAction -> ImprovementCandidate
```

Expected user-facing command:

```bash
arpagona cognitive run --objective "..." --domain business --json
```

Required properties:

- works for professional domains, not only code;
- read-only and non-autonomous;
- no LLM calls unless already supported and explicitly safe;
- produces structured working memory, plan and next action;
- exposes missing context and improvement candidates.

### P5 — Cognitive Observation to Governed Learning

Goal: convert observation candidates into governed learning proposals.

Target chain:

```text
CognitiveObservation -> FailureInsightCandidate -> ProposedAction -> Decision Gate -> Audit -> governed FailureInsight readback
```

Required properties:

- candidate promotion remains explicit;
- no automatic memory write without governance;
- readback remains evidence-only;
- tests prove blocked, truncated or empty observations can produce governed learning proposals.

### P6 — Working Memory integration

Goal: observations and objectives must accumulate into active cycle state.

Target chain:

```text
Objective + CognitiveObservations -> WorkingMemory -> Plan update -> ProposedNextAction
```

Required properties:

- pure/domain-first design;
- no hidden prompt injection;
- no uncontrolled persistence;
- CLI or MCP readback for current cycle state.

### P7 — Compute Reservoir integration

Goal: make resource selection part of the cognitive loop.

Target chain:

```text
Objective / Task -> ComputeRequirement -> ComputeReservoir allocation -> explanation -> audit-ready decision context
```

Required properties:

- no real cloud/local model invocation required at first;
- allocation can be deterministic or simulated;
- must explain why a resource is selected;
- must prepare local/cloud delegation without executing it.

### P8 — End-to-end governed alpha demo

Goal: provide one-command demonstration only after the underlying runtime bricks exist.

Target:

```bash
scripts/demo-full-loop.sh
```

Do not choose this before the relevant runtime brick exists unless it directly validates a merged major brick.

## 9. Two-track acceleration map

The loop may alternate between Track A and Track B when both are safe and unblocked, but the priority queue above wins over mechanical alternation.

P1 open PR cleanup always takes priority.

### Track A — MCP Server / external agent integration

| Phase | Action |
|-------|--------|
| A1 | ✅ Phase 1 — crate + stdio transport + `tools/list` + `tools/call` |
| A2 | ✅ Phase 2 — DecisionGate governance before `tools/call` |
| A3 | ✅ Phase 3 — HTTP/SSE transport via Axum endpoint `/mcp` |
| A4 | ✅ Phase 4 — Resources + Prompts |
| A5 | ✅ Phase 5 — notifications / `tools/list_changed` / protocol hardening |
| A6 | 🔜 MCP operator-readiness: documentation, examples, client smoke tests |

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
- new strategic roadmap without human direction.

MCP itself is not forbidden anymore. Unsafe MCP capabilities are forbidden.

## 11. Required verification

For any code change:

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI changes, run the affected commands manually.

For MCP changes, include protocol-level tests or client smoke tests where practical.

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
- track: A, B, or cross-cutting
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
Await human direction for the next strategic roadmap — all planned milestones are delivered.
```
