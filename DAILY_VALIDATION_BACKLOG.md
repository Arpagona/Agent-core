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

### DV-2026-06-01-001 — Ollama cognitive synthesis stays structured but often fails to answer context-backed beta prompts
- source: daily validation 2026-06-01 Beta Usage Lab
- category: beta usability / LLM synthesis quality
- severity: medium
- status: open; recorded in PR #252
- evidence: 8 local-only `cognitive run --llm --provider ollama --assess --allocate --json` scenarios against installed `qwen3.5:9b` all returned the required `[STATE]`, `[KEY GAP / RISK]`, and `[RECOMMENDED NEXT STEP]` sections, but several did not answer the concrete operator request even when the prompt carried enough information. Examples: repository orientation returned `RequestContext` instead of summarizing available local context; code-review asked for missing context instead of naming an argument-schema regression test; compute routing asked for local hardware details instead of recommending local/low-cost routing for a low-risk documentation task; Failure-to-Insight asked for more context instead of forming a non-authorizing candidate from the supplied synthetic bug.
- expected behavior: model-backed cognitive synthesis should remain non-authorizing and structured while using supplied objective text and safe local context to produce scenario-specific operator help; it should ask for context only when genuinely necessary.
- suggested fix/tests: add focused synthesis prompt or context-packing regression tests for the eight Beta Usage Lab scenario classes, especially orientation, code review, compute routing and Failure-to-Insight. Acceptance should require a concrete answer plus governance warning, not only the three-section scaffold.
- do not: call remote model APIs, require model downloads, read secrets, grant shell access, bypass Decision Gate, or let LLM prose authorize execution.

## Closed / superseded candidates

### DV-2026-05-31-001 — Workspace `cargo test` failed because snapshot integration could not start `arpagona-api-server`
- source: daily validation 2026-05-31 baseline health
- category: test reliability / integration harness
- severity: high
- status: **fixed in PR #238**
- evidence: `cargo test` failed in `crates/cli/tests/snapshot_integration.rs::cognitive_propose_pipeline_produces_governed_proposals` with `failed to start API server: Os { code: 2, kind: NotFound, message: "No such file or directory" }` at line 461. The test computed `target/debug/arpagona-api-server`, but no such binary was present after the workspace test build in that local run.
- fix: the snapshot integration harness now resolves the expected API server binary path and explicitly builds `arpagona-api-server` as a preflight if the binary is missing.
- verification: PR #238 local verification reported the focused snapshot integration test passing, `cargo fmt -- --check` passing, `cargo check` passing, and full workspace `cargo test` passing. GitHub CI was green before merge.
- follow-up: `arpagona doctor` was added in PR #239 to catch binary/readiness issues earlier in the validation flow.
- do not: weaken governance assertions, ignore the test, require secrets, call remote providers, or add broad process/scheduler capability while repairing harnesses.

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
