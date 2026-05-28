# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-28 focus loop — fix DV-2026-05-28-004)

PR #139 has been merged: `DV-2026-05-28-002` is fixed by documenting `mcp-governance-audit` and `llm`, with `bash scripts/check-cli-docs-coverage.sh` passing.

PR #140 is the active correction for `DV-2026-05-28-004` (restore governance/readback regression assertions). Verification reported by DEEP:
- `cargo fmt -- --check` ✅
- `cargo check` ✅
- `cargo test --workspace` ✅
- `cargo test --test snapshot_integration` ✅

Remaining unresolved 2026-05-28 DV entries after PR #140:
1. **DV-2026-05-28-003** — classify lexical `../` paths as security before filesystem lookup.
2. **DV-2026-05-28-005** — make local Ollama synthesis more specific to the operator request.
3. **DV-2026-05-28-001** — reduce false positives in the conflict-marker scan.

## Next action

**After PR #140 is merged: fix DV-2026-05-28-003** (classify lexical `../` paths as security in Tool Runtime before filesystem canonicalization lookup, so missing parent-traversal targets return `is_security: true`).
