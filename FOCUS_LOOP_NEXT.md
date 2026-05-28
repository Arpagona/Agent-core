# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-28 focus loop — fix DV-2026-05-28-004)

PR #139 open (docs: fix DV-2026-05-28-002) — mergeable, all checks green. DEEP does not merge main; pending human merge.

PR #140 opened for DV-2026-05-28-004 (restore governance/readback regression assertions). All verification passes:
- `cargo fmt -- --check` ✅
- `cargo check` ✅
- `cargo test --workspace` (all tests pass) ✅
- `cargo test --test snapshot_integration` (9 tests pass) ✅

Remaining unresolved 2026-05-28 DV entries:
1. **DV-2026-05-28-003** — classify lexical `../` paths as security before filesystem lookup (low)
2. **DV-2026-05-28-005** — make local Ollama synthesis more specific to the operator request (low)
3. **DV-2026-05-28-001** — reduce false positives in the conflict-marker scan (low)

## Next action

**After PRs #139 and #140 are merged: fix DV-2026-05-28-003** (classify lexical `../` paths as security in Tool Runtime before filesystem canonicalization lookup, so missing parent-traversal targets return `is_security: true`).
