# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add a read-only CLI command that lists all persisted demo snapshots and their metadata (creation timestamps, description previews, functional-alpha chain steps), or if that would be too broad, add an `arpagona memory demo snapshot-list --json` command that scans a configurable snapshot directory.

Why: the current description-propagation chain is now fully proven (in-process + cross-invocation), but there is no operator-facing way to discover which snapshots exist without knowing their exact paths. A listing command would complete the snapshot management surface.

Proof to seek: `cargo run -- memory demo snapshot-list --json` returns a JSON array of snapshot metadata including at least one entry with the custom description and functional-alpha chain steps visible.

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
