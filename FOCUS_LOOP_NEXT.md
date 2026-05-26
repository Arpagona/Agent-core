# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Add `--resonate` flag to `arpagona cognitive run` CLI for direct HolographicMemory resonance readback.

Why: P6 (resonate_for_working_memory) is implemented and tested, but the CLI only exposes `--assess` and `--allocate`. Adding `--resonate` completes the cognitive chain in a single CLI command: WorkingMemory → ComputeReservoir → HolographicMemory resonance, all in one JSON output.

Proof to seek: `cargo run -- cognitive run --objective "..." --domain business --assess --allocate --resonate --json` produces a `holographic_resonance` block in the JSON output containing hints, has_resonance, and non_authorizing_warning.

Do not: modify any core types, add LLM calls, persistence, shell execution, external effects, or Decision Gate bypass. This is purely a CLI wiring change + JSON output field.

## Required update at the end of every run

Replace the next action above with a new single-step handoff:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback, PR state or file confirming progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable.
