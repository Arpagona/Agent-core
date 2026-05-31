# ARPAGONA Agent Core — Next Focus Loop Handoff

## Current status (GONA 2026-05-31T07:45 UTC — process plan V0 merged)

- GitHub/source of truth: `/home/thibaud/arpagona-agent-core`, branch `main`, remote `git@github.com:Arpagona/Agent-core.git`.
- Local `main` is synced with `origin/main` at `733d4b4c8c8456a70582d39e9aa151da3fb5e41f`.
- Open PR list was empty immediately after merging PR #245.
- PR #244 merged: process run journal/status V0 — durable, inspectable run records with run IDs, persisted journal, and status readback.
- PR #245 merged: `arpagona process plan daily-validation` — read-only inspection of the daily-validation process.

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

1. Process observability V1: add `arpagona process list` to enumerate journaled process runs.
2. Process recovery V0: design/implement bounded resume semantics for blocked process runs.
3. Product validation: run the full local Beta Usage Lab with `qwen3.5:9b` through the new process/doctor surfaces.
4. Only after the above: evaluate next provider/self-improvement bricks. Do not start DeepSeek or self-improvement until process list/status/resume are solid.
