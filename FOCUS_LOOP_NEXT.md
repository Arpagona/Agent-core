# ARPAGONA Agent Core — Next Focus Loop Handoff

## Current status (DEEP 2026-05-31T17:45 UTC — Ollama UX/smoke hardening)

- GitHub/source of truth: `/home/thibaud/arpagona-agent-core`, branch `feat/ollama-ux-smoke-hardening`, remote `git@github.com:Arpagona/Agent-core.git`.
- Local `main` is synced with `origin/main` at `3c0cef33...`.
- PR #248 merged: `feat: isolate process journals from integration tests (#248)` is merged on main.
- Open PR list: PR #249 — `feat/ollama-ux-smoke-hardening` — Ollama provider UX/smoke-test hardening.

## Changes in this PR

- **Smoke tests**: Added 6 actor-command tests to `scripts/smoke-human-cli.sh`: `actor help`, `actor status`, `actor run` (deterministic), `actor run --json`, `actor session --help`, and `actor run --intent-provider ollama` (with graceful skip if Ollama not reachable).
- **Session UX**: Enhanced `actor session` text-mode `/help` to show the active intent provider (`deterministic` or `ollama`) and Ollama model name when configured. Enhanced `/status` to show the same provider/model context.
- All tests pass: `cargo test --workspace` (all crates), smoke test (11/11 PASS).
- No new providers, tools, or autonomy. All changes are bounded hardening.

## Verification

- `cargo fmt -- --check`: clean
- `cargo check`: clean
- `cargo test --workspace`: all pass
- `bash scripts/smoke-human-cli.sh`: 11/11 PASS (including Ollama intent-provider integration)

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

1. [IN PROGRESS] Ollama UX/smoke hardening — smoke tests + session help/status improvements.
2. Process recovery V0: design/implement bounded resume semantics for blocked process runs.
3. Product validation: run the full local Beta Usage Lab with `qwen3.5:9b` through the new process/doctor surfaces.
4. Only after the above: evaluate next provider/self-improvement bricks.
