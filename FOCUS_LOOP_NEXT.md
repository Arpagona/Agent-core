# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: create a CLI command `arpagona memory demo governed-loop` that runs the full end-to-end governed FailureInsight learning loop in one command and prints the chain (signal → proposal → decision → audit → persistence → readback) including any operator-supplied `--description`.

Why: the `--description` flag and the full governed path are now verified by an end-to-end test, but there is no single CLI command that runs the complete loop — only the test and the existing `failure-insight` demo subcommand. A dedicated `governed-loop` demo command would make the repeatable demo recipe self-contained and operator-friendly.

Proof to seek: a working `cargo run -q --bin arpagona -- memory demo governed-loop --description "test"` that prints the full chain with signal summary containing "test".

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
