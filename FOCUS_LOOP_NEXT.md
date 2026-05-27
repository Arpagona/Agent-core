# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

Two-track alternation (from 2026-05-26):
- **Track A** — MCP Server (Phases 2+)
- **Track B** — Holographic Memory (integration steps)

P1 (open PRs) takes priority over alternation.

## Next action

**Track A Phase 5 — MCP notifications / tools/list_changed.** PR #117 created this run implementing Track A Phase 5.

Wait for CI on PR #117. If CI passes green, auto-merge. Then advance to **Track B Step B6 — governance of memory writes via DecisionGate (`MemoryWriteKind::HolographicTrace`).**

Step B6 adds DecisionGate governance to holographic memory writes, ensuring that storing a `HolographicTrace` is a governed action subject to the same `ProposedAction -> DecisionGate -> Decision -> Audit` path used by other memory operations.

Implementation requires:
- Adding `MemoryWriteKind::HolographicTrace` variant to the existing `MemoryWriteKind` enum
- Adding a governance path in the holographic memory store that creates `ProposedAction` → evaluates through DecisionGate → records `AuditEvent`
- Tests proving that ungoverned holographic memory writes are blocked and governed writes produce audit traces
- Readback showing the governance decision context

Do not: add real execution, shell access, LLM calls, browser automation, email sending, or SurrealDB persistence beyond existing usage.
