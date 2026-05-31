# ARPAGONA Agent Core — Next Focus Loop Handoff

## Current status (GONA 2026-05-31T04:07 UTC — after doctor/preflight V0 merge)

- GitHub/source of truth: `/home/thibaud/arpagona-agent-core`, branch `main`, remote `git@github.com:Arpagona/Agent-core.git`.
- Local `main` is synced with `origin/main` at `c1f9c04ba875ee23d28e482aaf07d29d6a26652e`.
- Open PR list was empty immediately after merging PR #239.
- PR #235 merged: Ollama Intent Provider for `actor run/session`.
- PR #236 merged: Ollama provider hardening / UX / edge-case tests.
- PR #237 merged: actor memory/status/journal readback surfaces with journal redaction.
- PR #238 merged: DV-2026-05-31-001 snapshot integration harness now builds/locates `arpagona-api-server` before spawning it.
- PR #239 merged: `arpagona doctor` local preflight diagnostic.

## Functional capability now available

- `arpagona actor run` and `arpagona actor session` can use deterministic or local Ollama intent providers.
- Actor readback/status/journal surfaces exist for operator visibility.
- Snapshot integration baseline no longer depends on a manually prebuilt API server binary.
- `arpagona doctor` performs a local preflight across git state, CLI/API binaries, Ollama/qwen3.5:9b readiness, tool runtime smoke, and stale secondary workspace copy detection.

## Active brick

PR #241 `feat/doctor-severity-and-process-run-v0` (open):
1. **Doctor severity fix**: `secondary_copy` stale state now `[WARN]` (not `[FAIL]`). JSON includes `severity` field. `all_pass` only considers FAIL checks.
2. **`process run daily-validation` V0**: Quality-gated 4-step workflow (doctor → cargo fmt → cargo check → cargo test). Stops on blocker. Supports human/JSON output.

## Constraints

- One tight PR at a time.
- No generic workflow DSL yet.
- No arbitrary JS/TS process execution.
- No YOLO/forever/scheduler.
- No DeepSeek provider yet.
- No self-improvement/autonomy escalation.
- No hidden external effects; local-only except Ollama readiness checks already covered by doctor.
- Auto-merge clean/green/scope-exact PRs; stop/report on CI red, scope drift, or governance doubt.

## Next action (after PR #241 merges)

Propose next Babysitter-inspired brick: likely `process run` V1 with additional hardcoded processes, or a `process plan` subcommand. See GONA directive for detailed next steps.
