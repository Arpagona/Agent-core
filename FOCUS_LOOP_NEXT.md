# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP 2026-05-30 — PRs #219/#218/#217 merged, P3 stack #211–#216 awaiting GONA)

**main is green:** ✅ Full workspace tests pass (925 tests, 0 failures, 17 ignored doc-tests).
**Conflict markers:** ✅ None found.
**DAILY_VALIDATION_BACKLOG.md:** ✅ No open candidates.

### Recently merged to main

| PR | Branch | Title | Status |
|----|--------|-------|--------|
| #219 | `docs/recover-governance-bootstrap` | docs: recover governance bootstrap (gona-deep-governance + steroid-hermes-action-plan) | ✅ Merged |
| #218 | `fix/context-grounding-llm-v2` | fix: include context item content in LLM synthesis wm_summary (v2) | ✅ Merged |
| #217 | `fix/cycle-trace-ambiguity-v2` | fix: distinguish zero-item from unavailable source states in detect_failure_candidates (v2) | ✅ Merged |

### P3 stack awaiting GONA merge (all OPEN, GREEN CI, mergeable)

| PR | Branch | Title |
|----|--------|-------|
| #216 | `feat/p3-26-cycles-with-audit` | P3-26: orchestrator cycles --with-audit for external audit event coverage |
| #215 | `feat/p3-25-orchestrator-save-audit` | P3-25: orchestrator run --save-audit for governed audit event persistence |
| #214 | `feat/p3-24-orchestrator-run-collect-insights` | P3-24: orchestrator run --collect-insights for automatic failure insight collection |
| #213 | `feat/p3-23-orchestrator-insights-collect` | P3-23: orchestrator insights CLI — collect and list failure insight candidates from CycleTrace |
| #212 | `feat/p3-22-cycle-trace-cost-quality` | P3-22: Add structured compute cost/quality metadata to CycleTrace |
| #211 | `feat/p3-audit-cycle-trace-bridge` | P3-21: Connect CycleTrace to governed Audit system |

### Stale open PRs (awaiting GONA evaluation)

| PR | Branch | Title | Note |
|----|--------|-------|------|
| #199 | `feat/p3-17-efficiency-context-assembly` | P3-17: Efficiency feedback context assembly | 10 commits behind main (P3-14 base), includes #198/#197 content |
| #198 | `feat/p3-16-compute-efficiency-feedback` | P3-16: Compute efficiency feedback | 10 commits behind main |
| #197 | `feat/p3-15-cycle-trace-to-failure-insight` | P3-15: CycleTrace to Failure-to-Insight (--insights flag) | 10 commits behind main |
| #204 | `feat/orchestrator-status-auto-trace` | orchestrator status UX — auto-save trace | 10 commits behind main |
| #202 | `feat/p3-15-cycletrace-cost-quality-meta` | P3-15: structured cost/quality metadata in CycleTrace | 10 commits behind main |

**Closed as superseded this run:** #200 (superseded by #219), #207 (superseded by #217), #208 (superseded by #218).

## Recommended merge order (by GONA)

1. **PR #211 → #212 → #213 → #214 → #215 → #216** (P3 stacked order)
2. **Then evaluate stale PRs #197, #198, #199, #202, #204** — they may need rebasing or may contain content now covered by the merged P3-21/22/23 stack

## Next action (after GONA merges the P3 stack)

DEEP can advance P3-27 on next cron run:
- `audit list-from-dir` — dedicated CLI readback surface for saved audit events
- Wire collected insights into Failure-to-Insight demo snapshot pipeline
- `orchestrator cycles --json` to include audit event type breakdown

## Current blocker

DEEP cannot proceed to P3-27 per governance charter §7: the P3 stack (#211–#216) is open and blocking the milestone. These are mergeable and green, awaiting GONA merge action.
