# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: extend the demo snapshot path to include the custom `--description` in the JSON snapshot output, then add a cross-process integration test that writes a snapshot with a custom description in one process and reads it back in another.

Why: the `--description` end-to-end test was added this session, proving operator text flows through the full governed signal -> proposal -> decision -> audit -> persistence -> readback path. However, the demo snapshot path (`--snapshot-path`) does not record the description in the snapshot JSON — the snapshot always uses the default hardcoded values. Proving the description survives disk persistence and cross-process readback closes the final gap in the governed learning demo chain.

Proof to seek: a new cross-process integration test (in `crates/cli/tests/`) that invokes `memory demo failure-insight --json --snapshot-path <path> --description "custom text"` in one process, then reads the snapshot back in a separate process via `memory demo snapshot-read <path> --json`, and asserts the readback JSON contains the custom description text.

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
