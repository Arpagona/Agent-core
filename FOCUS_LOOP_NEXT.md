# ARPAGONA Agent Core — Next Focus Loop Handoff

## Current status (DEEP 2026-05-31T14:45 UTC — process list V0 PR in progress)

- GitHub/source of truth: `/home/thibaud/arpagona-agent-core`, branch `feat/process-list-v0`, remote `git@github.com:Arpagona/Agent-core.git`.
- Local `main` is synced with `origin/main` at `eda9ce78ada8954ba41f52e90a0175e647375fe5`.
- Open PR list: PR #247 — `arpagona process list V0` — bounded command to enumerate persisted process run journals.
- PR #244 merged: process run journal/status V0 — durable, inspectable run records with run IDs, persisted journal, and status readback.
- PR #245 merged: `arpagona process plan daily-validation` — read-only inspection of the daily-validation process.
- PR #246 merged: handoff docs updated after process plan merge.

## Functional capability now available

- `arpagona actor run` and `arpagona actor session` can use deterministic or local Ollama intent providers.
- Actor readback/status/journal surfaces exist for operator visibility.
- Snapshot integration baseline no longer depends on a manually prebuilt API server binary.
- `arpagona doctor` performs a local preflight across git state, CLI/API binaries, Ollama/qwen3.5:9b readiness, tool runtime smoke, and stale secondary workspace copy detection.
- Doctor warnings are non-blocking: stale secondary workspace copy reports `severity: "warn"`, while true blockers return errors.
- `arpagona process run daily-validation` runs a Babysitter-inspired quality-gated V0 workflow: doctor → `cargo fmt -- --check` → `cargo check` → `cargo test`.
- Every `process run` invocation gets a deterministic run ID and persists a durable JSON journal at `~/.arpagona/process-journal/<run_id>.json`.
- `arpagona process status --last` and `arpagona process status <run-id>` provide readback of past run records.
- `arpagona process plan daily-validation` shows the process steps without executing doctor/cargo or writing journals.
- `arpagona process list` enumerates all persisted process run journals from `~/.arpagona/process-journal/`, newest first, with `--json` output. Handles empty and corrupt journals gracefully.

## Latest local verification

- `arpagona process plan daily-validation --json`: valid JSON, `total_steps: 4`, read-only description.
- PR #245 CI: green before merge.
- Repository state after sync: `main...origin/main`, clean.

## Constraints still active

- One tight PR at a time.
- No generic workflow DSL yet.
- No arbitrary JS/TS process execution.
- No YOLO/forever/scheduler.
- No DeepSeek provider yet.
- No self-improvement/autonomy escalation.
- No hidden external effects; local-only except Ollama readiness checks already covered by doctor.
- Auto-merge clean/green/scope-exact PRs; stop/report on CI red, scope drift, or governance doubt.

## Recommended next priorities

1. [DONE] Process observability V1: `arpagona process list` — enumerate journaled process runs. (PR #247).
2. Process recovery V0: design/implement bounded resume semantics for blocked process runs.
3. Product validation: run the full local Beta Usage Lab with `qwen3.5:9b` through the new process/doctor surfaces.
4. Only after the above: evaluate next provider/self-improvement bricks. Do not start DeepSeek or self-improvement until process list/status/resume are solid.
