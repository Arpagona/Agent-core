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

### DV-2026-05-26-001 — Parent traversal should be reported as a security block
- source: daily validation 2026-05-26
- category: safety observability
- severity: low
- status: open
- evidence: `cargo run -q --bin arpagona -- tool demo read-file ../Cargo.toml --json` returned `failed: invalid_path` with `is_security: false` instead of a security block.
- expected behavior: parent traversal is blocked with `status: blocked` and `is_security: true`, consistently with `.env`, `.git` and absolute-path blocks.
- suggested fix: normalize parent traversal rejection through the Tool Runtime security-error path.
- suggested tests: add a `crates/tool-runtime` regression test asserting parent traversal returns a structured blocked/security result; keep existing `.env`, `.git` and absolute path tests green.
- do not: broaden file access, permit parent traversal, or add new write/execute capability.

### DV-2026-05-26-002 — CLI documentation can drift behind CLI surface
- source: daily validation 2026-05-26
- category: documentation / operator usability
- severity: medium
- status: open
- evidence: before PR #87, `docs/cli.md` missed 4 top-level command groups that were already exposed by `arpagona --help`.
- expected behavior: top-level CLI commands are either documented in `docs/cli.md` or explicitly marked as internal/experimental and intentionally undocumented.
- suggested fix: add a lightweight docs-coverage check or daily validation step comparing `arpagona --help` command groups with `docs/cli.md` headings.
- suggested tests: if feasible, add a script/test that fails when stable public commands are absent from CLI docs.
- do not: turn docs coverage into a brittle exact-output snapshot unless the CLI output is intended to be stable.

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

## Closed / superseded candidates

- **DV-2026-05-26-002 — CLI documentation can drift behind CLI surface**
  - fixed: 2026-05-26 focus loop
  - summary: added `scripts/check-cli-docs-coverage.sh` (lightweight docs-coverage check validating all `arpagona --help` top-level commands have sections in `docs/cli.md`) and added the missing `auth` command documentation to `docs/cli.md`. The check script is now integrated as part of the docs-validation workflow; it can be run manually or wrapped into a CI step once the CLI surface stabilizes.
  - evidence: `bash scripts/check-cli-docs-coverage.sh` passes with exit 0; `arpagona auth --help` command group is now documented in `docs/cli.md` under "### Auth — Statut et configuration OpenAI".
