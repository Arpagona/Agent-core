# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement execution capability registry.

Why: the dry-run sandbox now simulates approved proposals. The next step is a declarative registry mapping `ActionType` + `RiskLevel` to the actual executors that would handle them outside sandbox mode. This creates a formal contract between the Decision Gate, the human reviewer, and the future execution runtime.

Proof to seek: `arpagona capability list` shows available executors per action type and risk level, with a `sandbox_only` flag.

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. The registry is purely declarative.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
