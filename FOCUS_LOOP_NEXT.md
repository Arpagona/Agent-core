# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — P3-16: Trace-to-Insight snapshot bridge)

**main is green:** ✅ Full workspace tests pass.

**PR #195 (P3-15):** Open, mergeable, CI green — awaiting GONA merge. Blocks P3-16 from reaching main.

**PR #196 (P3-16):** Created by this run — `feat/p3-16-trace-to-insight-snapshot-path`. Adds `--snapshot-path` to `orchestrator trace-to-insight`, writing extracted FailureInsight candidates as a FailureInsightDemoSnapshot for cross-invocation operator readback via `memory demo snapshot-read`. All checks clean locally.

**DAILY_VALIDATION_BACKLOG.md:** 0 open entries.

## Next actions (in order)

1. **GONA: merge PR #195** (P3-15 — Trace-to-Insight heuristic analysis, CI green, mergeable).
2. **Then: merge PR #196** (P3-16 — snapshot bridge, once CI confirms).
3. **Then: P3-17** — Integrate trace-to-insight into the orchestrator run command itself, so that `orchestrator run --trace --save-trace --trace-to-insight` automatically produces candidates without a separate command.
