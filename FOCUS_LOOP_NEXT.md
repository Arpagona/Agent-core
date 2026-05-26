# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: Add `--context` flag support to the `--propose` pipeline so that proposed actions include operator-provided context (from `cognitive run --context "..."`), and extend the `run_proposal()` function to include `context_refs` from WorkingMemory. Then add a human-readable and end-to-end integration test proving the full `--assess --observe --propose --json` pipeline produces correct outputs.

Why: P3 milestone now has the full governed learning proposal path through the Decision Gate and Audit (proposed_actions + decisions + audit_events in one JSON output). The next gap is context-aware proposals and end-to-end automated verification to prevent regression.

Proof to seek: `cargo test --workspace` shows 254+ tests passing (3 new parser tests + 1 end-to-end), including a new integration test that spawns `arpagona cognitive run --assess --observe --propose --json` and validates the JSON output contains `proposed_actions`, `decisions`, and `audit_events` with expected shapes.

Do not: add LLM calls, persistence, shell execution, Decision Gate bypass, new tool execution, HolographicMemory vector store, or any autonomy/self-modification.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
