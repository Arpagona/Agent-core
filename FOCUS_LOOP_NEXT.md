# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP 2026-05-30 — merge queue: #222 merged, #223 rebased)

**main is green:** Full workspace tests pass (947+ tests, 0 failures).
**Conflict markers:** None found.
**DAILY_VALIDATION_BACKLOG.md:** No open candidates.

Merged since last handoff:
- PR #221 — P3-27 audit/insight readback surface consolidation ✅
- PR #222 — C4 Compute Reservoir CLI documentation coverage ✅

## Next action (requires GONA merge)

Continue the merge queue in order after each PR is green and mergeable:
1. ~~#222 — C4 documentation coverage~~ ✅ **merged by GONA**
2. #223 — C5 anti-drift/adversarial tests — **rebased onto main, ready for GONA review/merge**
3. #224 — sandboxed write_file + patch_file (needs rebase — 6 commits ahead, 2 behind)
4. #225 — orchestrator run output improvement (needs rebase — 2 ahead, 2 behind)
5. #226 — P3-18+ efficiency metadata readback (needs rebase — 2 ahead, 2 behind)
6. #227 — sandboxed append_file + mkdir (needs rebase — 8 ahead, 2 behind)

After the queue is merged, continue the steroid-Hermes plan with the next safe sandboxed tool-runtime slice. Do **not** add unrestricted shell, browser, network, secrets access, or hidden autonomy.
