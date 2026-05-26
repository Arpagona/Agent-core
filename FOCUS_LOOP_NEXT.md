# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement proposal deduplication and batching for `cognitive run --propose`.

Why: multiple FailureInsightCandidates often produce identical or nearly identical proposals (same tool, same action type, same target). This noise makes the ranked proposal list harder to review. Deduplication merges identical proposals into single batched entries with aggregate metadata (merged rationale, combined benefit, averaged confidence, max risk).

Proof to seek: `cognitive run --objective "..." --assess --observe --propose --json` produces fewer proposed_actions than failure_insight_candidates when duplicates exist, with a `batched` flag and `merged_count: N` in each proposal's payload.

Do not: modify any core types, add LLM calls, autonomous execution, or Decision Gate bypass. This is purely a CLI-level enrichment on the existing proposal bridge.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
