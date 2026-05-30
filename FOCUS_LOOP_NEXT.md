# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (GONA 2026-05-30 — merge queue in progress)

GONA has started merging the previously green PR queue.

Completed in this merge sequence:
- PR #221 — P3-27 audit/insight readback surface consolidation
- PR #222 — C4 Compute Reservoir CLI documentation coverage branch rebased after #221; ready to merge once CI is green again

## Next action

Continue the merge queue in order after each PR is green and mergeable:
1. #222 — C4 documentation coverage
2. #223 — C5 anti-drift/adversarial tests
3. #224 — sandboxed write_file + patch_file
4. #225 — orchestrator run output improvement
5. #226 — P3-18+ efficiency metadata readback
6. #227 — sandboxed append_file + mkdir

After the queue is merged, continue the steroid-Hermes plan with the next safe sandboxed tool-runtime slice. Do **not** add unrestricted shell, browser, network, secrets access, or hidden autonomy.
