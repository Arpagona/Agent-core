# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement executor interface, disabled by default.

Why: the capability registry, policy engine, and dry-run layer now form a complete governance pipeline for action checking. What's missing is the formal executor abstraction — a trait/interface that consumes approved, policy-checked actions. Keep execution disabled at the trait level (`supports_real_execution: false` everywhere), but define the interface so the pipeline has a target.

Proof to seek: a trait `Executor` with methods like `supports(action_type, risk_level) -> bool` and `dry_run(action) -> DryRunResult` exists; no real executor implementation exists.

Do not: add real execution, modify files/tools/systems, call LLMs, or enable autonomous execution. The executor trait is purely abstract.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
