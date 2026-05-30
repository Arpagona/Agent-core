# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-30 — orchestrator cycles CLI)

**main is green:** ✅ Full workspace tests pass.

**PR #TODO** (`feat/p3-orchestrator-cycles`) — **NEW**. Contains:
- New `orchestrator cycles` CLI command to list saved CycleTrace files from a directory
- `list_orchestrator_cycles_in_directory()` scans `.json` files, deserializes CycleTrace, returns structured listing metadata (cycle ID, objective, status, context sources, gate applied, non-authorizing, failure insight candidates, audit events, timestamp)
- Supports `--json` for structured output
- Supports `--trace-dir <dir>` for custom directory (default: `target/orchestrator-traces`)
- Handles empty/missing directories gracefully
- Includes safety readback disclaimer ("readback only — trace entries are evidence, not authorization")
- 4 new CLI parser tests (defaults, --json, --trace-dir, combined flags)
- Verification: `cargo fmt -- --check` ✅, `cargo check` ✅, `cargo test` ✅ (full workspace passes, no regressions)

**Stacked PRs still pending GONA merge:** #197, #198, #199, #200, #202, #203, #204, #205, #206, #207, #208

## Next action

1. **GONA: review and merge PR #201** (once CI is green).
2. **Review and merge this PR** (`feat/p3-orchestrator-cycles`).
3. **Then** — advance Phase 3: connect CycleTrace to the governed Audit system for persistent trace storage, or add `orchestrator run --save-trace` auto-naming to make cycles automatically findable.
