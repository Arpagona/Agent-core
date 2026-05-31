# ARPAGONA Agent Core — Next Focus Loop Handoff

## Current status (DEEP 2026-05-31T18:15 UTC — post PR #249 merge refresh)

- GitHub/source of truth: `/home/thibaud/arpagona-agent-core`, branch `main`, remote `git@github.com:Arpagona/Agent-core.git`.
- Local `main` is synced with `origin/main` at `b20815d1eecbc700ed16b87f219ec1dd4601406c`.
- PR #249 merged at `b20815d1`: **feat: add Ollama provider UX/smoke-test hardening** — smoke tests (6 deterministic actor-command tests + 5 Ollama integration tests via `smoke-human-cli.sh`), session help/status provider display, bounded hardening only. Main is clean — no open PRs.
- PR #248 merged: **feat: isolate process journals from integration tests** — journals no longer leak into integration-test assertion output.

## Completed recently

- **Ollama UX/smoke hardening (PR #249)** — shipped 11 smoke tests (deterministic + Ollama), `/help` and `/status` now show intent-provider and Ollama model context. All bounded hardening, no new providers/tools/autonomy.

## Planned next priorities

1. [PENDING] Process recovery V0: design/implement bounded resume semantics for blocked process runs.
2. Product validation: run the full local Beta Usage Lab with `qwen3.5:9b` through the new process/doctor surfaces.
3. Only after the above: evaluate next provider/self-improvement bricks.

## Constraints still active

- One tight PR at a time.
- No generic workflow DSL yet.
- No arbitrary JS/TS process execution.
- No YOLO/forever/scheduler.
- No DeepSeek provider yet.
- No self-improvement/autonomy escalation.
- No hidden external effects; local-only except Ollama readiness checks already covered by doctor.
- Auto-merge clean/green/scope-exact PRs; stop/report on CI red, scope drift, or governance doubt.
