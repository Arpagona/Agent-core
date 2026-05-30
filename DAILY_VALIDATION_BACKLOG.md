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

### DV-2026-05-30-001 — Local Ollama cognitive beta path is structured but under-grounded for repository/operator tasks
- source: daily validation 2026-05-30 Beta Usage Lab
- category: beta usability / LLM synthesis quality
- severity: medium
- status: open
- evidence: `qwen3.5:9b` was installed and eight `cargo run -q --bin arpagona -- cognitive run --llm --provider ollama --assess --allocate --json --objective ...` scenarios completed through Agent Core. Responses consistently preserved `[STATE]`, `[KEY GAP / RISK]`, and `[RECOMMENDED NEXT STEP]`, but most reported `0 context items` and `1 missing context item`; orientation, planning, code-review and Failure-to-Insight prompts therefore returned generic missing-context guidance rather than scenario-specific operator help.
- expected behavior: local-model beta scenarios should either consume explicit bounded context supplied by the operator/runtime or make the missing-context requirement precise enough to be actionable, while still refusing secrets and unrestricted shell.
- suggested fix/tests: add a bounded regression/demo path that injects small safe context via `--context` or a documented local read-only context bridge, then assert the synthesis cites concrete repository/tool evidence for orientation/planning/code-review prompts.
- do not: call remote model APIs, pull models, read `.env`/secrets, or treat model prose as authorization.

### DV-2026-05-30-002 — CycleTrace failure-candidate detection lacked a regression for zero-item partial-source ambiguity
- source: daily validation 2026-05-30 code review of latest commit `3cae1c6`
- category: regression test / observation accuracy
- severity: low
- status: **fixed in this PR**
- evidence: `CycleTrace::detect_failure_candidates()` emitted `All N context source(s) unavailable` whenever `total_context_items == 0 && !unavailable_sources.is_empty()`, even when some sources were available but returned zero items.
- fix: added per-source availability check via `context_source_summaries` to distinguish "all sources truly unavailable" from "mixed: some available (returned zero), others unavailable." Added targeted unit test `detect_candidates_with_zero_items_and_mixed_source_availability` (c8) that proves the summary does NOT claim "all unavailable" when some sources are actually available.
- verification: `cargo fmt -- --check` ✅, `cargo check` ✅, `cargo test` — 921+ tests pass including new test.
- do not: add new Failure-to-Insight mutation paths, bypass Decision Gate, or broaden runtime capabilities.

## Closed / superseded candidates

### DV-2026-05-29-001 — CLI documentation coverage missing `orchestrator`
- source: daily validation 2026-05-29 CLI documentation check
- category: documentation / operator usability
- severity: low
- status: **fixed in PR #178**
- evidence: `bash scripts/check-cli-docs-coverage.sh` exited 1 with `Missing docs for CLI commands: orchestrator` while `arpagona --help` listed the top-level `orchestrator` command.
- fix: documented `orchestrator run` in `docs/cli.md` and added the `orchestrator` coverage pattern to `scripts/check-cli-docs-coverage.sh`.
- verification: `bash scripts/check-cli-docs-coverage.sh` passes after the fix; full baseline remains required before PR.
- do not: add scheduler/autonomy, treat orchestrator output as authorization, or expand execution capabilities.

### DV-2026-05-28-005 — Local Ollama synthesis remains structured but often generic against operator prompts
- source: daily validation 2026-05-28 Beta Usage Lab
- category: beta usability / LLM synthesis quality
- severity: low
- status: **fixed in PR #147**
- evidence: fixed `MockProvider::synthesize()` to use actual parsed context_items and assumptions counts instead of `'?'` placeholders. Extended `parse_wm_summary_fields()` to return all 7 working-memory fields. Improved `COGNITIVE_SYNTHESIS_SYSTEM_PROMPT` to explicitly require citing objective text, domain name, and all concrete counts. Added 2 acceptance tests (`mock_synthesis_references_context_items_and_assumptions`, `mock_synthesis_with_zero_context_still_self_contained`) proving mock output references concrete field values and never contains `'?'` placeholders.
- verification: `cargo test -p arpagona-llm` — 38 tests pass including both new acceptance tests. Full workspace: 536+ tests pass.
- do not: call remote model APIs, require model downloads, or allow prose to bypass Decision Gate governance.

### DV-2026-05-28-001 — Conflict-marker scan still produces false positives outside the protocol/backlog files
- status: **fixed in PR #143** (2026-05-28 focus loop)
- evidence: `docs/daily-agent-validation.md` excludes `PROJECT_STATUS.md`, the remaining self-referential false-positive source. Verification reported zero conflict-marker matches (grep exit 1 = clean).

### DV-2026-05-28-002 — CLI documentation coverage is missing `mcp-governance-audit` and `llm`
- status: **fixed in PR #139** (2026-05-28 focus loop)
- evidence: `bash scripts/check-cli-docs-coverage.sh` exits 0 after adding both commands to `docs/cli.md` and their patterns to the coverage script.

### DV-2026-05-28-003 — Missing parent-traversal target is reported as non-security `invalid_path`
- status: **fixed in PR #141** (2026-05-28 focus loop)
- evidence: lexical `..` escape detection runs before `canonicalize()` in `crates/tool-runtime/src/lib.rs`; missing parent-traversal targets now return `Blocked`/`is_security: true`.

### DV-2026-05-28-004 — Recent snapshot integration simplification removed useful governance regression assertions
- status: **fixed in PR #140** (2026-05-28 focus loop)
- evidence: targeted governance/readback regression assertions were restored in `crates/cli/tests/snapshot_integration.rs`.

### Older closed / superseded items
- DV-2026-05-29-002 — Safety refusal for forbidden secret/shell objectives in LLM synthesis — **fixed in PR #179**
- DV-2026-05-26-001 — fixed in PR #119.
- DV-2026-05-26-002 — fixed 2026-05-26.
- DV-2026-05-27-001 — closed, superseded by PR #103 merge.
- DV-2026-05-27-002 — fixed in PR #124.
- DV-2026-05-27-003 — fixed in 2026-05-27 focus loop.
- DV-2026-05-27-004 — fixed in PR #108.
