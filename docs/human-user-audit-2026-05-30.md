# ARPAGONA human-user audit — 2026-05-30

Scope: black-box use of the local ARPAGONA CLI as a human operator trying to use it for a concrete business objective: `Prépare un plan de lancement commercial pour une formation IA rentable`.

This audit is intentionally product-facing: what feels broken, fake, confusing or slow when using ARPAGONA as a normal operator.

## Environment observed

- Repo: `/home/thibaud/arpagona-agent-core`
- Branch observed during audit: `feat/p3-audit-cycle-trace-bridge`
- Existing untracked strategic doc: `docs/steroid-hermes-roadmap.md`
- Direct binary tested: `target/debug/arpagona`
- `cargo run` was not a reliable human path during the audit because another DEEP cargo process was compiling and holding/contending on the shared build directory.
- Long-running processes were present outside Hermes process tracking:
  - `target/debug/arpagona-api-server`
  - `target/debug/arpagona chat --provider ollama`

## What works

### 1. Top-level command discovery exists

`arpagona --help` presents a broad command surface: `run`, `chat`, `status`, `memory`, `tool`, `cognitive`, `compute`, `orchestrator`, `audit`, etc.

This proves there is more than a bare façade.

### 2. `arpagona run <objective>` is simple and pleasant

Observed output:

```text
❍ ARPAGONA
───────────────────────────────
Objective   Prépare un plan de lancement commercial pour une formation IA rentable
Action      ReadDocument at Low
Decision    ✅ Approved
Summary     Approved
Cycle       oc-...
```

This is the right direction for a human entry point.

### 3. Full orchestrator trace exists behind the pretty run output

`arpagona orchestrator run --objective "..." --json --trace` produced structured `CycleTrace` JSON with:

- `cycle_id`
- `objective_text`
- context sources: `graph_memory`, `holographic_memory`, `reservoir_echo`
- `compute_route_label`
- `compute_route_justification`
- `action_type`
- `decision_status`
- `audit_event_count`
- `failure_insight_candidates`

This confirms that `run` is not purely fake. There is a real orchestrator trace underneath.

### 4. Compute routing preview is real enough for alpha

`arpagona compute routing` shows resource selection, local/cloud trade-off, cost, latency, sensitivity and provider mapping.

This is a strong differentiator if connected to the main mission loop.

## What is wrong

### P0 — Human path is unreliable when using `cargo run`

As a normal developer/operator, `cargo run -q -p arpagona-cli -- status` became unusable during the audit because another DEEP process was compiling in the same repo and contending for the build directory.

Impact:

- operator sees hanging commands;
- test feedback becomes noisy;
- GONA/DEEP can block each other;
- this makes the system feel unstable even when `target/debug/arpagona` works.

Fix direction:

- Standardize human testing on `target/debug/arpagona` after a known build, or isolate DEEP build target dir.
- Consider `CARGO_TARGET_DIR=target/gona` / `target/deep` per agent profile.
- Add `scripts/arpagona-smoke.sh` that builds once, then runs binary commands with timeouts.

### P0 — Background process hygiene is bad

Observed process list included long-running API server and chat sessions outside current Hermes tracking.

Impact:

- status may report API health because an old server is running;
- chat/API state may be stale;
- future tests become non-reproducible;
- port 3000 may be occupied silently.

Fix direction:

- Add `arpagona doctor` or `scripts/dev-processes.sh` to show active API/chat processes and port ownership.
- Chat/API startup should print PID and state file location.
- Add a clean shutdown command/script for local dev.

### P0 — `run` is pleasant but shallow

`arpagona run` gives a clean output, but the result is essentially:

```text
Action ReadDocument at Low
Decision Approved
Summary Approved
```

For a human business objective, this is not yet useful. It does not produce a plan, ask a good question, inspect local context, or explain next safe step.

Impact:

- feels like a demo formatter rather than a useful assistant;
- validates Thibaud's fear: the surface is ahead of the capability;
- the operator gets approval metadata, not help.

Fix direction:

- `run` must include an operator-facing `Next step` generated from real cycle state.
- If the simulated proposal is generic, say so and offer `--llm` / `--inspect` / `--trace` paths.
- Never let the default output stop at “Approved”.

### P0 — Chat experience is behind the newer steroid-Hermes UX

On the observed branch, `chat --provider ollama` still shows old banner/help:

- `Cognitive Runtime Alpha`
- `Read-only mode - nothing is executed directly`
- no `/mission`
- no `/banner`
- no Mission Control brief

Impact:

- branch drift: the better chat UX work is not present on this branch;
- user experience varies depending on branch;
- merge/rebase discipline is blocking product coherence.

Fix direction:

- Decide the source branch for UX and rebase/merge coherently.
- Avoid parallel UX branches.
- Add snapshot tests for chat banner/help so regressions are visible.

### P1 — Ollama chat gives a real direct answer, but still not a Mission Control experience

Correction after retest: mock provider is not a valid product test path. The same business prompt was retested with Ollama:

```text
Aide-moi à lancer une offre de formation IA rentable sans action externe
```

Ollama chat answered with a direct structured plan:

```text
DirectReply: Voici une stratégie complète pour lancer une offre de formation IA rentable :

1. Analyse du marché
2. Structure de l'offre
3. Canal d'acquisition
4. Modèle de revenus
5. Rentabilité
6. Lancement
```

This is materially better than mock: it does not propose an irrelevant `simulate_email`; it gives useful strategic content.

Remaining product issue:

- the answer is generic LLM advice, not yet ARPAGONA-grade Mission Control;
- no explicit mission framing;
- no runtime evidence;
- no safe next-step separation;
- no memory/context signal;
- no clear bridge to orchestrator/audit/decision gate.

Fix direction:

- Treat `--provider ollama` as the default human/product test path.
- Keep mock only for deterministic unit tests.
- Add intent classifier / simple routing before proposing actions:
  - direct answer / plan;
  - clarifying question;
  - read-only inspection;
  - proposed action.
- For business/strategy prompts, Ollama/direct answer is the floor; Mission Control framing is the target.

### P1 — Command grammar is inconsistent

Examples:

- `arpagona run "objective"` accepts positional objective.
- `arpagona cognitive run "objective"` fails and requires `--objective`.
- `arpagona compute routing "objective"` fails and requires `--purpose`.
- Chat has `/actions`, but shell command is not `arpagona action list`; the actual action subcommands are not obvious from operator habit.

Impact:

- users guess wrong;
- commands feel like internal APIs, not product CLI;
- the system appears less polished than the banner suggests.

Fix direction:

- Normalize the common objective grammar:
  - `arpagona run <objective>`
  - `arpagona cognitive run <objective>` or alias to `--objective`
  - `arpagona compute routing <purpose>` or alias to `--purpose`
- Add intuitive aliases: `actions list`, `audit list`, `mission status`.

### P1 — Status output is too noisy and stale

`arpagona status` output included many pending actions from unrelated previous tests about `Cargo.toml`.

Impact:

- status is not scoped to current mission;
- operator sees stale noise;
- it undermines confidence in “Mission Control”.

Fix direction:

- Add mission/workspace/task scoping defaults.
- Show top-level health first, then stale pending actions only behind `--detail`.
- Add stale-age grouping and cleanup suggestions.

### P1 — Memory status is honest but not useful yet

`arpagona memory status` clearly says Graph Memory is compiled but not configured, and lists many not-implemented pieces.

Impact:

- honest alpha state is good;
- but as a product, it says “memory advantage not active”.

Fix direction:

- Add a local zero-config memory demo store for alpha.
- Provide `memory doctor` with exact setup steps.
- Connect at least one read-only memory recall into `run --trace`.

### P2 — Audit exists but is not yet operator-useful

`audit list` returned `No audit events`, while orchestrator trace reported `audit_event_count: 1`.

Impact:

- audit is present in cycle but not persisted/readable through the obvious audit command;
- this weakens the “inspectable runtime” promise.

Fix direction:

- Prioritize audit spine: `orchestrator run --save-audit`, `audit list-from-dir`, and trace/audit bridge.

## Recommended action plan

### Sprint 0 — Stop the bleeding / make testing reproducible

1. Kill/clean stale dev processes or at least expose them via `doctor`.
2. Add per-agent `CARGO_TARGET_DIR` or agree that GONA uses binary smoke tests while DEEP compiles.
3. Create `scripts/smoke-human-cli.sh`:
   - build once;
   - run `arpagona --help`;
   - run `arpagona run "..."`;
   - run `orchestrator run --json --trace`;
   - run `compute routing --purpose "..."`;
   - run `memory status`;
   - run `audit list`;
   - all with timeouts.

Acceptance: a human smoke report runs in under 60 seconds after build, with no hanging cargo locks.

### Sprint 1 — Make `run` actually useful

1. Keep pretty output, but add:
   - `Understanding`
   - `Useful next step`
   - `Runtime evidence`
   - `Trace hint`
2. If context is empty, say it plainly: “No memory/context found yet; this is a deterministic starter plan.”
3. Add `--trace` and `--json` to top-level `run`, or show the command to inspect the trace.

Acceptance: for a business objective, default `run` produces something actionable, not just “Approved”.

### Sprint 2 — Unify objective grammar

1. Accept positional objective for:
   - `cognitive run`
   - `compute routing`
   - maybe `orchestrator run`
2. Keep flags as explicit alternatives.
3. Add tests for human-guessable command forms.

Acceptance: common user guesses work instead of producing clap errors.

### Sprint 3 — Merge chat UX with real mission loop

1. Rebase/merge the premium chat banner/help work into the active branch.
2. Add `/mission <objective>`.
3. Route chat natural prompts through the same mission framing as `run`.
4. Mock provider should not default every prompt to `simulate_email`.

Acceptance: chat and run feel like the same product, not two separate demos.

### Sprint 4 — Audit spine

1. Persist audit events from orchestrator cycles.
2. Add `audit list-from-dir`.
3. Add `orchestrator cycles --json` audit event breakdown.

Acceptance: if a cycle says `audit_event_count: 1`, the operator can inspect that event from CLI.

### Sprint 5 — Memory signal, not memory theatre

1. Add local zero-config memory/demo recall.
2. Show memory source status in `run --trace`.
3. Add Failure-to-Insight candidate collection into the visible loop.

Acceptance: at least one memory signal can influence context assembly visibly and safely.

## Bottom line

ARPAGONA has real runtime bricks. But the current human experience still exposes too many internal seams:

- hanging cargo/dev-process interference;
- branch drift between UX experiments;
- shallow default `run` output;
- chat proposes irrelevant actions;
- command grammar inconsistency;
- status/audit not scoped enough.

The next product work should not be “more façade”. It should be: reliable smoke path, useful `run`, unified grammar, coherent chat, persistent audit spine.
