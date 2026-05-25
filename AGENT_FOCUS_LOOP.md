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
7. `FOCUS_LOOP_NEXT.md`
8. Any docs, issues or source files directly relevant to the chosen task

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

Move beyond memory-write vocabulary and make governed memory-write proposals observable, controllable and suitable for controlled local persistence tests.

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
governed memory-write proposals -> observable/controllable proposal readback -> controlled local Graph Memory persistence -> audit-linked memory facts
```

Real Graph Memory persistence is allowed at this stage for local alpha testing, provided it is explicit, governed, inspectable, manipulable and reversible enough for development use.

The immediate goal is to let a human or local supervision surface inspect, understand, approve and test proposed memory writes. Persistence is acceptable only when the proposal, permission, audit and readback path is clear.

This must still preserve the founding rule:

```text
Agent -> ProposedAction -> DecisionGate -> Decision -> Audit -> controlled effect only if approved
```

## 4. Preferred next work

Prefer one coherent bounded PR per loop. The PR may be substantial if it advances one architectural slice cleanly.

The preferred next milestone is:

```text
Observable, controllable and optionally persistable governed memory-write proposals
```

A strong next PR should allow a supervisor to answer:

```text
What memory write did the agent propose?
Why did it propose it?
What would be remembered?
What provenance/source supports it?
What confidence does it carry?
What permission/policy blocked, approved or escalated it?
Was it persisted, and where?
How can it be inspected or superseded later?
What is the next safe human action?
```

Good implementation shapes include one or more of the following if they form a coherent slice:

### Today's functional-alpha objectives

For today's remaining focus-loop runs, raise the ambition from isolated memory plumbing to a demonstrable functional alpha path.

The cron should aim to move ARPAGONA Agent Core closer to something a human can run locally and understand as a working governed agent core, not merely a library of disconnected primitives.

Minimum acceptable objective for today:

```text
A reproducible local demo or test proving one complete governed learning loop from signal to readback.
```

Strong objective for today:

```text
A CLI-accessible alpha demo path that creates or simulates a safe operational signal, materializes a governed proposal, passes through Decision Gate, records/validates audit linkage, persists approved Graph Memory state, and proves the result through readback.
```

Stretch objective for today, only if the strong objective is already safe and passing:

```text
A small operator-facing command or documented demo recipe that makes the governed loop easy to run again locally, without adding uncontrolled autonomy or external effects.
```

The loop should prefer implementation work that creates an end-to-end product slice over another narrow schema/readback field. A product slice may still be local, alpha, simulated and test-driven; it does not need real external tool execution.

Today's work should make at least one of these user-visible statements true:

- `I can run one command or test and see ARPAGONA learn from a correction through the governed path.`
- `I can inspect what was proposed, why it was approved or blocked, what audit event proves it, and what memory artifact resulted.`
- `I can repeat the demo locally without relying on hidden context or manual database surgery.`
- `I can see a clear next step toward a real alpha operator workflow.`

Do not choose work that only improves internal elegance unless it directly unlocks one of those statements.

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

### Controlled local persistence path

If the foundations are ready, add a minimal real Graph Memory persistence path for safe internal operational/project memory.

Acceptable first persistence targets:

- approved `create_memory_fact`;
- approved `create_failure_insight_memory`;
- audit-linked operational memory facts;
- local SurrealDB-backed persistence when the adapter is suitable;
- development/test-only persistence with explicit configuration;
- CLI readback of persisted facts.

The key proof is not broad autonomy. The key proof is that memory can be proposed, governed, persisted only when approved, and inspected afterward.

### Immediate end-to-end scenario target for the next loop

The next focus loop should prioritize a small but demonstrable safe scenario over another isolated readback improvement.

Preferred scenario:

```text
observed failure or correction -> proposed FailureInsight memory write -> Decision Gate decision -> audit event -> approved local Graph Memory persistence -> CLI/readback proof that the persisted insight exists and remains linked to the proposal, decision and audit event
```

The goal is to produce a local alpha proof that ARPAGONA can learn from a failure or correction without bypassing governance.

Acceptance criteria for this scenario:

- create or reuse a bounded test fixture representing a failure, correction or missing-context signal;
- materialize a governed `create_failure_insight_memory` proposal;
- pass the proposal through Decision Gate without adding any bypass;
- record or validate an audit linkage between proposed action, decision and persisted memory artifact;
- persist only after an approved decision;
- expose a readback path proving the persisted FailureInsight can be inspected afterward;
- update `PROJECT_STATUS.md` if implementation status changes;
- keep the PR bounded to this end-to-end memory-learning proof.

A strong implementation may be mostly tests and CLI/readback glue if that is the safest path. However, avoid another PR that merely adds fields without proving the full governed learning loop.

This scenario must not introduce broad autonomy, hidden prompt injection, direct provider/runtime memory mutation, shell/tool execution, scheduler expansion, browser automation, MCP, secrets handling, destructive operations or Mission Control Web work.

If the full scenario is not yet safe in one PR, implement the most ambitious missing slice that directly unblocks it and explicitly report what remains for the next loop.

## 5. Secondary useful work

If controlled persistence is not yet safe, choose the most ambitious enabling slice, not a trivial substitute.

Good enabling slices include:

- tests proving `create_memory_fact` and `create_failure_insight_memory` appear clearly in decision/audit summaries;
- serialization/readback improvements for `MemoryWriteIntent`;
- documentation of the lifecycle from memory-write proposal to approved persistence;
- CLI listing of pending/proposed actions filtered to memory-write action types;
- improved explanatory audit metadata for memory-write proposals;
- minimal test fixture proving approved memory facts can later be read back.

Do not jump to broad persistent memory unless the proposal, permission, audit and readback path is clear and tested.

For today only, secondary work should be accepted only if it clearly states which part of the functional-alpha loop it unblocks:

```text
signal -> proposal -> decision -> audit -> approved persistence -> readback -> repeatable local demo
```

If a proposed task does not move at least one arrow in this chain, defer it.

## 6. Allowed work

Allowed during the current phase:

- read-only CLI/audit readback for memory-write proposals;
- JSON output for memory proposal inspection;
- proposal formatting and serialization helpers;
- tests proving proposed memory writes are inspectable;
- tests proving blocked and confirmation-required memory-write proposals remain governed;
- controlled local Graph Memory persistence for safe internal operational/project facts;
- CLI readback for persisted memory facts;
- local SurrealDB-backed persistence if explicitly configured and safe;
- Graph Memory documentation and architecture clarification;
- Decision Gate support for memory-write permissions and risk classification;
- explanatory audit for memory-write proposals;
- narrow operational-memory design around FailureInsight, corrections and project decisions;
- local demo fixtures, examples or CLI commands that exercise the governed path without external side effects;
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

Important clarification: real Graph Memory persistence is no longer forbidden if it is local, explicit, governed, auditable, inspectable and limited to safe internal operational/project memory.

## 8. Mandatory safeguards for memory work

Any memory-write proposal, persistence path or design must include, at minimum:

- typed memory target;
- explicit provenance/source;
- confidence level;
- timestamp;
- actor/source;
- reason for remembering;
- Decision Gate result;
- audit event linkage;
- CLI readback or another explicit inspection path;
- future invalidation/supersession path, even if not fully implemented yet.

Sensitive or personal facts must require human confirmation. The first alpha should focus on operational/project memory, not private personal memory.

## 9. PR policy

Each loop should usually create one coherent bounded PR.

A good PR is:

- architecturally meaningful;
- bounded;
- testable;
- aligned with observable/controllable Graph Memory proposals or controlled persistence;
- safe by construction;
- documented;
- accompanied by `PROJECT_STATUS.md` update when significant.

Do not split one coherent architectural slice into tiny PRs solely out of caution. It is acceptable for one PR to touch core, decision-gate, graph-memory, CLI, docs and tests if the change is coherent and governed.

Avoid PRs that only add generic CLI polish unless they directly support memory proposal observability, governed memory writes, controlled persistence, audit explainability or local supervision of the current priority.

For today's functional-alpha push, a PR is considered ambitious enough only if it either:

- proves the governed learning loop end-to-end;
- adds a repeatable local command or demo recipe for that loop;
- or removes a concrete blocker that prevents the next loop from delivering that demo.

### PR creation and reporting verification

A pushed branch is not a pull request.

A GitHub `/pull/new/<branch>` URL is only a PR creation page. It must never be reported as a created PR, a PR link, or proof that a PR exists.

A pull request may be reported as created only when GitHub returns a real PR number and a canonical URL of this form:

```text
https://github.com/<owner>/<repo>/pull/<number>
```

After pushing a branch, run a verification command equivalent to:

```bash
gh pr list --head <branch> --json number,title,url,state,headRefName,baseRefName
```

If no PR exists, attempt PR creation when credentials permit:

```bash
gh pr create --base main --head <branch> --title "<title>" --body "<body>"
```

After creating a PR, verify it again with `gh pr list --head <branch>` or `gh pr view <number>`.

Final reports must use exactly one of these states:

```text
PR created: #<number> <https://github.com/<owner>/<repo>/pull/<number>>
```

or:

```text
Branch pushed, PR not created: <reason>. PR creation URL: https://github.com/<owner>/<repo>/pull/new/<branch>
```

Do not write `PR link` unless the URL is a canonical `/pull/<number>` URL.

No PR number means no PR.

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
- any real persistence path is local/test-oriented, explicit, inspectable and documented;
- any local demo remains simulated/internal and does not create external side effects;
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

When adding controlled persistence, include at least one local readback verification showing that an approved persisted memory fact can be inspected afterward.

For today's functional-alpha objective, the report should include the exact local command, test name or demo recipe that proves the implemented loop or blocker removal.

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

For today's push, creating a new issue is acceptable if it frames a concrete functional-alpha milestone, for example:

```text
Deliver repeatable governed FailureInsight learning demo
```

Do not create vague roadmap issues when a direct implementation PR is possible.

## 14. Next-pass handoff rule

Every focus-loop run must leave a concrete instruction for the next run by updating `FOCUS_LOOP_NEXT.md` before it finishes.

The handoff must be short, operational and immediately actionable. It is not a roadmap and not a long report. It should capture the best next move discovered during the run.

At the end of every run, replace the current handoff with this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Rules:

- write one next step, not a list of ten possibilities;
- prefer the next action that most directly advances the functional-alpha chain;
- include the exact test, command or readback the next run should try first;
- mention any known blocker that the next run should not rediscover from scratch;
- do not use the handoff to bypass `AGENT_FOCUS_LOOP.md`, safety, Decision Gate or project status;
- if the run fully completed the previous handoff, write the next natural continuation;
- if the run failed, write the smallest correction or diagnostic the next run should perform.

The next run must read `FOCUS_LOOP_NEXT.md`, compare it with the current strategic priority, and either follow it or explicitly explain why another action is more coherent or safer.

## 15. Report format

Every focus-loop report must include:

- trigger;
- guidance files read;
- selected issue or reason no issue was selected;
- branch;
- whether LOCO/Ollama was delegated to;
- local task summary;
- why this work was chosen;
- why the selected PR is ambitious enough for the current priority;
- which part of the functional-alpha chain was advanced;
- exact command, test or demo recipe proving the advance;
- work completed;
- files changed;
- tests run;
- test result;
- GitHub push status;
- PR creation status, using exactly one of:
  - `PR created: #<number> <https://github.com/<owner>/<repo>/pull/<number>>`
  - `Branch pushed, PR not created: <reason>. PR creation URL: https://github.com/<owner>/<repo>/pull/new/<branch>`
- auto-merge attempted or skipped, with reason;
- blockers;
- risks;
- deliberately not changed;
- failures observed;
- whether FailureInsights were created;
- recommended next loop;
- `FOCUS_LOOP_NEXT.md` updated, with a one-line summary of the next-pass instruction.

## 16. Current intent in one sentence

Make governed memory-write proposals observable, controllable and suitable for controlled local persistence tests, then turn that into a repeatable local functional-alpha demo where ARPAGONA learns from a safe correction through ProposedAction, Decision Gate, Audit, approved Graph Memory persistence and readback.