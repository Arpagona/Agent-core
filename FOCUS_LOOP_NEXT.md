# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Add `--observe` tool observation bridge to the `--assess` pipeline so that cognitive tool observations flow through assessment → FailureInsightCandidates → governed learning proposal path.

Why: P2 (cognitive run), P4 (working memory), P5 (compute allocation), P6 (holographic resonance) are all complete with CLI flags and parser tests. P3 is the next incomplete milestone: converting tool observations into governed learning proposals through the existing FailureInsightCandidate → ProposedAction → DecisionGate → Audit chain. The `--observe` flag exists but its results aren't piped into the assessment path.

Proof to seek: `cargo run -- cognitive run --objective "..." --domain research --assess --observe --json` produces `failure_insight_candidates` that include observation-derived entries (e.g. tool observation results mapped to candidate insights), not only improvement-candidate-derived entries. The `cognitive_observations` in working_memory show real tool output.

Do not: add LLM calls, persistence, shell execution, Decision Gate bypass, or HolographicMemory vector store. Keep it pure Rust + tool runtime.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
