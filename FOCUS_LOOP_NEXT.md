# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement dry-run execution sandbox for approved low-risk proposals.

Why: proposals now flow through the full lifecycle (cognitive pipeline → scoring → dedup → human review). The next step is a safe sandbox where approved low-risk proposals (risk_level = informational or low) can be executed in simulation mode — generating realistic output without side effects, proving the execution contract before enabling real execution.

Proof to seek: `arpagona action sandbox run <id>` executes the proposal in dry-run mode, returning structured output showing what would happen. `arpagona action sandbox list` shows pending sandbox runs with their status.

Do not: execute real side effects, add LLM calls, modify core types, or bypass Decision Gate. Sandbox mode must be explicitly opt-in and clearly marked as simulation.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
