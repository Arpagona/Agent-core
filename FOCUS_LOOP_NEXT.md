# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — P3-8 complete)

**main is green:** ✅ 0 failures across full workspace.

**Phase 3 progress:**
- P3-0 through P3-7: ✅ All completed
- **P3-8 (Proposal routing CLI surface): ✅ Completed — `arpagona orchestrator run --proposal-generator simulated|llm` flag added. Default: `simulated`. `llm` wraps `MockProvider` with `LlmProposalGenerator` for real proposal-only cycle integration.**

**PR #183** (`feat/p3-8-proposal-routing-cli`) — open, needs merge.

**DV backlog:** 0 open entries.

## Next action

**P3-4f or P3-9:** The next Phase 3 step. Options:
1. **P3-9: Demo script** — Create `scripts/demo-full-orchestrator-loop.sh` demonstrating both `simulated` and `llm` proposal generators end-to-end.
2. **Orchestrator documentation** — Add P3-8 `--proposal-generator` flag to `docs/cli.md`.
3. **C2: Governed direct tool-calling by the LLM** — Allow LLM tool-call intents through the existing governance envelope.
