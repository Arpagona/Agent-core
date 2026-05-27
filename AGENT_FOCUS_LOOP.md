# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical instruction file for the scheduled hourly ARPAGONA focus loop.

The focus loop must now accelerate toward a usable general-purpose cognitive agent runtime. It must stop producing isolated polish work unless that work directly unlocks a major runtime milestone.

## 1. Mission

ARPAGONA Agent Core is not a coding assistant project. It is a local-first professional cognitive runtime.

The focus loop exists to build the runtime bricks required for:

```text
Objective -> Working Memory -> Observations -> Plan -> Tool Use -> Assessment -> ProposedAction -> Decision Gate -> Audit -> Reflection -> Governed Learning
```

Each run must either:

1. finish/merge/clean work that blocks this chain;
2. implement one substantial milestone from the priority queue;
3. or report NO-OP with the precise blocker.

## 2. Files to read first

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
safety/governance > AGENT_FOCUS_LOOP.md > DAILY_VALIDATION_BACKLOG.md > FOCUS_LOOP_NEXT.md > PROJECT_OBJECTIVES.md > local opportunity
```

## 3. Operating mode: major bounded increments

One run may create at most one PR, but that PR should be a coherent runtime brick, not a tiny cosmetic change.

A good PR may touch several crates, CLI, docs and tests if all changes serve one milestone.

Avoid:

- another isolated flag;
- another demo-only wrapper;
- another readback field without a loop-level proof;
- duplicate branches for the same idea;
- opportunistic work not tied to the queue below.

Prefer:

- end-to-end cognitive loops;
- bridges between existing bricks;
- structured runtime state;
- governed learning paths;
- repeatable local alpha scenarios.

Core rule:

```text
Build hard on internal cognitive architecture. Gate every external effect.
```

## 4. P0 — Hygiene before acceleration

Before new development:

- `main` must be green or clearly reported blocked;
- no unresolved conflict markers;
- no duplicate open PRs for the same topic;
- no stale branch should be reused unless its commits are explicitly relevant;
- one topic should have one active PR.

If duplicate or superseded PRs exist, clean them before feature work.

When uncertain, do not delete. Report.

## 5. Morning use of the daily validation backlog

At the 7am focus-loop run, before selecting a new runtime milestone, inspect `DAILY_VALIDATION_BACKLOG.md`.

If it contains an open validation item that is:

- evidenced by a midnight daily validation run;
- safe and bounded;
- directly related to runtime correctness, safety boundaries, CLI observability, model interaction quality, tests or documentation required for operator use;
- small enough to complete as one coherent PR;

then prefer fixing that item before starting new feature work, unless P0/P1 hygiene or an already-open major PR blocks it.

Rules:

- Process at most one backlog item per run.
- If several items qualify, choose the highest severity, then the one most directly tied to safety or broken documented behavior.
- If an item is fixed, update `DAILY_VALIDATION_BACKLOG.md` in the same PR with the fix evidence and status.
- If an item is not chosen, briefly explain why in the report.
- Do not treat backlog entries as authorization to add real execution, unrestricted shell, hidden autonomy, external effects or broad new capabilities.

## 6. Merge & run cycle (auto-merge policy)

This rule governs every cron run. It ensures the loop stays productive without requiring a human review step for every PR.

### 6.1 At the start of every cron run

1. **Fetch `origin/main`** and check for new commits.
2. **Fetch the PR created by the previous run** (if any) using `gh pr list --state open --json number,headRefName,mergeable,title`.
3. **Determine the state:**

   | Previous PR state | Action |
   |---|---|
   | Already merged (no open PR for that branch) | ✅ Continue to new feature |
   | Open, mergeable, **and the previous push had green cargo checks** | ✅ **Merge auto** into `main`, then start new feature |
   | Open, mergeable, **but previous push failed checks** | ❌ **Do not merge.** Notify Thibaud. Block until resolved. |
   | Open, **not mergeable** (conflict with main) | 🔧 **Rebase** onto latest `main`, re-run `cargo fmt --check && cargo check && cargo test --workspace`. If green → push, merge auto, then continue. If red → notify, block. |
   | No previous PR (first run) | ✅ Start new feature directly |

4. **Merge is unconditional when checks are green.** No human gate. Thibaud can always revert or close a PR after the fact.

### 6.2 After merge — new feature

1. Rebase onto `origin/main`.
2. Create a new branch named `feat/<milestone-key>`.
3. Implement one bounded increment from the milestone queue.
4. `cargo fmt --check && cargo check && cargo test --workspace` before every commit.
5. Push and create a PR via `gh pr create`.

### 6.3 If checks fail before push

- **Do not push.** Do not create a PR.
- Report the failure with full `cargo test` output in the Telegram report.
- Set `FOCUS_LOOP_NEXT.md` to the same handoff (no progress made).
- The next run will retry the same feature.

## 7. Runtime milestone queue

Choose the first safe actionable milestone.

### P1 — Finish open major PRs before starting new work

Goal: prevent branch sprawl and finish active runtime bricks.

Rules:

- if a major PR is open and mergeable, verify and merge when policy allows;
- if a major PR is open but conflicted, rebase/resolve before starting another brick;
- if a PR is superseded, close it only with clear evidence;
- do not open a duplicate branch for the same milestone.

### P2 — General Cognitive Work Loop V0

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
- no LLM calls yet unless explicitly already supported and safe;
- produces structured working memory, plan and next action;
- exposes missing context and improvement candidates.

This is the next core AGI-like runtime brick.

### P3 — Cognitive Observation to Governed Learning

Goal: convert observation candidates into governed learning proposals.

Target chain:

```text
CognitiveObservation -> FailureInsightCandidate -> ProposedAction -> Decision Gate -> Audit -> governed FailureInsight readback
```

Required properties:

- candidate promotion remains explicit;
- no automatic memory write without governance;
- readback remains evidence-only;
- tests must prove blocked/truncated/empty observations can produce governed learning proposals.

### P4 — Working Memory integration

Goal: observations and objectives must accumulate into active cycle state.

Target chain:

```text
Objective + CognitiveObservations -> WorkingMemory -> Plan update -> ProposedNextAction
```

Required properties:

- pure/domain-first design;
- no hidden prompt injection;
- no uncontrolled persistence;
- CLI readback for current cycle state.

### P5 — Compute Reservoir integration

Goal: make resource selection part of the cognitive loop.

Target chain:

```text
Objective / Task -> ComputeRequirement -> ComputeReservoir allocation -> explanation -> audit-ready decision context
```

Required properties:

- no real cloud/local model invocation required at first;
- allocation can be deterministic/simulated;
- must explain why a resource is selected;
- must prepare local/cloud delegation.

### P6 — Holographic Memory experimental bridge

Goal: surface non-authorizing pattern resonance in the loop.

Target chain:

```text
Observation / Failure / Plan -> HolographicTrace -> HolographicMatch -> caution / recall hint
```

Required properties:

- no vector database required initially;
- no authorization;
- no runtime dependence on embeddings;
- CLI/status proof acceptable for first PR.

### P7 — Demo script only after runtime bricks

Goal: provide one-command demonstration only after the underlying runtime brick exists.

Target:

```bash
scripts/demo-full-loop.sh
```

Do not choose this before P2/P3 unless it directly validates a merged major brick.

## 8. Forbidden work

Do not add:

- unrestricted shell;
- file deletion;
- write tools outside a specifically approved patch flow;
- secrets access;
- browser automation;
- MCP integration;
- email sending;
- scheduler/autonomy expansion beyond existing external cron;
- hidden prompt/context injection;
- broad user-memory ingestion;
- readback-as-authorization behavior;
- Decision Gate bypass;
- self-modification without explicit governed proposal;
- new strategic roadmap without human direction.

## 9. Required verification

For any code change:

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI changes, run the affected commands manually.

For Tool Runtime or Observation changes, include safety-boundary tests for `.git`, `.env`, absolute paths and parent traversal.

For documentation-only changes, scan for conflict markers and explain why code tests were not required.

## 10. Reporting format

Every run must report:

```text
Focus Loop Report
- trigger:
- selected priority item:
- why this item was chosen:
- PR/branch handled:
- runtime chain advanced:
- work completed:
- tests run:
- merge/auto-merge status:
- blockers:
- risks:
- deliberately not changed:
- next recommended handoff:
```

If no safe action is available, report `NO-OP` and explain the blocker.

## 11. Handoff rule

At the end of every successful run, update `FOCUS_LOOP_NEXT.md` with one concrete next action only.

The handoff must target the next highest-priority runtime milestone, not a convenience script or cosmetic cleanup unless P0/P1 requires it.

## 12. Two-track alternation (from 2026-05-26)

The focus loop runs every 15 minutes and must advance **both** tracks below, alternating between them on each run.

P1 (open PRs) takes priority over alternation: if an open PR is mergeable, merge it first regardless of track. If one track is blocked (e.g. awaiting merge), advance the other track.

### Track A — MCP Server (connect Arpagona to external agents)

| Phase | Action |
|-------|--------|
| A1 | ✅ Phase 1 — crate + stdio transport + tools/list + tools/call (PR #105) |
| A2 | ✅ Phase 2 — DecisionGate governance before tools/call (PR #107 + PR #110) |
| A3 | ✅ Phase 3 — HTTP/SSE transport via Axum endpoint `/mcp` (PR #112) |
| A4 | ✅ Phase 4 — Resources + Prompts (current PR) |
| A5 | 🔜 Phase 5 — notifications/tools/list_changed |

### Track B — Holographic Memory (internal cognitive memory)

| Step | Action |
|------|--------|
| B1 | ✅ Intégration avec `conversation-memory` — encoder les tours comme `HolographicTrace` (PR #109) |
| B2 | ✅ Graphe mémoire récursif — suivre les `linked_memory_ids` en profondeur (PR #111) |
| B3 | ✅ Embeddings locaux optionnels (généralisation sémantique) (PR #113) |
| B4 | 🔜 Persistance SQLite/SurrealDB |
| B5 | Consolidation périodique + fusion traces redondantes |
| B6 | Gouvernance des écritures via DecisionGate (`MemoryWriteKind::HolographicTrace`) |

### Reporting requirement

Every run report must include the chosen track:

```text
- track: A or B
- track phase/step: e.g. A2, B1
- alternation status: e.g. "last run was B, now A"
```

