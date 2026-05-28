# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-30 DEEP implementation — P3-2 merged)

**main is green:** ✅ PR #170 (P3-2 Neutral Orchestrator V0 deterministic loop skeleton) was merged after all checks passed.

**Open PRs:** None known after PR #170 merge.

**Phase 3 progress:**
- P3-0 (Roadmap definition): ✅ Completed in #168
- P3-1 (Domain contract): ✅ Completed in #169
- **P3-2 (Deterministic loop skeleton): ✅ Completed in #170**
- P3-3 (Orchestrator CLI/MCP readback): 🔜

## Next action

**P3-3 — Orchestrator CLI/MCP readback surface.**

Add a CLI readback surface and/or MCP resource for the orchestrator cycle state, making the `OrchestratorCycle` and `OrchestratorOutcome` inspectable by operators and external agents:

- add `arpagona orchestrator run --objective <TEXT> --json` CLI command that instantiates the skeleton and prints the cycle trace;
- optionally add an MCP resource `arpagona://orchestrator/cycle/<id>` for external agent inspection;
- tests prove CLI output structure, JSON output, and that outputs remain non-authorizing.
