# ARPAGONA Agent Core — Daily Validation Backlog

This file is the structured handoff from the midnight daily validation run to the morning focus loop.

Purpose:
- capture bugs, regressions, safety gaps, usability frictions and beta-test findings observed by the daily validation run;
- keep `FOCUS_LOOP_NEXT.md` focused on one concrete next action;
- give the 7am focus loop factual evidence before it chooses work.

Rules:
- The midnight daily validation run must update this file at the end of every run.
- The 7am focus loop must read this file before selecting a milestone.
- Keep entries evidence-first and bounded. Do not add vague wishes.
- Do not treat entries as authorization to add broad capabilities, bypass governance or expand execution.
- Close or update entries when the issue is fixed, superseded or intentionally deferred.

## Open candidates

### DV-2026-05-26-001 — Path escape attempts should be reported as security blocks
- source: daily validation 2026-05-26
- category: safety observability
- severity: low
- status: open
- evidence: `cargo run -q --bin arpagona -- tool demo read-file ../Cargo.toml --json` returned `failed: invalid_path` with `is_security: false` instead of a security block. Daily validation 2026-05-27 also observed absolute-path reads such as `/etc/hosts` returning `failed: invalid_path` / `is_security: false`, while `.git/config` and `.env` correctly return `blocked` / `is_security: true`.
- expected behavior: parent traversal and absolute-path escape attempts are blocked with `status: blocked` and `is_security: true`, consistently with `.env` and `.git` blocks.
- suggested fix: normalize path escape rejection through the Tool Runtime security-error path.
- suggested tests: add `crates/tool-runtime` regression tests asserting parent traversal and absolute-path inputs return structured blocked/security results; keep existing `.env` and `.git` tests green.
- do not: broaden file access, permit parent traversal, or add new write/execute capability.

### DV-2026-05-26-003 — Context parser ambiguity should be explicit
- source: daily validation 2026-05-26
- category: CLI usability
- severity: low
- status: open
- evidence: `--context "priority:green,workstream:validation"` is treated as one key/value pair where the comma remains part of the value.
- expected behavior: either document that each `--context` invocation accepts one `key:value` pair, or explicitly support comma-separated context pairs.
- suggested fix: prefer documentation and tests first; only change parsing if a clear user-facing need is confirmed.
- suggested tests: add CLI parser tests for repeated `--context key:value` flags and for comma-containing values.
- do not: silently change parsing in a way that breaks values containing commas.

### DV-2026-05-26-004 — Compute Reservoir allocation may over-prefer `cloud-strong`
- source: daily validation 2026-05-26
- category: compute routing / cognitive usefulness
- severity: medium
- status: open
- evidence: daily validation observed allocation output choosing `cloud-strong` for the tested cognitive work loop scenario.
- expected behavior: low-sensitivity, low-complexity, local-first tasks should have a clear path to local or cheaper resources when policy allows.
- suggested fix: add targeted allocation tests covering public/low-complexity, private/high-sensitivity and complex/high-value objectives.
- suggested tests: assert allocation reasons explain why local, cheap or cloud-strong resources were selected.
- do not: call real models or cloud providers as part of the unit test; keep allocation deterministic unless a governed provider integration exists.

### DV-2026-05-27-001 — Open Holographic Memory PR needs regression coverage before merge
- source: daily validation 2026-05-27 code review
- category: code review / test coverage / governance UX
- severity: medium
- status: open
- evidence: open PR #103 adds CLI commands for `holographic-memory store`, `find-similar`, `find-related`, and `promote`, but the diff adds no CLI or integration regression tests for the new operator-facing surface. The implementation does include governance warnings in JSON output and appears read-only / local-only for the tested flows.
- expected behavior: new user-facing CLI commands have at least bounded regression tests covering JSON shape, non-authorizing warnings, and no accidental mutation for readback commands.
- suggested fix: before merge, add targeted tests for the Holographic Memory CLI commands and warning fields; keep tests local-only with temp in-memory or temp-file storage.
- suggested tests: JSON output includes `non_authorizing_warning` for readback commands; `promote --json` emits a non-authorizing review artifact, not approval; command errors remain structured.
- do not: merge the PR solely on manual validation; do not broaden approval, persistence, or external execution semantics.

### DV-2026-05-27-002 — Local LLM synthesis is useful but too generic for beta operator scoring
- source: daily validation 2026-05-27 Beta Usage Lab
- category: beta usability / LLM synthesis quality
- severity: low
- status: open
- evidence: local Ollama `qwen3.5:9b` was available and `cargo run -q --bin arpagona -- cognitive --llm local ... --json` completed locally without remote APIs, but the `llm_synthesis` text stayed generic and did not quote concrete structured fields or produce a compact self-scorecard for the operator.
- expected behavior: local synthesis should remain non-authorizing while making the readback easier to audit, preferably by referencing concrete fields from the working-memory output and summarizing missing context, risk, and next action in a predictable structure.
- suggested fix: tighten the prompt/template for local synthesis so it asks for grounded bullets tied to structured fields, without adding new capabilities or remote calls.
- suggested tests: local-model tests should remain opt-in/manual; deterministic unit tests can cover the prompt assembly and required warning text without invoking Ollama.
- do not: require a model download, call remote model APIs, or treat LLM prose as approval.

### DV-2026-05-27-003 — Daily validation conflict-marker scan produces protocol false positives
- source: daily validation 2026-05-27
- category: validation tooling / operator signal
- severity: low
- status: open
- evidence: the required protocol command `git grep -nE '<<<<<<<|=======|>>>>>>>' -- . ':!target'` matches its own example inside `docs/daily-agent-validation.md`, producing noise even when no real merge-conflict marker is present in source code.
- expected behavior: the validation scan distinguishes documentation examples from real conflict markers or documents the expected false positive explicitly.
- suggested fix: adjust the protocol command exclusion or make the example non-matching while preserving the intended check.
- suggested tests: run the conflict-marker scan and verify it is empty on a clean tree, or explicitly assert only the protocol example is ignored.
- do not: weaken conflict-marker detection for actual source, config, or docs accidentally containing real merge markers.

### DV-2026-05-27-004 — CLI docs coverage regressed for `executor`
- source: daily validation 2026-05-27 CLI/documentation check
- category: documentation / operator usability
- severity: medium
- status: open
- evidence: `bash scripts/check-cli-docs-coverage.sh` exited 1 with `Missing docs for CLI commands: executor` on current `main`; `arpagona --help` exposes `executor   List and inspect executor registry state`, but `docs/cli.md` has no matching top-level section.
- expected behavior: the docs coverage check remains green when new top-level CLI command groups are introduced, or the command is explicitly marked internal/experimental and excluded deliberately.
- suggested fix: document the `executor` command group in `docs/cli.md` or intentionally exclude it from the coverage check with rationale.
- suggested tests: rerun `bash scripts/check-cli-docs-coverage.sh` and verify exit 0.
- do not: remove the docs coverage check or weaken it broadly; keep CLI surface discoverable for operators.

## Closed / superseded candidates

- **DV-2026-05-26-002 — CLI documentation can drift behind CLI surface**
  - fixed: 2026-05-26 focus loop
  - summary: added `scripts/check-cli-docs-coverage.sh` (lightweight docs-coverage check validating all `arpagona --help` top-level commands have sections in `docs/cli.md`) and added the missing `auth` command documentation to `docs/cli.md`. The check script is now integrated as part of the docs-validation workflow; it can be run manually or wrapped into a CI step once the CLI surface stabilizes.
  - evidence: `bash scripts/check-cli-docs-coverage.sh` passes with exit 0; `arpagona auth --help` command group is now documented in `docs/cli.md` under "### Auth — Statut et configuration OpenAI".
