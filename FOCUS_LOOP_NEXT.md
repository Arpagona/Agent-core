# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: Add `--propose` flag to `cognitive run` that converts `failure_insight_candidates` (from merged improvement-candidate and observation sources) into `ProposedAction` objects through the Decision Gate, proving the full governed learning proposal path in one invocation.

Why: P3 milestone now bridges observations through assessment into FailureInsightCandidates. The remaining gap is converting those candidates into governed proposals (ProposedAction → DecisionGate → Decision → Audit). A `--propose` flag would complete the chain.

Proof to seek: `cargo run -- cognitive run --objective "..." --domain research --assess --observe --propose --json` produces output containing `proposed_actions`, `decisions`, and `audit_events` alongside existing `failure_insight_candidates`. The proposed actions are non-authorizing and gated by the Decision Gate.

Do not: add LLM calls, persistence, shell execution, Decision Gate bypass, new tool execution, HolographicMemory vector store, or any autonomy/self-modification. Keep it pure Rust + existing Decision Gate + existing Audit types.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
