# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add a documented operator CLI workflow or demo recipe showing how to run `arpagona memory demo failure-insight --description "..."` locally and inspect the governed learning loop output.

Why: the `--description` flag now has an automated test proving full governed path propagation, but there is no operator-facing CLI walkthrough or README/demo recipe that documents how a human can run it locally, which weakens the functional-alpha chain from "tested" to "usable."

Proof to seek: a `docs/` or README update with a CLI example showing the exact commands and expected output for running the operator-supplied description demo from scratch, or a cargo run invocation that a human can copy-paste and verify the description appears in the output.

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
