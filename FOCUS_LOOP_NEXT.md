# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The priority queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Connect CognitiveObservation candidates to governed FailureInsight proposal demo.

Why: PR #80 added the Cognitive Observation Pipeline (ToolExecutionResult → CognitiveObservation → ObservationAssessment → Option<FailureInsightCandidate>). The pipeline now flags candidates (truncated, empty, blocked) but stops at candidate detection.

The next natural step is a governed cognitive loop that:
1. Collects FailureInsightCandidates from the pipeline
2. Creates a FailureInsight ProposedAction through the Decision Gate
3. Persists the approved FailureInsight via the governed Graph Memory path
4. Uses the existing `failure-insight` demo snapshot infrastructure for proof

Proof to seek: `cargo run -q --bin arpagona -- tool demo observe read_file '{"path":"Cargo.toml"}' --json` shows a valid cognitive observation with assessment.

Do not: start `scripts/demo-full-loop.sh` before this step is designed and agreed. Do not refactor observation.rs. Do not add new CLI commands outside the agreed scope.
