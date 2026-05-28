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

### DV-2026-05-28-005 — Make local Ollama synthesis more specific to the operator request
- source: daily validation 2026-05-28
- category: beta usability / LLM synthesis quality
- severity: low
- status: open
- evidence: `cognitive run --llm --provider ollama` produces structured [STATE]/[KEY GAP/RISK]/[RECOMMENDED NEXT STEP] sections since PR #124 (fixed DV-2026-05-27-002), but the content is generic and does not reference specific fields from the operator's objective/context. The mock provider references concrete fields; the real Ollama provider's output depends on the model's instruction-following ability. The gap is that `--provider ollama` (the default) may produce less-specific output than `--provider mock`.
- suggested fix: verify that the Ollama model (qwen3.5:9b) can follow the structured prompt. This is a model-quality issue, not a code issue. If unacceptable, document in `docs/cli.md` that `--provider mock` gives deterministic structured output while `--provider ollama` quality depends on local model capability.
- do not: weaken the safety prompt (no tool calls, no authorization claims) to improve specificity.

## Closed / superseded candidates

- **DV-2026-05-26-001 — Path escape attempts should be reported as security blocks**
  - fixed: PR #119 (2026-05-27 focus loop)
  - summary: All three tools (read_file, list_files, search_text) consistently return blocked/security for absolute paths and parent traversal. 3 new regression tests added.

- **DV-2026-05-26-002 — CLI documentation can drift behind CLI surface**
  - fixed: 2026-05-26 focus loop
  - summary: added `scripts/check-cli-docs-coverage.sh` and added missing `auth` command documentation to `docs/cli.md`.

- **DV-2026-05-26-003 — Context parser ambiguity should be explicit**
  - fixed: 2026-05-28 focus loop (PR #? — pending human merge)
  - summary: 7 CLI parser tests added covering key:value, comma in value, spaces, empty, multi-key, multi-line, and repeated-flag rejection. No parsing changes made.

- **DV-2026-05-26-004 — Compute Reservoir allocation justification coverage**
  - fixed: 2026-05-28 focus loop (PR pending merge)
  - summary: 4 new targeted allocation justification tests added to `crates/compute-reservoir`.

- **DV-2026-05-27-001 — Open Holographic Memory PR needs regression coverage before merge**
  - closed: superseded (PR #103 merged on 2026-05-27)

- **DV-2026-05-27-002 — Local LLM synthesis is useful but too generic for beta operator scoring**
  - fixed: PR #124 (2026-05-27 focus loop)
  - summary: `COGNITIVE_SYNTHESIS_SYSTEM_PROMPT` now requests structured sections; MockProvider produces deterministic structured output.

- **DV-2026-05-27-003 — Daily validation conflict-marker scan produces protocol false positives**
  - fixed: 2026-05-28 focus loop (PR pending merge)
  - summary: Added `--exclude=daily-agent-validation.md` and `--exclude=DAILY_VALIDATION_BACKLOG.md` to the grep command. No false positives from protocol/tracking documents.

- **DV-2026-05-27-004 — CLI docs coverage regressed for `executor`**
  - fixed: PR #108 (2026-05-27 focus loop)
  - summary: Added `executor` command group and `mcp-server` command documentation to `docs/cli.md`; added pattern entries to `scripts/check-cli-docs-coverage.sh`.

- **DV-2026-05-28-001 — Reduce false positives in the conflict-marker scan (PROJECT_STATUS.md)**
  - fixed: 2026-05-28 focus loop (this run)
  - summary: Added `--exclude=PROJECT_STATUS.md` to the conflict-marker grep command in `docs/daily-agent-validation.md`. PROJECT_STATUS.md contains self-referential grep command examples that trigger false positives. Verified: grep returns zero matches (exit 1 = clean).

- **DV-2026-05-28-003 — Lexical parent traversal should be classified as security before filesystem lookup**
  - fixed: PR #141 (2026-05-28 focus loop) — waiting for human merge
  - summary: lexical `..` escape detection added before `canonicalize()` in Tool Runtime. 4 new tests.

- **DV-2026-05-28-004 — Governance/readback regression assertions**
  - fixed: PR #140 (2026-05-28 focus loop) — waiting for human merge
  - summary: restored governance/readback regression assertions.
