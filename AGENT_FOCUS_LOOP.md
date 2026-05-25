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
6. `docs/daily-agent-validation.md`
7. files directly required by the selected milestone

If files conflict:

```text
safety/governance > AGENT_FOCUS_LOOP.md > FOCUS_LOOP_NEXT.md > PROJECT_OBJECTIVES.md > local opportunity
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

## 5. Runtime milestone queue

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

## 6. Forbidden work

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

## 7. Required verification

For any code change:

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI changes, run the affected commands manually.

For Tool Runtime or Observation changes, include safety-boundary tests for `.git`, `.env`, absolute paths and parent traversal.

For documentation-only changes, scan for conflict markers and explain why code tests were not required.

## 8. Reporting format

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

## 9. Handoff rule

At the end of every successful run, update `FOCUS_LOOP_NEXT.md` with one concrete next action only.

The handoff must target the next highest-priority runtime milestone, not a convenience script or cosmetic cleanup unless P0/P1 requires it.

