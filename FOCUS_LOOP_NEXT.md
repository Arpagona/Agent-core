# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should prioritize delivering or unblocking the repeatable governed FailureInsight learning demo:

```text
signal -> proposed FailureInsight memory write -> Decision Gate decision -> audit linkage -> approved Graph Memory persistence -> readback proof -> repeatable local demo command or test
```

Start by checking whether the full loop is already implemented and testable. If not, identify the smallest missing link that prevents the demo from being reproducible, implement that link, and report the exact command or test proving the advance.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.