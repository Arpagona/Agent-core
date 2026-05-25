# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Connect General Cognitive Work Loop V0 to CognitiveObservation inputs and FailureInsightCandidate promotion.

Why: General Cognitive Work Loop V0 (P2) is now implemented — it produces `RequiredObservation` and `ImprovementCandidate` objects. The Cognitive Observation Pipeline (P3 ready) flags candidates (truncated, empty, blocked) but stops at candidate detection. The next natural step is a governed cognitive loop that:
1. Feeds `RequiredObservation` back into the cognitive observation pipeline
2. Collects `ImprovementCandidate` items as candidates for `FailureInsight` creation
3. Creates a `FailureInsight` `ProposedAction` through the Decision Gate
4. Persists the approved `FailureInsight` via the governed Graph Memory path

Proof to seek: A new test showing that `ImprovementCandidate` can be mapped to a `FailureInsight` with `FailureClass::MissingContext` without side effects, and that `RequiredObservation` items appear in the `CognitiveObservation` pipeline.

Do not: start `scripts/demo-full-loop.sh` before P3 is implemented and merged. Do not add another snapshot/readback convenience command.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
