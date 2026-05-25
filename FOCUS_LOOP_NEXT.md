# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add an end-to-end integration test proving that `--description` text flows through the full governed path (signal → proposal → decision → audit → persistence → readback) and appears in the readback output.

Why: the `--description` flag was added this session, but there is no test verifying that operator-supplied text actually propagates through the full FailureInsight → ProposedAction → Decision Gate → Audit → persistence → readback chain. The parser test only verifies CLI argument parsing.

Proof to seek: a new test in the existing `#[cfg(test)] mod tests {}` block in `main.rs` that calls `memory_demo_failure_insight_readback(Some("inspect-id"), Some("custom description"))` and asserts the readback JSON/text contains the custom description in the signal summary, failure summary, and relevant fields.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, MCP/browser automation, personal/sensitive memory, readback-as-authorization behavior, or always-on native/unstable backend requirements.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
