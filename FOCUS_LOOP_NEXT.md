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

## Active brick launched to DEEP

Topic: `arpagona-process-run-v0`
Message: `20260531T040712494142Z-1c1a84`

Required first correction:
- `arpagona doctor` currently reports stale secondary workspace copy as `[FAIL] secondary_copy` and `all_pass=false`.
- This should be a WARN/non-blocker unless the canonical repo/product validation is actually blocked.
- DEEP must fix severity semantics before or inside the next tight PR.

Next implementation target:
- Babysitter-inspired `process run` V0 / quality-gated workflow skeleton.
- Preferred command: `arpagona process run daily-validation`.
- One hardcoded process only for V0.
- Steps should be explicit/local: doctor/preflight, `cargo fmt -- --check`, `cargo check`, focused snapshot integration test if appropriate, `cargo test`, structured report.

## Constraints

- One tight PR at a time.
- No generic workflow DSL yet.
- No arbitrary JS/TS process execution.
- No YOLO/forever/scheduler.
- No DeepSeek provider yet.
- No self-improvement/autonomy escalation.
- No hidden external effects; local-only except Ollama readiness checks already covered by doctor.
- Auto-merge clean/green/scope-exact PRs; stop/report on CI red, scope drift, or governance doubt.

## Next action

Read DEEP mail for `arpagona-process-run-v0`, verify any open PR live on GitHub, auto-merge if clean/green/scope-exact, then sync `main` and update this file again.
