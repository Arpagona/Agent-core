# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: pick P2 (General Cognitive Work Loop V0) from the milestone queue — specifically add parser tests and documentation for the existing `cognitive run` CLI flags (--assess, --observe, --allocate, --resonate, --propose) and fill any gaps before starting deeper runtime work.
Why: offline executor commands now have full end-to-end integration coverage and PR #99 is merged. P2 is the next unstarted milestone. Before adding new cognitive run behavior, existing flags should have parser parity with the rest of the CLI.
Proof to seek: all `cognitive run` flags (--assess, --observe, --propose, --allocate, --resonate) have parser tests matching the pattern in existing executor tests.
Do not: add real execution, modify cognitive runtime behavior, bypass Decision Gate, or add new flags in this session.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
