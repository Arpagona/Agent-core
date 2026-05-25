# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The priority queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: wait for CI on PR #77 to complete (mergeStateStatus: UNSTABLE), verify green, then merge into main.
Why: #77 has been rebased, conflicts resolved, superseded PRs (#74, #72) closed. The description-propagation chain is ready for merge.

Proof to seek: `gh pr view 77 --json mergeStateStatus,state,statusCheckRollup` shows MERGEABLE + green CI checks. Then `gh pr merge 77 --squash` succeeds.

Do not: create new work until #77 is merged. After merge, create `scripts/demo-full-loop.sh` as the next feature increment (self-contained repeatable governed FailureInsight demo).
