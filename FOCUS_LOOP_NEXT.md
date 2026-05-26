# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Implement proposal scoring and prioritization for `cognitive run --propose`.

Why: the `--propose` bridge now produces context-rich ProposedActions with `expected_benefit`, `risk_level`, `suggested_action_type`, and `confidence` metadata. The next step is to rank proposals so the user sees the most impactful ones first — by expected_benefit × confidence, with risk_level as tiebreaker.

Proof to seek: `cognitive run --objective "..." --assess --observe --propose --json` produces `proposed_actions` sorted by priority score, with a `priority_scored: true` flag and `priority_rationale` explaining the ranking.

Do not: modify any core types, add LLM calls, autonomous execution, or Decision Gate bypass. This is purely a CLI-level sorting + output enrichment.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
