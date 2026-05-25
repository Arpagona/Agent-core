# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Connect WorkingMemory + ComputeAllocation to Holographic Memory resonance hints.

Why: P5 (PR #85) ties WorkingMemory state to ComputeReservoir allocation — the next step is to feed these decisions into HolographicMemory::resonate() as contextual hints, closing the loop between cognitive state, resource selection, and pattern recall.

Proof to seek: A test showing that a ComputeAllocation reason string or WorkingMemory sensitivity/complexity fields can be passed to HolographicMemory::resonate() and produce a non-authorizing MatchResult with hints.

Do not: add LLM calls, persistence, shell execution, external effects, or Decision Gate bypass. P6 must remain pure readback — HolographicMemory is a non-authorizing resonance surface, not an action engine.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
