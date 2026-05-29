# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-md-29 — H1b edge-case tests)

**main is green:** ✅ ~868 tests, 0 failures across full workspace.

**PR #186** (`feat/h1b-edge-case-tests`) — open, awaiting CI.

**H1b progress:**
- Tool Runtime: 3 new symlink handling tests (internal symlink follows, outside symlink is security-blocked, directory symlink lists correctly)
- Decision Gate: 3 new edge-case tests (empty tool name, empty permissions, empty rationale — none panic)
- CLI: 4 new error-path parser tests (missing objective, empty objective, unknown provider, missing positional arg)
- **10 new tests total**, 0 regressions

**DV backlog:** 0 open entries.

## Next action

**H2: Missing security boundary at the CLI surface.** The Tool Runtime blocks absolute paths, parent traversal, `.env`, `.ssh` and system directories. But the CLI `tool demo read-file` command passes the path through — verify that the runtime's security boundaries are not bypassable through any documented CLI path (especially `--memory-value` JSON injection, `--json` pipe-to-file, or relative-path tricks that resolve differently than the runtime expects).
