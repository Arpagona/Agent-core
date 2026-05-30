# ARPAGONA Actor Run — Acquisition Loop Design Note

> **Status:** Design implemented on branch `feat/actor-session-loop`. PR pending.
> **Prepared by DEEP** per GONA direction (2026-05-30).
> **Governance:** Decision Gate boundaries expressed inline. GONA approval required before any implementation. Thibaud approval required for code affecting governance boundaries, tool execution, or broad autonomous loop behavior.
> **Supersedes:** This document does not supersede `docs/actor-run-command-design.md`. It adds the acquisition loop wrapper around the existing governed `actor run` pipeline.

---

## 1. Objective

The existing `arpagona actor run "<task>"` is a **one-shot command**: it accepts one natural-language task, runs the governed simulation→approval→execution→readback→journal pipeline, and exits.

The **acquisition loop** wraps this one-shot command into a **persistent interactive loop** that:

1. Acquires tasks from a local input source (V0: stdin prompt)
2. Feeds each task through the existing governed pipeline (`actor_run()`)
3. Shows readback
4. Acquires the next task
5. Repeats until explicit exit condition

**Key constraint:** The acquisition loop itself has **no governance authority**. It is a pure orchestration shell. Every individual task still goes through the full Decision Gate pipeline inside `actor_run()`. The loop cannot approve, skip, or accelerate any tool execution.

---

## 2. User-Facing Behavior (V0)

### Entry point

```text
arpagona actor session                    # Interactive stdin acquisition loop (default)
arpagona actor session --max 5            # Run 5 tasks then exit
arpagona actor session --workspace /path   # Run all tasks in specified workspace dir
arpagona actor session --json             # Structured JSON output for each task
```

### Session flow

With `--json`, each task produces a newline-delimited JSON object per task on stdout (one JSON envelope per task, e.g. `{"task": "...", "status": "...", "output": "..."}`). This makes programmatic parsing and test assertions predictable.

```
$ arpagona actor session
[WARNING — Actor Session is a governed local loop. Each task is simulated first; execution requires --approve.]

=== Actor Session ===
Type a task, or /help for commands. Ctrl+C or /quit to exit.

> append meeting notes to docs/log.md

--- Task #1 ---
[Full actor run output: intent → Decision Gate → simulation → approval step → readback → journal]

> read docs/log.md

--- Task #2 ---
[Full actor run output]

> /quit
Session ended. 2 tasks processed.
```

### Session commands (V0)

| Command | Action |
|---------|--------|
| `<task>` | Run task through governed pipeline |
| `/quit` or Ctrl+C | Exit session |
| `/help` | Show session commands |
| `/status` | Show task count and session state |

---

## 3. Architecture

### Position in the CLI hierarchy

```text
ActorCommand
├── Run        # Existing one-shot: actor run "<task>"
└── Session    # New: actor session [--max N] [--workspace PATH] [--json]
```

### How Session wraps Run

The session loop does **not** duplicate actor_run logic. It calls `actor_run()` directly:

```text
Session loop
  │
  ├─ 1. Print header + warning (same as actor run)
  ├─ 2. Loop:
  │      ├─ a. Read line from stdin
  │      ├─ b. If "/quit" or Ctrl+C → break
  │      ├─ c. If "/help" → print commands → continue
  │      ├─ d. If "/status" → print session stats → continue
  │      ├─ e. If task line → call actor_run(task, workspace, ...)
  │      └─ f. Increment task counter
  ├─ 3. Print session summary
  └─ 4. Exit
```

The `--workspace` flag is passed through to every `actor_run()` call, preserving parity with the one-shot command. Each task operates in the same workspace directory for the entire session.

### Existing code integration

No changes to `parse_intent()`, `DeterministicIntentInterpreter`, `IntentInterpreter` trait, `govern_tool_call()`, simulation, execution, readback, or journal paths.

The session loop is **pure orchestration** at the CLI layer — same pattern as `actor_run()` but calling it in a loop.

### File impact

| File | Change | Scope |
|------|--------|-------|
| `crates/cli/src/main.rs` | Add `Session` subcommand variant + `ActorSessionArgs` (with `--max`, `--workspace`, `--json`) + `actor_session()` function + dispatch wiring | ~110-160 lines |
| `crates/cli/src/main.rs` (tests) | Add session integration tests | ~100-150 lines |
| `docs/cli.md`, `docs/actor-run-command-design.md`, `FOCUS_LOOP_NEXT.md` | Documentation updates (Phase 4) | ~20-40 lines total |
| No new files, no new crates, no new dependencies | | |

---

## 4. Acquisition Sources Roadmap

| Phase | Source | Description | When |
|-------|--------|-------------|------|
| **V0** | stdin prompt | Interactive loop. User types tasks. Simplest acquisition. | Now (this design) |
| **V1** | File/batch input | `actor session --file tasks.txt` — one task per line | After V0 stable |
| **V2** | MCP resource read | MCP client reads tasks from resource/tool | After Neutral Orchestrator V0 |
| **Deferred** | Websocket / IPC | Stream of tasks from external process | After V2 + governance review |
| **Deferred** | Scheduler | Cron-like periodic task acquisition | Per AGENT_FOCUS_LOOP.md — scheduler is deferred |

---

## 5. Governance Boundaries (Decision Gate Explicit)

### What the Session loop MAY do autonomously

- Acquire the next task from the current input source
- Call `actor_run()` with the acquired task
- Show readback / display output
- Maintain task counter and session state
- Handle `/help`, `/status`, `/quit`
- Read `--max N` to bound iteration count

### What the Session loop MUST NOT do

- **No bypass of inner governance** — each task goes through `parse_intent()` → Decision Gate → simulation → (optional `--approve`) → execution → readback → journal. The session loop cannot skip, accelerate, or override any phase.
- **No tool execution authority** — the session loop has no tool permissions. All tool calls flow through the existing `govern_tool_call()`.
- **No implicit approval** — the session loop does not inject `--approve` automatically. Each task follows the same simulation-first rule.
- **No interpreter/provider switching** — the session loop cannot change `IntentInterpreter` from Deterministic to Ollama. That requires an explicit design decision and Thibaud approval.
- **No memory writes beyond journal** — the session loop cannot write to Graph Memory, Holographic Memory, or any other store. Only the existing journal path writes.
- **No scheduling** — the session loop is not a scheduler. It does not run on cron, does not queue tasks for later, does not retry failed tasks automatically.
- **No external effects** — no network, no shell, no file writes outside the governed tool path.
- **No secrets access** — the session loop does not read or expose secrets.
- **No persistent state** — session state is in-process only. On exit, it is lost. Persistence would require a design decision.

### Governance invariant

> The acquisition loop adds orchestration only. It does not add governance authority, tool execution, or autonomy expansion. Every bounded task still goes through the full `actor_run` governed pipeline.

### Verification (pre-implementation)

Before any Session implementation is merged, acceptance tests must prove:
1. A session with multiple `read` tasks processes each one independently (same as calling `actor run` N times)
2. A session with an `append` task (without `--approve`) produces simulation-only output — no file mutation
3. A session with `/quit` immediately exits without processing further input
4. `--max N` stops the loop after N tasks
5. Unrecognized tasks return error but the loop continues (does not crash)
6. Ctrl+C signal handling exits gracefully (no panic, partial output printed)

---

## 6. Task Breakdown

### Phase 1 — Scaffold (estimate: 110-160 LoC)

- [ ] Define `ActorSessionArgs` struct with `--max`, `--workspace`, and `--json` flags
- [ ] Add `Session` variant to `ActorSubcommand` enum
- [ ] Wire dispatch in the `Command` handler: `ActorSubcommand::Session(args) => actor_session(args)?`
- [ ] Implement `actor_session()`: loop, input, dispatch to `actor_run()`, counter, exit

### Phase 2 — Session commands (estimate: 50-80 LoC)

- [ ] Implement `/quit` detection (line starts with `/quit` or Ctrl+C via signal handling; best-effort, no new crate)
- [ ] Implement `/help` — print available commands
- [ ] Implement `/status` — print task count and session state

### Phase 3 — Tests (estimate: 100-150 LoC)

Prefer clap parse tests (verify `ActorSessionArgs` parsing for `--max`, `--workspace`, `--json`) plus function-level stdin-driver tests over full interactive integration. Avoid fragile TTY assumptions.

- [ ] `actor_session_single_read_task` — starts session, sends "read file", verifies output, sends "/quit"
- [ ] `actor_session_max_two_tasks` — `--max 2`, sends 4 tasks, verifies only 2 processed
- [ ] `actor_session_workspace` — `--workspace /tmp/test-workspace`, sends "read file", verifies workspace passed to actor_run
- [ ] `actor_session_quit_command` — sends "read file", "/quit", verifies early exit
- [ ] `actor_session_simulate_only` — sends append without --approve, verifies no file created
- [ ] `actor_session_unrecognized_task_continues` — sends garbage, verifies error but loop continues
- [ ] `actor_session_empty_input` — sends empty line, verifies loop continues (no crash)

### Phase 4 — Documentation (estimate: 20-40 LoC)

- [ ] Update `docs/cli.md` with `actor session` command
- [ ] Update `docs/actor-run-command-design.md` with Session cross-reference
- [ ] Update `FOCUS_LOOP_NEXT.md` handoff after implementation

**Total estimate:** ~300-470 lines across all phases. 4 files modified (1 source, 1 test, up to 3 documentation files).

---

## 7. What Is Explicitly Deferred

| Feature | Rationale |
|---------|-----------|
| Ollama `IntentInterpreter` | Separate design decision. Needs Thibaud approval. The acquisition loop calls `actor_run()` which already has the `IntentInterpreter` seam — Ollama can be added independently. |
| File/batch input (`--file`) | Can run outside the loop via shell piping. Add only if user demand emerges. |
| MCP task acquisition | Depends on Neutral Orchestrator V0 being stable first. |
| Websocket/realtime task stream | Requires scheduler governance review. Blocked by AGENT_FOCUS_LOOP.md scheduler deferral. |
| Persistent session state | Session state is ephemeral. Persistence (resume, history) would need a design decision. |
| `--approve` flag per task within session | User can already run `actor run "<task>" --approve` separately. Session is simulation-first. A future shorthand could add `!` prefix (e.g. `!append hello to x.txt`) but deferred. |
| Session timeout / auto-exit | V0 is interactive. Timeout or idle-exit is a later concern. |
| Multi-turn context | Each task is independent. No cross-task working memory, no conversation history sharing. That is a future cognitive loop concern, not an acquisition loop concern. |

---

## 8. Risk Assessment

| Risk | Mitigation |
|------|------------|
| Session loop encourages "batch approval" mindset | Warning header at session start. Each task still simulation-first. No batch `--approve-all`. |
| Signal handling complexity (Ctrl+C) | Best-effort only. `/quit` is sufficient for V0. Defer signal handling to separate PR. |
| Session interferes with existing actor run tests | All session code is new subcommand. Existing `actor_run` tests untouched. |
| Scope creep — someone adds auto-retry or persistence during implementation | **Blocked by this design note.** Session is explicitly ephemeral and non-retrying. Any change requires GONA review of this note. |
| User expects session to remember context between tasks | Session is explicitly stateless across tasks. Multi-turn is a separate design. |
