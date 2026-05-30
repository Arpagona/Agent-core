# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP 2026-05-30 — sandboxed sync_file/fsync tool added to tool-runtime)

This session added `sync_file` (alias: `fsync`) to the sandboxed Tool Runtime.

The sandboxed tool set now has 8 tools (7 existing + 1 new):
- **`sync_file`** / `fsync`: workspace-bounded fsync, simulation-first, security-blocked for absolute paths/parent traversal, max 1 MiB. Also syncs parent directory for directory entry flush.

Changed:
- `crates/tool-runtime/src/lib.rs` — Added `sync_file`/`fsync` dispatch entry, `execute_sync_file()` function, and 11 tests (simulate, execute, absolute path block, parent traversal, nonexistent, directory, fsync alias, blocked file pattern, missing argument, workspace file)
- `crates/cli/src/main.rs` — Added `SyncFile` enum variant, `ToolDemoSyncFileArgs` struct, dispatch line, tool-list entry, inspect entry, demo handler
- All verification passes: `cargo fmt -- --check`, `cargo check`, `cargo test --workspace` (987+ tests, 0 failures)
- PR #230 force-pushed with the new commit

## Next action

Advance the steroid-Hermes plan with one of:
1. E5 product positioning evidence — turn technical progress into reusable marketing proof (3-5 claims with implementation evidence).
2. E1 SME documentary assistant demo — create a product-facing scenario using the existing read-only tools.
3. C5 anti-drift and adversarial tests — add safety tests for the existing LLM integration (hallucination containment, tool bypass attempts, prompt injection).

Do **not** add unrestricted shell, browser, network, secrets access, or hidden autonomy.
