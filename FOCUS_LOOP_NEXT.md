# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — H2 CLI security boundary verification)

**main is green:** ✅ ~874 tests, 0 failures across full workspace.

**H2 progress — CLI security boundary verification:**
- Probed all documented CLI tool demo commands against the Tool Runtime security boundary:
  - `tool demo read-file`: absolute paths ✅ blocked, parent traversal ✅ blocked, `.env` ✅ blocked
  - `tool demo list-files`: `.git` ✅ blocked, `..` ✅ blocked, absolute paths ✅ blocked
  - `tool demo search-text`: `.git` ✅ blocked, parent traversal ✅ blocked
  - `tool demo observe`: crafted JSON `{"path":"../Cargo.toml"}` — still routed through runtime validate ✅ blocked
- **Discovered security gap:** `.git/config` was fully readable via `read-file`, leaking SSH identity path and remote URL
- **Fix:** Added `.git/` to `BLOCKED_FILE_PATTERNS` in `crates/tool-runtime/src/lib.rs`
- **5 new regression tests:**
  - `read_file_blocks_git_config` — `.git/config` blocked with `is_security: true`
  - `read_file_blocks_git_head` — `.git/HEAD` blocked
  - `read_file_blocks_relative_git_path` — `./.git/config` also blocked
  - `read_file_gitignore_still_readable` — `.gitignore` remains readable (negative test)
  - `read_file_github_dir_not_blocked` — `.github/workflows/*` not falsely blocked
- All 5 tests pass (874 total, +5 from previous run)

**DV backlog:** 0 open entries.

## Next action

**Phase 3 — Neutral Orchestrator V0 integration.** With H1a (clippy), H1b (edge-case tests), H2 (CLI security boundary) all delivered, the H hardening track is complete. The focus loop should now re-engage Phase 3: bounded Neutral Orchestrator integration — particularly the `--proposal-generator` integration tests and operator readback surfaces for orchestrator state.
