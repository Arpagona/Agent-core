# ARPAGONA Agent Core — Focus Loop Instructions

This file is the canonical instruction file for the scheduled ARPAGONA focus loop.

The project has completed Phase 2. The previous P1-P8 queue, Track A MCP milestones, Track B Holographic Memory milestones, and Phase 2 C/D/E/H milestones are considered delivered unless a regression is found. The focus loop must now advance Phase 3: a bounded Neutral Orchestrator layer that coordinates objectives, context, compute routing, proposal generation, governance decisions and audit outcomes without adding hidden autonomy or external-effect bypasses.

```text
Build hard on internal cognitive architecture. Gate every external effect.
```

## 1. Mission

ARPAGONA Agent Core is not a coding assistant project. It is a local-first professional cognitive runtime and agentic orchestration kernel.

The target runtime chain now has two safe modes:

```text
Proposal mode:
Objective -> Working Memory -> Observations -> Plan -> Assessment -> ProposedAction -> Decision Gate -> Audit -> Reflection -> Governed Learning

Governed tool-call mode:
Objective -> Working Memory -> LLM ToolCall Intent -> Decision Gate -> Tool Runtime/MCP -> Observation -> Audit -> Reflection -> Governed Learning
```

Each run must either:

1. finish, merge or clean work that blocks this chain;
2. implement one substantial milestone from the active Phase 3 roadmap;
3. or report `NO-OP` with the precise blocker.

## 2. Current strategic posture

The Phase 1 alpha foundation and Phase 2 governed runtime layer are complete enough to serve as the base for Phase 3:

- governed cognitive loop demo exists;
- MCP server foundation exists;
- Holographic Memory has durable and governed integration paths;
- Working Memory and Compute Reservoir are integrated enough for alpha scenarios;
- real-model proposal-only paths, governed direct tool-call paths, traceability, operator readbacks, demo scenarios and hardening work are delivered as Phase 2;
- compressed-cognitive-attention exists as a standalone library crate on main, but is not yet integrated into the runtime loop;
- CLI and MCP readback surfaces exist for local supervision.

The active Phase 3 objective is now:

```text
A bounded Neutral Orchestrator that coordinates the existing cognitive/runtime bricks into governed, inspectable work cycles without gaining direct execution authority.
```

The Phase 3 acceleration pillars are:

1. **Neutral Orchestrator V0** — define and implement coordination contracts for objective intake, context assembly, compute routing, proposal routing, Decision Gate outcomes and audit linkage.
2. **Governance-preserving orchestration** — orchestration must route through existing Decision Gate, Tool Runtime/MCP constraints and Audit; it must not become an approval or execution layer.
3. **Memory-aware context assembly** — use Graph Memory, Holographic Memory, Reservoir Echo and compressed-cognitive-attention only as advisory context sources, never as authorization.
4. **Compute-aware delegation** — ask Compute Reservoir for routing advice and record the explanation, without treating compute allocation as permission to act.
5. **Operator inspectability** — expose orchestrator state through read-only CLI/MCP surfaces before any Web Mission Control expansion.
6. **Product-facing scenarios** — keep SME/business demo usefulness visible; avoid architecture for architecture's sake.
7. **Safety boundary preservation** — no unrestricted shell, no unrestricted write, no secrets access, no browser automation, no hidden autonomy, no scheduler expansion.

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
- proposal-only LLM integration first;
- then governed LLM direct tool-calls with audit traces.

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

### 6.1 Active daily-validation gate for 2026-05-28

Until all open `DV-2026-05-28-*` entries are fixed, superseded with evidence, or intentionally deferred with a strong written rationale, the next DEEP/focus-loop pass must not start from the assumption that green `main` means no work is needed.

Before choosing corrective work, read in this order:

1. `DAILY_VALIDATION_BACKLOG.md`;
2. the latest available `daily-agent-core-validation` report/output for 2026-05-28;
3. open PRs related to daily validation, especially PR #139 if still open;
4. the `Recommended Next Day Actions` section of the latest daily-validation report.

Then select exactly one unresolved daily-validation entry to correct. Preferred priority order:

1. `DV-2026-05-28-002` — document missing CLI commands `mcp-governance-audit` and `llm`, then make `bash scripts/check-cli-docs-coverage.sh` pass;
2. `DV-2026-05-28-004` — restore targeted governance/readback regression assertions;
3. `DV-2026-05-28-003` — classify lexical `../` paths as security before filesystem lookup;
4. `DV-2026-05-28-005` — make local Ollama synthesis more specific to the operator request;
5. `DV-2026-05-28-001` — reduce false positives in the conflict-marker scan.

Before changing files, explicitly report:

- selected DV entry;
- why this entry is the highest-priority safe correction now;
- likely affected files;
- acceptance criteria;
- validation commands planned.

Do not open a new non-DV chantier while any `DV-2026-05-28-*` entry remains open unless there is a strong blocker or safety/P0 rationale.

After the correction, create one dedicated technical PR and report:

- modified files summary;
- commands executed;
- `cargo fmt -- --check` result;
- `cargo check` result;
- `cargo test` result;
- if `DV-2026-05-28-002` was selected, `bash scripts/check-cli-docs-coverage.sh` result;
- remaining visible DV backlog entries for the next pass.

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
- Do not treat backlog entries as authorization to add unrestricted shell, hidden autonomy, secrets access, unsafe writes or Decision Gate bypasses.

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

Goal: connect `arpagona cognitive run --llm` to the existing LLM/provider abstraction without granting tool execution power yet.

Target chain:

```text
Objective -> WorkingMemory -> LLM-assisted reasoning -> ProposedAction -> Decision Gate -> Audit
```

Required properties:

- LLM output may enrich working memory, observations, plans and proposals;
- LLM output must never approve actions;
- LLM output must never write memory directly;
- LLM output must never bypass Decision Gate;
- provider, model, prompt summary and response summary should be audit-readable where practical;
- CLI smoke test with `--llm` must demonstrate proposal-only behavior;
- direct tool-calls are not required in C1 and may remain deferred to C2.

### C2 — Governed direct tool-calling by the LLM

Goal: allow direct tool-call intents produced by the LLM, without forcing them to be converted into inert proposals first.

This milestone deliberately **does not prevent direct tool-calls by the LLM**. Instead, it makes direct tool-calling safe by forcing every call through the existing governance envelope.

Target chain:

```text
LLM ToolCall Intent -> Decision Gate -> Tool Runtime/MCP -> Observation -> Audit -> Reflection
```

Required properties:

- the LLM may emit a direct tool-call intent;
- the call must be evaluated by Decision Gate before execution;
- blocked calls must produce audit/readback, not silent failure;
- approved calls must execute only through bounded Tool Runtime/MCP capabilities;
- tool results return as observations, not as final authority;
- no shell, secrets, browser, email or unrestricted write tools;
- no readback-as-authorization behavior;
- tests must prove allowed, blocked and malformed tool-call paths.

### C3 — Prompt, response, decision and risk journaling

Goal: make model interaction auditable after the fact.

Required properties:

- journal prompt summaries;
- journal response summaries;
- journal provider/model metadata;
- journal proposed actions, direct tool-call intents, Decision Gate outcomes and risk levels;
- preserve enough information for debugging without leaking secrets;
- support CLI or MCP readback for recent LLM interaction traces.

### C4 — Compute Reservoir model routing

Goal: integrate Compute Reservoir to choose between local, cloud, small and large model strategies.

Target chain:

```text
Objective / Task -> ComputeRequirement -> ComputeReservoir -> ModelRoute(local/cloud/small/large) -> Explanation -> Audit context
```

Required properties:

- model route selection must be explainable;
- model route may be deterministic or simulated before real provider dispatch;
- cost/latency/privacy trade-offs should be represented where practical;
- local-first preference should be expressible;
- route selection does not itself authorize tool execution;
- audit/readback should show why the model strategy was chosen.

### C5 — Anti-drift and adversarial tests

Goal: protect the C1-C4 model layer against predictable failure modes.

Required test families:

- hallucination containment;
- tool bypass attempts;
- prompt injection attempts;
- malformed tool-call payloads;
- overconfident model claims;
- unsafe memory-write attempts;
- model/provider failure fallback;
- regression tests proving Decision Gate remains mandatory.

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
- show LLM/provider availability once C1 exists;
- read-only only.

### D2 — ProposedAction and tool-call supervision surface

Goal: make the operator able to inspect pending/proposed actions and direct tool-call decisions.

Required properties:

- list recent ProposedActions;
- list recent LLM ToolCall intents;
- show Decision Gate result;
- show risk level and required permissions;
- show associated audit event IDs;
- read-only first.

### D3 — Memory and resonance visibility

Goal: make Holographic Memory understandable to the operator.

Required properties:

- show recent traces;
- show resonance matches;
- show linked decisions/memory IDs;
- show consolidation/fusion evidence;
- show whether a recall hint is advisory only.

### D4 — Minimal Web Mission Control skeleton

Goal: start Web Mission Control only after D1-D3 have clear read-only contracts.

Required properties:

- read-only dashboard;
- no execution buttons initially;
- no approval buttons unless governance semantics are explicitly designed;
- display current runtime state, audit events, proposed actions, tool-call decisions and memory status.

### D5 — Operator approval design study

Goal: design but not necessarily implement human approval semantics.

Required properties:

- distinguish inspect, approve, reject, override and retry;
- specify audit requirements for each operator action;
- specify risk thresholds;
- no hidden auto-approval.

### E1 — SME documentary assistant demo

Goal: produce a product-facing ARPAGONA Agent scenario.

Scenario:

```text
User objective -> ingest/read bounded documents -> extract observations -> propose/direct governed tool actions -> Decision Gate -> audit -> summary
```

Required properties:

- useful to a small business / local SME;
- demoable from CLI first;
- no uncontrolled document ingestion;
- no hidden memory write;
- no external effects beyond governed read-only tooling.

### E2 — Business/prospecting workflow demo

Goal: demonstrate how ARPAGONA Agent can help with a real business workflow.

Candidate scenarios:

- qualification of a client need;
- preparation of a proposal outline;
- analysis of a project brief;
- follow-up action suggestions.

Required properties:

- proposal-only or governed tool-call mode;
- traceable;
- explainable;
- usable in a sales/product demonstration.

### E3 — Local company assistant demo pack

Goal: build a reusable demo pack for ARPAGONA commercial conversations.

Required properties:

- one scripted scenario;
- one sample dataset or synthetic document set;
- one expected output report;
- one explanation of governance/audit value;
- one operator-friendly README.

### E4 — README: demo in 10 minutes

Goal: make the project demonstrable by a human without reading the whole architecture.

Required properties:

- prerequisites;
- commands;
- expected outputs;
- troubleshooting;
- safety boundaries explained simply.

### E5 — Product positioning evidence

Goal: turn technical progress into reusable marketing proof.

Required properties:

- extract 3-5 claims the demo proves;
- map claims to implementation evidence;
- avoid overclaiming autonomy or AGI;
- prepare language usable for ARPAGONA Agent communication.

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
- arbitrary command execution outside the bounded Tool Runtime/MCP governance path;
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
- direct LLM approval of actions;
- direct LLM memory writes.

MCP itself is not forbidden. Unsafe MCP capabilities are forbidden.

LLM direct tool-calling itself is not forbidden. Ungoverned direct tool-calling is forbidden.

## 11. Required verification

For any code change:

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
```

For CLI changes, run the affected commands manually.

For MCP changes, include protocol-level tests or client smoke tests where practical.

For LLM/provider changes, include tests or smoke tests proving either proposal-only behavior (C1) or governed direct tool-call behavior (C2).

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
