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

### DV-2026-05-28-003 — Lexical parent traversal should be classified as security before filesystem lookup
- source: FOCUS_LOOP_NEXT.md handoff + daily validation 2026-05-28 code audit
- category: safety observability
- severity: medium
- status: **fixed in PR #141** (2026-05-28 focus loop)
- evidence: `resolve_path()` in `crates/tool-runtime/src/lib.rs` now performs lexical `..` escape detection before calling `canonicalize()`. Missing parent-traversal targets that would escape the workspace (e.g. `../nonexistent.txt`) now return `Blocked`/`is_security: true` instead of `Failed`/`is_security: false`. 4 new tests added: `nonexistent_parent_traversal_is_security_blocked`, `deep_parent_traversal_is_security_blocked`, `list_files_nonexistent_parent_traversal_is_security_blocked`, `search_text_nonexistent_parent_traversal_is_security_blocked`. Existing `read_file_blocks_path_escaping_workspace` updated to expect `Blocked`/`is_security: true`.
- verification: `cargo test -p arpagona-tool-runtime` — 28 tests pass including 4 new + 1 updated parent-traversal tests.
- do not: weaken absolute-path blocking or broad `..` detection to allow workspace escape attempts.

### DV-2026-05-26-001 — Path escape attempts should be reported as security blocks
- source: daily validation 2026-05-26
- category: safety observability
- severity: low
- status: **fixed in PR #119** (2026-05-27 focus loop)
- evidence: `cargo run -q --bin arpagona -- tool demo read-file /etc/passwd --json` now returns `"status": "blocked"` with `"error": {"is_security": true}`. All three tools (read_file, list_files, search_text) consistently return blocked/security for absolute paths and parent traversal. 3 new regression tests added: `absolute_path_parent_traversal_is_security_blocked`, `list_files_blocks_absolute_paths`, `list_files_blocks_parent_traversal`.

### DV-2026-05-26-003 — Context parser ambiguity should be explicit
- source: daily validation 2026-05-26
- category: CLI usability
- severity: low
- status: **fixed in this session** (PR pending merge)
- evidence: 7 CLI parser tests added covering: basic key:value, comma in value, spaces in value, empty context, multi-key with newlines, multi-line key:value (pre-existing), and repeated-flag rejection. All tests pass at `cargo test --bin arpagona -- context`. File: `crates/cli/src/main.rs` `#[cfg(test)] mod tests`.
- suggested fix: done — see tests above; no parsing changes made (comma stays part of value, single-flag usage documented by test)
- do not: silently change parsing in a way that breaks values containing commas.

### DV-2026-05-26-004 — Compute Reservoir allocation justification coverage
- source: daily validation 2026-05-26
- category: compute routing / cognitive usefulness
- severity: medium
- status: **fixed in this session** (PR pending merge)
- evidence: 4 new targeted allocation justification tests added to `crates/compute-reservoir/src/lib.rs`:
  1. `p4_public_low_complexity_prefers_cheap_local_with_justification` — public, low-complexity, local_first → local-small with justification mentioning cost/locality
  2. `p4_high_sensitivity_justifies_local_resource_by_sensitivity` — confidential data → local-small with justification mentioning sensitivity
  3. `p4_complex_high_value_justifies_strong_model_by_capability` — complex, high-value → cloud-strong with justification mentioning capability
  4. `p4_justification_explains_fallback_when_ideal_missing` — capability gap → FallbackSelected with explanation in fallback.reason
- all 19 compute-reservoir tests pass; full workspace 530+ tests pass.

### DV-2026-05-27-001 — Open Holographic Memory PR needs regression coverage before merge
- source: daily validation 2026-05-27 code review
- category: code review / test coverage / governance UX
- severity: medium
- status: **closed — superseded** (PR #103 was merged since this was written)
- evidence: PR #103 (holographic-memory-cli) was merged on 2026-05-27. The CLI commands for holographic-memory are now in main. Follow-up coverage can be added as a separate PR if needed.

### DV-2026-05-27-002 — Local LLM synthesis is useful but too generic for beta operator scoring
- source: daily validation 2026-05-27 Beta Usage Lab
- category: beta usability / LLM synthesis quality
- severity: low
- status: **fixed in PR #124** (2026-05-27 focus loop)
- evidence: `COGNITIVE_SYNTHESIS_SYSTEM_PROMPT` now requests a structured self-scorecard with [STATE], [KEY GAP / RISK], [RECOMMENDED NEXT STEP] sections and explicitly asks to reference concrete field values. `MockProvider::synthesize()` parses WM summary fields and produces deterministic structured output. 7 new deterministic unit tests cover prompt assembly, prompt safety, field extraction, and structured output format. PR #124 merged into main.
- verification: `cargo test -p arpagona-llm` — 21 tests pass including `cognitive_synthesis_prompt_contains_structured_sections`, `cognitive_synthesis_prompt_retains_safety_warnings`, `mock_synthesis_output_contains_structured_sections`, and `mock_synthesis_output_references_concrete_fields`.
- do not: require a model download, call remote model APIs, or treat LLM prose as approval.

### DV-2026-05-27-003 — Daily validation conflict-marker scan produces protocol false positives
- source: daily validation 2026-05-27
- category: validation tooling / operator signal
- severity: low
- status: **fixed in this session** (PR pending merge)
- evidence: added `--exclude=daily-agent-validation.md` and `--exclude=DAILY_VALIDATION_BACKLOG.md` to the grep command in `docs/daily-agent-validation.md` (lines 128-129). Both files are protocol/tracking documents whose content unavoidably contains the scan patterns as self-referential examples — not real conflict markers in source code. The `--exclude` flag uses basename matching, so the full paths are not needed.
- verification: the conflict-marker scan with both exclusions now returns zero matches (grep exit code 1 = clean). No false positives from protocol/tracking documents.
- do not: weaken conflict-marker detection for actual source, config, or docs accidentally containing real merge markers. The `--exclude` only removes the protocol doc itself from the scan.

|### DV-2026-05-27-004 — CLI docs coverage regressed for `executor`
|- source: daily validation 2026-05-27 CLI/documentation check
|- category: documentation / operator usability
|- severity: medium
|- status: **fixed in PR #108** (2026-05-27 focus loop)
|- evidence: `bash scripts/check-cli-docs-coverage.sh` now exits 0 — both `executor` and `mcp-server` documentation have been added to `docs/cli.md` along with their pattern mappings in the check script.
|- expected behavior: the docs coverage check remains green when new top-level CLI command groups are introduced, or the command is explicitly marked internal/experimental and excluded deliberately.
|- suggested fix: documented the `executor` command group and `mcp-server` command in `docs/cli.md`; added pattern entries to `scripts/check-cli-docs-coverage.sh`.
|- suggested tests: rerun `bash scripts/check-cli-docs-coverage.sh` and verify exit 0.
|- do not: remove the docs coverage check or weaken it broadly; keep CLI surface discoverable for operators.

## Closed / superseded candidates

- **DV-2026-05-26-002 — CLI documentation can drift behind CLI surface**
  - fixed: 2026-05-26 focus loop
  - summary: added `scripts/check-cli-docs-coverage.sh` (lightweight docs-coverage check validating all `arpagona --help` top-level commands have sections in `docs/cli.md`) and added the missing `auth` command documentation to `docs/cli.md`. The check script is now integrated as part of the docs-validation workflow; it can be run manually or wrapped into a CI step once the CLI surface stabilizes.
  - evidence: `bash scripts/check-cli-docs-coverage.sh` passes with exit 0; `arpagona auth --help` command group is now documented in `docs/cli.md` under "### Auth — Statut et configuration OpenAI".
