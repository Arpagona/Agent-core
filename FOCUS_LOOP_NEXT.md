# ARPAGONA Agent Core — Next Focus Loop Handoff

## Current status (DEEP 2026-05-31T20:04 UTC — PR #250 open)

- GitHub/source of truth: `/home/thibaud/arpagona-agent-core`, branch `main`, remote `git@github.com:Arpagona/Agent-core.git`.
- Local `main` is synced with `origin/main` at `3a9313cd272fa124944fe44ea8eaa66fd369964d`.
- PR #250 open on `feat/ollama-provider-argument-schema`: **fix: add explicit per-tool argument schemas to Ollama system prompt** — qwen3.5:9b was returning `file_path` instead of `path` for `read_file`, causing `Missing argument: read_file requires non-empty string 'path'` on Ollama provider. Fixed by adding explicit argument schemas in the prompt. Smoke test verified: `actor run --intent-provider ollama --json "read Cargo.toml"` now passes with proper `path` argument.

## Completed recently

- **Ollama argument schema fix (PR #250)** — qwen3.5:9b inferred `file_path` as the read_file argument because the system prompt only said "arguments must be valid for the chosen tool" without specifying field names. Added per-tool argument schemas (`path`, `content`, `pattern`) to the prompt. All 1042 workspace tests pass. No new providers/tools/autonomy.

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
