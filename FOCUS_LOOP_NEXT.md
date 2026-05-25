# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Integrate CognitiveObservation and FailureInsightCandidates into WorkingMemory cycle state.

Why: P3 bridge is now merged (PR #83) — `FailureInsightCandidate::from_improvement_candidates()` and `--assess` CLI flag are live on main. The next step is to feed `CognitiveObservation` pipeline outputs and resolved `FailureInsightCandidate` items back into `WorkingMemory` so subsequent cognitive cycles benefit from accumulated state.

Proof to seek: A test showing that a `CognitiveObservation` assessment (e.g. `UsefulAndComplete`) can be stored as a `ContextItem` in `WorkingMemory`, and that a `FailureInsightCandidate` can produce a `Constraint` or `Assumption` update in the next cycle.

Do not: add LLM calls, persistence, shell execution, external effects, or Decision Gate bypass. P4 must remain pure-domain state transformation.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
