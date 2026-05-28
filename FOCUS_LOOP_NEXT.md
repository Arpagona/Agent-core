# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-28)

Daily validation 2026-05-28 found five evidence-backed DV entries. PR #139 records the backlog/report handoff only; it deliberately did not fix runtime, CLI, Tool Runtime, tests, or LLM behavior. Green baseline checks do not mean the daily-validation backlog is clear.

## Next action

**Fix exactly one unresolved 2026-05-28 daily-validation entry, starting with `DV-2026-05-28-002` unless it is already fixed or blocked.**

Required pre-read before choosing work:

1. `DAILY_VALIDATION_BACKLOG.md`
2. latest available `daily-agent-core-validation` report/output for 2026-05-28
3. open daily-validation PRs, especially PR #139 if still open
4. the `Recommended Next Day Actions` section from the latest daily-validation report

Selection priority:

1. `DV-2026-05-28-002` — document missing CLI commands `mcp-governance-audit` and `llm`; make `bash scripts/check-cli-docs-coverage.sh` pass.
2. `DV-2026-05-28-004` — restore targeted governance/readback regression assertions.
3. `DV-2026-05-28-003` — classify lexical `../` paths as security before filesystem lookup.
4. `DV-2026-05-28-005` — make local Ollama synthesis more specific to the operator request.
5. `DV-2026-05-28-001` — reduce conflict-marker scan false positives.

Before editing, the focus-loop report must state:

- selected DV entry;
- why it is priority now;
- likely affected files;
- acceptance criteria;
- validation commands planned.

After editing, produce one dedicated technical PR and report:

- modified files summary;
- commands executed;
- `cargo fmt -- --check` result;
- `cargo check` result;
- `cargo test` result;
- if `DV-2026-05-28-002` was handled, `bash scripts/check-cli-docs-coverage.sh` result;
- remaining visible DV backlog entries for the next pass.

Do not start a non-DV milestone while any `DV-2026-05-28-*` entry remains open unless a strong blocker or safety/P0 rationale is written explicitly.
