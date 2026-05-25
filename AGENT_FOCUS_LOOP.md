# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical instruction file for the scheduled hourly ARPAGONA focus loop.

The focus loop is not the product strategist. It is a disciplined operator that consumes the priority queue below, keeps the repository clean, and advances one bounded task at a time.

## 1. Files to read first

Every run must read:

1. `AGENT_CONTEXT.md`
2. `PROJECT_OBJECTIVES.md`
3. `PROJECT_STATUS.md`
4. `AGENT_FOCUS_LOOP.md`
5. `FOCUS_LOOP_NEXT.md`
6. `docs/daily-agent-validation.md`
7. Any file directly required by the selected task

If files conflict, apply this order:

```text
safety/governance > AGENT_FOCUS_LOOP.md > FOCUS_LOOP_NEXT.md > PROJECT_OBJECTIVES.md > local opportunity
```

## 2. Operating rule

One run may do at most one coherent bounded action.

Allowed action types:

- verify an open PR;
- merge a green, safe, explicitly relevant PR if policy allows;
- close or clean a superseded branch/PR when evidence is clear;
- implement one bounded priority-queue item;
- produce a no-op report when no safe task is actionable.

Do not invent a new strategic direction. Do not open multiple PRs in one run. Do not split the same intention into duplicate branches.

## 3. P0 — Git hygiene first

Before new work, inspect repository hygiene.

Required checks:

- `main` must be green or clearly reported as blocked;
- no unresolved conflict markers;
- no duplicate open PRs for the same topic;
- no stale branch should be reused unless its commits are explicitly relevant;
- one topic should have one active PR.

If duplicate or superseded PRs exist, prefer cleanup over feature work.

A PR or branch may be closed/deleted only when evidence shows it is:

- already merged into `main`;
- fully superseded by a newer PR;
- explicitly abandoned;
- or safe to remove after human/DEEP cleanup report.

When uncertain, do not delete. Report.

## 4. Current priority queue

The focus loop must choose the first safe actionable item.

### P1 — Finish open PRs before new work

Goal: prevent branch sprawl.

Current known priority:

```text
Review and finish the active `--description` propagation PR chain.
```

Rules:

- if #77 is open and checks are green, verify it and merge only if safe;
- close or ignore older superseded `--description` PRs only after evidence;
- do not create a new `--description` branch while #77 exists.

Exit criteria:

- operator `--description` appears in readback;
- in-process and cross-invocation proof exists;
- duplicate branches/PRs are closed or clearly marked superseded;
- `main` remains green.

### P2 — Tool Runtime Observation bridge

Goal: make read-only tool outputs usable by the cognitive runtime.

Target chain:

```text
ToolExecutionResult -> ToolObservation -> future Working Memory / Reflection / FailureInsight
```

Allowed scope:

- pure types;
- structured readback;
- tests for success / blocked / failed observations;
- no new external effects;
- no write tools;
- no shell;
- no autonomous execution.

### P3 — FailureInsight candidates from tool outcomes

Goal: blocked or failed tool usage can become non-authorizing learning candidates.

Allowed scope:

- candidate only;
- no automatic memory write;
- no Decision Gate bypass;
- no persistence unless already governed and explicitly tested.

### P4 — Holographic Memory status CLI

Goal: expose Holographic Memory implementation status without pretending runtime integration exists.

Target command:

```bash
arpagona memory holographic status --json
```

Allowed scope:

- read-only status;
- no embeddings;
- no vector store;
- no runtime integration.

### P5 — Demo snapshot discovery

Goal: discover local demo snapshots without knowing exact paths.

Target command:

```bash
arpagona memory demo snapshot-list --json
```

Allowed scope:

- local demo artifacts only;
- evidence-only warnings;
- no production persistence claim.

## 5. Forbidden work

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
- new strategic roadmap without human direction.

## 6. Required verification

For any code change, run:

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI behavior changes, also run the exact affected command(s).

For Tool Runtime changes, include safety-boundary tests for blocked paths such as `.git`, `.env`, absolute paths and parent traversal.

For documentation-only changes, still scan for conflict markers and explain why code tests were not required.

## 7. Reporting format

Every run must report:

```text
Focus Loop Report
- trigger:
- selected priority item:
- why this item was chosen:
- PR/branch handled:
- work completed:
- tests run:
- merge/auto-merge status:
- blockers:
- risks:
- deliberately not changed:
- next recommended handoff:
```

If no safe action is available, report `NO-OP` and explain the blocker.

## 8. Handoff rule

At the end of every successful run, update `FOCUS_LOOP_NEXT.md` with one concrete next action only.

Do not write a roadmap there. The roadmap lives in this file. The handoff file is only the next executable step.

