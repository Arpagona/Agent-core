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

### DV-2026-05-28-001 — Conflict-marker scan still produces false positives outside the protocol/backlog files
- source: daily validation 2026-05-28 repository sync
- category: validation tooling / operator signal
- severity: low
- status: open
- evidence: the mandatory scan `grep -R "<<<<<<<\|=======\|>>>>>>>" --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude=daily-agent-validation.md --exclude=DAILY_VALIDATION_BACKLOG.md .` returned matches in `PROJECT_STATUS.md` because that document embeds the grep pattern and previous run evidence, not unresolved merge markers.
- expected behavior: daily validation should flag real unresolved conflict markers while avoiding self-referential documentation examples in status/protocol artifacts.
- suggested fix/tests: update the protocol scan to use line-anchored marker detection or explicitly exclude generated/status handoff files.
- do not: weaken detection for real source/config/document conflicts or ignore grep failures broadly.

### DV-2026-05-28-005 — Local Ollama synthesis remains structured but often generic against operator prompts
- source: daily validation 2026-05-28 Beta Usage Lab
- category: beta usability / LLM synthesis quality
- severity: low
- status: open
- evidence: eight local-only `cognitive run --llm --provider ollama --assess --allocate --json` scenarios produced safe structured responses but repetitive generic framing.
- expected behavior: local synthesis should retain the safe structured format while grounding each answer in the specific request.
- suggested fix/tests: add deterministic mock-provider acceptance tests or prompt assembly tests requiring request-specific fields and one concrete bounded next step without claiming authorization.
- do not: call remote model APIs, require model downloads, or allow prose to bypass Decision Gate governance.

## Closed / superseded candidates

### DV-2026-05-28-002 — CLI documentation coverage is missing `mcp-governance-audit` and `llm`
- status: **fixed in PR #139** (2026-05-28 focus loop)
- evidence: `bash scripts/check-cli-docs-coverage.sh` exits 0 after adding both commands to `docs/cli.md` and their patterns to the coverage script.

### DV-2026-05-28-003 — Missing parent-traversal target is reported as non-security `invalid_path`
- status: **fixed in PR #141** (2026-05-28 focus loop)
- evidence: `resolve_path()` in `crates/tool-runtime/src/lib.rs` now performs lexical `..` escape detection before calling `canonicalize()`. Missing parent-traversal targets that would escape the workspace now return `Blocked`/`is_security: true`.
- verification: parent-traversal tests cover missing targets for read_file, list_files, and search_text.

### DV-2026-05-28-004 — Recent snapshot integration simplification removed useful governance regression assertions
- status: **fixed in PR #140** (2026-05-28 focus loop)
- evidence: targeted governance/readback regression assertions were restored in `crates/cli/tests/snapshot_integration.rs`.

### DV-2026-05-26-001 — Path escape attempts should be reported as security blocks
- status: **fixed in PR #119** (2026-05-27 focus loop)

### DV-2026-05-26-002 — CLI documentation can drift behind CLI surface
- status: fixed 2026-05-26 focus loop

### DV-2026-05-27-001 — Open Holographic Memory PR needs regression coverage before merge
- status: **closed — superseded** (PR #103 was merged)

### DV-2026-05-27-002 — Local LLM synthesis is useful but too generic for beta operator scoring
- status: **fixed in PR #124** (2026-05-27 focus loop)

### DV-2026-05-27-003 — Daily validation conflict-marker scan produces protocol false positives
- status: fixed in 2026-05-27 focus loop

### DV-2026-05-27-004 — CLI docs coverage regressed for `executor`
- status: **fixed in PR #108** (2026-05-27 focus loop)
