# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add an end-to-end async test proving the `governed-loop` command's `--description` text flows through the full chain and appears in the readback output.

Why: the `--description` flag is supported by the `governed-loop` command and flows through `memory_demo_failure_insight_readback`, but there is no test verifying that operator-supplied text propagates through the full FailureInsight → ProposedAction → Decision Gate → Audit → persistence → readback chain when invoked via the governed-loop CLI path.

Proof to seek: a new async test in `main.rs` that calls `memory_demo_governed_loop` (or `memory_demo_failure_insight_readback(None, Some("custom description"))`) and asserts the custom description appears in the FailureInsight fields of the persisted readback.

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
