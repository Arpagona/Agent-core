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

Move beyond small generic CLI niceties and begin integrating the next major architectural brick:

```text
Graph Memory alpha integration
```

The goal is to make Graph Memory progressively useful, inspectable and connected to the governed runtime path.

The system should now move from:

```text
read-only CLI observability only
```

toward:

```text
Graph Memory visibility -> governed memory-write proposals -> audit-linked memory facts -> approved memory persistence
```

This must still preserve the founding rule:

```text
Agent -> ProposedAction -> DecisionGate -> Decision -> Audit -> controlled effect only if approved
```

## 4. Preferred next work

Prefer one coherent bounded PR per loop. The PR may be substantial if it advances one architectural slice cleanly.

The preferred next milestone is not another generic status/readback nicety. The preferred next milestone is:

```text
First governed memory-write proposal path
```

Minimum acceptable shape:

- introduce or refine memory-write ProposedAction vocabulary;
- add or refine WriteMemory-related permission/risk classification;
- ensure Decision Gate blocks or requires confirmation by default when permission is missing;
- ensure explanatory audit shows why the memory write was blocked, approved or escalated;
- add tests proving memory-write proposals are governed;
- update documentation and `PROJECT_STATUS.md`.

Target behavior:

```text
FailureInsight / correction / important decision observed
-> ProposedAction(type: create_memory_fact or create_failure_insight_memory)
-> DecisionGate
-> Decision
-> Audit
-> Graph Memory write only if approved
```

In the first alpha pass, it is acceptable and even desirable for Decision Gate to block or require confirmation if `WriteMemory` permission is missing.

The key proof is not direct persistence. The key proof is that the system can autonomously identify a candidate memory write, route it through governance, and produce an explanatory audit.

## 5. Secondary useful work

If the preferred memory-write proposal path is blocked by missing foundations, choose the most ambitious enabling slice, not a trivial substitute.

Good enabling slices include:

### Graph Memory status/readback

Add a read-only CLI supervision surface that exposes the current Graph Memory alpha state.

Possible commands:

```bash
arpagona memory status
arpagona memory status --json
```

Expected output should clarify:

- whether Graph Memory support exists in the build;
- which backend is expected/configured, if any;
- whether the SurrealDB adapter exists;
- current alpha limitations;
- what is not implemented yet;
- that the command is read-only and non-authorizing.

### Governed memory-write vocabulary

Introduce explicit domain vocabulary for memory write intentions, without bypassing governance.

Possible action types:

```text
write_memory
create_memory_fact
link_memory_fact
invalidate_memory_fact
create_failure_insight_memory
```

These must be ProposedAction types first. The LLM/provider/runtime must not mutate Graph Memory directly.

### Minimal approved write path for safe internal operational facts

Only after the proposal, permission and audit path is implemented and tested, consider a minimal approved write path.

Scope must be narrow:

- operational/project memory only;
- typed facts only;
- explicit provenance;
- confidence;
- timestamp;
- audit linkage;
- no secrets;
- no personal sensitive memory;
- no hidden context injection.

## 6. Allowed work

Allowed during the current phase:

- Graph Memory read-only status/readback;
- Graph Memory documentation and architecture clarification;
- ProposedAction vocabulary for governed memory writes;
- autonomous memory-write proposals when routed through ProposedAction -> DecisionGate -> Decision -> Audit;
- Decision Gate support for memory-write permissions and risk classification;
- explanatory audit for memory-write proposals;
- tests proving memory write proposals are governed;
- CLI readback for memory status, proposed memory writes or decision/audit linkage;
- narrow operational-memory design around FailureInsight, corrections and project decisions;
- minimal approved memory persistence only after the governed proposal path is proven;
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

Any memory-write design must include, at minimum:

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
- aligned with Graph Memory integration;
- safe by construction;
- documented;
- accompanied by `PROJECT_STATUS.md` update when significant.

Do not split one coherent architectural slice into tiny PRs solely out of caution. It is acceptable for one PR to touch core, decision-gate, CLI, docs and tests if the change is coherent and governed.

Avoid PRs that only add generic CLI polish unless they directly support Graph Memory, governed memory writes, audit explainability or local supervision of the current priority.

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
cargo run -q -p arpagona-cli -- memory status --json
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
#44 — Start Graph Memory alpha integration through governed read-only supervision
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

Build real Graph Memory integration by trusting Hermes/GONA to choose meaningful governed architectural slices: first memory-write proposals, then audit-linked memory facts, then minimal approved persistence, while every state-changing effect remains gated and auditable.
