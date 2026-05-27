# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track status (from 2026-05-28):
- **Track A** — MCP Server (Phases 1-5 complete ✅, A6 operator-readiness 🔜)
- **Track B** — Holographic Memory (Steps B1-B6 complete ✅, B7 cognitive-loop recall hints 🔜)

Both tracks are fully implemented through the planned phases/steps and merged into main.

Daily validation backlog:
- All items resolved or closed.

P3 status (Governed MCP observability):
- A6 operator documentation is in progress (this run). MCP server docs updated to cover full A1-A5 feature set. PROJECT_STATUS.md updated to reflect actual MCP and Holographic Memory state.

## Next action

**Advance toward P4 — General Cognitive Work Loop V0 (AGENT_FOCUS_LOOP.md P4).**

Target chain:
```text
Objective -> WorkingMemory -> Plan -> RequiredObservations -> ProposedNextAction -> ImprovementCandidate
```

Expected user-facing command:
```bash
arpagona cognitive run --objective "..." --domain business --json
```

Required properties:
- works for professional domains, not only code;
- read-only and non-autonomous;
- no LLM calls unless already supported and explicitly safe;
- produces structured working memory, plan and next action;
- exposes missing context and improvement candidates.

The current cognitive work loop in `crates/runtime` runs the cognitive chain per-invocation but does not yet produce a fully structured working memory/plan/next-action output for all domains. The next increment should bridge the gap between the existing cognitive run loop and the P4 target chain.

If P4 is too large for one run, the fallback is Track A A6 (MCP operator-readiness: documentation, examples, client smoke tests) or Track B B7 (cognitive-loop recall hints from resonance matches).

Do not: add real execution, shell access, LLM calls (to remote models), browser automation, email sending, or SurrealDB persistence beyond existing usage.
