# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical operational instruction file for the scheduled ARPAGONA Agent Core focus loop.

The local cron/Hermes job must treat this file as the current source of truth for what to work on, what to avoid, how ambitious to be, and how to report the result.

The local cron should stay minimal: pull the repository, read this file and the canonical project files, perform one coherent bounded PR, optionally auto-merge it if it satisfies the controlled auto-merge policy, then report.

## 1. Files that must be read at the start of every loop

Every focus-loop run must read, in this order:

1. `AGENT_CONTEXT.md`
2. `PROJECT_OBJECTIVES.md`
3. `PROJECT_STATUS.md`
4. `AGENT_FOCUS_LOOP.md`
5. `docs/operating-doctrine.md`
6. `docs/development-acceleration.md`
7. Any docs, issues or source files directly relevant to the chosen task

If these files conflict, prioritize safety and governance first, then `AGENT_FOCUS_LOOP.md` for current operational priority, then `PROJECT_OBJECTIVES.md` for long-term architecture.

## 2. Operating mode: ambitious governed iteration

The focus loop should no longer default to the smallest cosmetic or generic CLI change.

Hermes/GONA is expected to use strong reasoning and judgment to select meaningful architectural increments. GPT-5.5-level supervision may be trusted to choose a more ambitious implementation when it remains bounded, testable, reversible and auditable.

Do not choose the safest trivial increment when a more meaningful bounded architectural increment is possible.

Prefer progress that integrates real project bricks over superficial polish. A good loop should move the architecture forward, not merely add another readback flag.

When several safe options exist, choose the most ambitious one that remains:

- coherent as a single PR;
- bounded in scope;
- testable locally;
- reversible through Git;
- explainable in the PR body;
- aligned with Graph Memory, governed memory writes, Decision Gate or Audit;
- free of uncontrolled external side effects.

A coherent PR may modify several files when required by the architectural slice. For example, a good PR may include domain vocabulary, permissions, Decision Gate handling, audit metadata, CLI readback, tests and docs if all changes serve the same governed memory objective.

Avoid paralysis through excessive caution. Governance is meant to control effects, not prevent architectural progress.

Core rule:

```text
Move fast on internal architecture. Gate anything that changes state, memory, tools or the outside world.
```

## 3. Current strategic priority

Move beyond memory-write vocabulary and make governed memory-write proposals observable and controllable.

Current priority issue:

```text
#47 — Make governed memory-write proposals observable and controllable
```

The system has already moved through these first steps:

```text
Graph Memory visibility -> governed memory-write proposal vocabulary
```

The next target is now:

```text
governed memory-write proposals -> observable/controllable proposal readback -> audit-linked memory facts -> approved memory persistence
```

The immediate goal is not database persistence. The immediate goal is to let a human or local supervision surface inspect, understand and eventually act on proposed memory writes before any real Graph Memory mutation is introduced.

This must still preserve the founding rule:

```text
Agent -> ProposedAction -> DecisionGate -> Decision -> Audit -> controlled effect only if approved
```

## 4. Preferred next work

Prefer one coherent bounded PR per loop. The PR may be substantial if it advances one architectural slice cleanly.

The preferred next milestone is:

```text
Observable and controllable governed memory-write proposals
```

A strong next PR should allow a supervisor to answer:

```text
What memory write did the agent propose?
Why did it propose it?
What would be remembered?
What provenance/source supports it?
What confidence does it carry?
What permission/policy blocked, approved or escalated it?
What is the next safe human action?
```

Good implementation shapes include one or more of the following if they form a coherent slice:

### CLI proposal readback

Add or extend a read-only CLI surface for proposed memory writes.

Possible commands:

```bash
arpagona memory proposals
arpagona memory proposals --json
arpagona memory proposal <id>
arpagona memory proposal <id> --json
```

The exact command shape may differ if the existing CLI architecture suggests a cleaner fit.

### Audit/decision readback for memory proposals

Improve audit or decision summaries so memory-write proposals expose:

- action type;
- memory write kind;
- target type;
- provenance/source;
- confidence;
- reason for remembering;
- required permission;
- decision status;
- explicit reason;
- suggested next action.

### Structured formatting helpers

Add structured formatting or serialization helpers for `MemoryWriteIntent` so CLI and audit surfaces can show proposed memory writes consistently.

The key proof is not persistence. The key proof is that proposed memory writes are visible, understandable, non-mutating, and governed.

## 5. Secondary useful work

If memory proposal readback is blocked by missing foundations, choose the most ambitious enabling slice, not a trivial substitute.

Good enabling slices include:

- tests proving `create_memory_fact` and `create_failure_insight_memory` appear clearly in decision/audit summaries;
- serialization/readback improvements for `MemoryWriteIntent`;
- documentation of the lifecycle from memory-write proposal to later approved persistence;
- CLI listing of pending/proposed actions filtered to memory-write action types;
- improved explanatory audit metadata for memory-write proposals.

Do not jump directly to persistence unless the proposal, permission, audit and readback path is already clear and tested.

## 6. Allowed work

Allowed during the current phase:

- read-only CLI/audit readback for memory-write proposals;
- JSON output for memory proposal inspection;
- proposal formatting and serialization helpers;
- tests proving proposed memory writes are inspectable;
- tests proving blocked and confirmation-required memory-write proposals remain governed;
- Graph Memory documentation and architecture clarification;
- Decision Gate support for memory-write permissions and risk classification;
- explanatory audit for memory-write proposals;
- narrow operational-memory design around FailureInsight, corrections and project decisions;
- minimal approved memory persistence only after proposal observability/control is proven;
- `PROJECT_STATUS.md` updates reflecting each significant change.

## 7. Forbidden work

Do not implement yet:

- direct Graph Memory mutation by LLM/provider/runtime;
- broad unrestricted autonomous memory writing;
- memory writes without ProposedAction;
- memory writes without Decision Gate;
- memory writes without audit;
- hidden context injection into LLM prompts;
- broad semantic search;
- embeddings pipeline;
- ingestion of large user archives;
- personal or sensitive memory writes without human confirmation;
- secrets or credentials in memory;
- real tool execution;
- shell access as an ARPAGONA runtime capability;
- browser automation;
- MCP expansion;
- scheduler/autonomy expansion beyond the existing external cron;
- Mission Control Web expansion;
- readback-as-authorization behavior;
- destructive operations.

These restrictions are not meant to slow internal architecture. They are meant to prevent uncontrolled external effects or untraceable state mutation.

## 8. Mandatory safeguards for memory work

Any memory-write proposal or design must include, at minimum:

- typed memory target;
- explicit provenance/source;
- confidence level;
- timestamp;
- actor/source;
- reason for remembering;
- Decision Gate result;
- audit event linkage;
- future invalidation/supersession path, even if not fully implemented yet.

Sensitive or personal facts must require human confirmation. The first alpha should focus on operational/project memory, not private personal memory.

## 9. PR policy

Each loop should usually create one coherent bounded PR.

A good PR is:

- architecturally meaningful;
- bounded;
- testable;
- aligned with observable/controllable Graph Memory proposals;
- safe by construction;
- documented;
- accompanied by `PROJECT_STATUS.md` update when significant.

Do not split one coherent architectural slice into tiny PRs solely out of caution. It is acceptable for one PR to touch core, decision-gate, CLI, docs and tests if the change is coherent and governed.

Avoid PRs that only add generic CLI polish unless they directly support memory proposal observability, governed memory writes, audit explainability or local supervision of the current priority.

## 10. Controlled auto-merge policy

The focus loop may auto-merge its own PR without waiting for human review only when all of the following conditions are true:

- the PR was created by the current focus-loop run;
- the PR targets `main`;
- the PR is not a draft;
- GitHub reports the PR as mergeable;
- all required GitHub checks are passing;
- local verification passed before push;
- the PR body includes a clear risk assessment;
- the PR body includes a deliberately-not-changed section;
- the PR changes are bounded and coherent;
- the PR does not include any forbidden work from section 7;
- the PR does not add direct tool execution, shell execution, browser automation, MCP expansion, uncontrolled scheduler/autonomy, secret handling or destructive operations;
- the PR does not introduce direct Graph Memory mutation by LLM/provider/runtime;
- any memory state-changing capability remains routed through ProposedAction -> DecisionGate -> Decision -> Audit;
- `PROJECT_STATUS.md` is updated if the change is significant.

Preferred merge method:

```bash
gh pr merge <PR_NUMBER> --squash --delete-branch
```

If checks are pending, failed, missing, ambiguous, duplicated in a confusing way, or if the PR includes any state-changing capability whose governance path is unclear, do not auto-merge. Report the PR and wait for human review.

Auto-merge is a speed tool, not an authorization bypass. It may merge safe internal architecture faster, but it must never bypass the project governance principles.

## 11. Verification requirements

Before pushing a PR, run:

```bash
cargo fmt -- --check
cargo check
cargo test
```

When adding a CLI command, also run at least one manual command invocation, for example:

```bash
cargo run -q -p arpagona-cli -- memory proposals --json
```

Use the actual implemented command name.

## 12. LOCO/Ollama delegation rules

LOCO/Ollama may be used for bounded analysis or first-pass low-risk review.

After any delegation, immediately inspect:

```bash
git status --short --branch
```

If LOCO/Ollama created or modified files without explicit permission, revert or remove those changes before continuing.

Unexpected local file creation should be reported as a failure signal. If it repeats or requires correction beyond simple cleanup, create a FailureInsight or a dedicated issue.

## 13. Issue guidance

Prefer working from open GitHub issues when they exist.

Current priority issue:

```text
#47 — Make governed memory-write proposals observable and controllable
```

If no suitable issue exists, create one only when it clarifies a real architectural or implementation target.

## 14. Report format

Every focus-loop report must include:

- trigger;
- guidance files read;
- selected issue or reason no issue was selected;
- branch;
- whether LOCO/Ollama was delegated to;
- local task summary;
- why this work was chosen;
- why the selected PR is ambitious enough for the current priority;
- work completed;
- files changed;
- tests run;
- test result;
- GitHub push status;
- PR link if created;
- auto-merge attempted or skipped, with reason;
- blockers;
- risks;
- deliberately not changed;
- failures observed;
- whether FailureInsights were created;
- recommended next loop.

## 15. Current intent in one sentence

Make governed memory-write proposals observable and controllable first, so ARPAGONA can show what it wants to remember, why, under which permission/policy, and what the next safe human action is before any approved Graph Memory persistence path is expanded.
