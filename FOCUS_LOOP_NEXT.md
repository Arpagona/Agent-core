# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add a cross-invocation integration test that proves `--description` text survives through the demo snapshot path — run `failure-insight --json --snapshot-path` in one process, then `snapshot-read --json` in a separate process, and assert the description field appears in the readback.

Why: the current description-propagation test proves in-process propagation; the snapshot path already proves cross-invocation readback for the static demo, but there is no cross-process proof that operator-supplied description text also survives serialization, file I/O, process restart and deserialization.

Proof to seek: a new integration test (in the integration test directory or a separate test file) that spawns the built binary twice — first with `--description "cross-invocation desc" --json --snapshot-path`, then with `snapshot-read --json` — and asserts the description appears in the second invocation's output.

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
