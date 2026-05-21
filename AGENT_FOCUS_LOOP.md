# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical operational instruction file for the scheduled ARPAGONA Agent Core focus loop.

The local cron/Hermes job must treat this file as the current source of truth for what to work on, what to avoid, how ambitious to be, and how to report the result.

The local cron should stay minimal: pull the repository, read this file and the canonical project files, perform at most one bounded increment, then report.

## 1. Files that must be read at the start of every loop

Every focus-loop run must read, in this order:

1. `AGENT_CONTEXT.md`
2. `PROJECT_OBJECTIVES.md`
3. `PROJECT_STATUS.md`
4. `AGENT_FOCUS_LOOP.md`
5. `docs/operating-doctrine.md`
6. `docs/development-acceleration.md`
7. Any docs or source files directly relevant to the chosen task

If these files conflict, prioritize safety and governance first, then `AGENT_FOCUS_LOOP.md` for current operational priority, then `PROJECT_OBJECTIVES.md` for long-term architecture.

## 2. Current strategic priority

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

## 3. Current recommended next work

Prefer one bounded PR per loop.

The preferred next sequence is:

### Step 1 — Graph Memory status/readback

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

### Step 2 — Governed memory-write proposal vocabulary

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

### Step 3 — First governed autonomous memory-write proposal

Allow the runtime to propose a memory write when an important operational correction, failure or decision is observed.

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

The key proof is not direct persistence. The key proof is that the system autonomously identifies a candidate memory write, routes it through governance, and produces an explanatory audit.

### Step 4 — Minimal approved write path for safe internal operational facts

Only after Step 2 and Step 3 are implemented and tested, consider a minimal approved write path.

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

## 4. Allowed work

Allowed during the current phase:

- Graph Memory read-only status/readback;
- Graph Memory documentation and architecture clarification;
- ProposedAction vocabulary for governed memory writes;
- Decision Gate support for memory-write permissions and risk classification;
- explanatory audit for memory-write proposals;
- tests proving memory write proposals are governed;
- CLI readback for memory status, proposed memory writes or decision/audit linkage;
- narrow operational-memory design around FailureInsight, corrections and project decisions;
- `PROJECT_STATUS.md` updates reflecting each significant change.

## 5. Forbidden work

Do not implement yet:

- direct Graph Memory mutation by LLM/provider/runtime;
- broad autonomous memory writing;
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
- shell access as an ARPAGONA capability;
- browser automation;
- MCP expansion;
- scheduler/autonomy expansion beyond the existing external cron;
- Mission Control Web expansion;
- readback-as-authorization behavior;
- destructive operations.

## 6. Mandatory safeguards for memory work

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

## 7. PR policy

Each loop should usually create at most one small PR.

A good PR is:

- bounded;
- testable;
- aligned with Graph Memory integration;
- safe by construction;
- documented;
- accompanied by `PROJECT_STATUS.md` update when significant.

Avoid PRs that only add generic CLI polish unless they directly support Graph Memory, governed memory writes, audit explainability or local supervision of the current priority.

## 8. Verification requirements

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

## 9. LOCO/Ollama delegation rules

LOCO/Ollama may be used for bounded analysis or first-pass low-risk review.

After any delegation, immediately inspect:

```bash
git status --short --branch
```

If LOCO/Ollama created or modified files without explicit permission, revert or remove those changes before continuing.

Unexpected local file creation should be reported as a failure signal. If it repeats or requires correction beyond simple cleanup, create a FailureInsight or a dedicated issue.

## 10. Issue guidance

Prefer working from open GitHub issues when they exist.

Current priority issue:

```text
#44 — Start Graph Memory alpha integration through governed read-only supervision
```

If no suitable issue exists, create one only when it clarifies a real architectural or implementation target.

## 11. Report format

Every focus-loop report must include:

- trigger;
- guidance files read;
- selected issue or reason no issue was selected;
- branch;
- whether LOCO/Ollama was delegated to;
- local task summary;
- why this work was chosen;
- work completed;
- files changed;
- tests run;
- test result;
- GitHub push status;
- PR link if created;
- blockers;
- risks;
- deliberately not changed;
- failures observed;
- whether FailureInsights were created;
- recommended next loop.

## 12. Current intent in one sentence

Build toward real Graph Memory by first making it visible, then allowing agents to propose governed memory writes, then connecting those proposals to Decision Gate and Audit before any approved persistence path is expanded.
