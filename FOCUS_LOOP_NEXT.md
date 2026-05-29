# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP cron 2026-05-29 — P3-7 complete)

**main is green:** ✅ 821 tests pass (0 failures across full workspace).

**Phase 3 progress:**
- P3-0 (Roadmap definition): ✅ Completed in #168
- P3-1 (Domain contract): ✅ Completed in #169
- P3-2 (Deterministic loop skeleton): ✅ Completed in #170
- P3-3 (Orchestrator CLI/MCP readback): ✅ Completed in #171
- P3-4 (Memory-aware context integration): ✅ Completed in #172
- P3-5 (Cycle Trace V0): ✅ Completed in #173
- P3-4e (ToolRuntimeAdapter): ✅ Merged (#174)
- P3-4d (HolographicMemoryAdapter): ✅ Merged (#175)
- P3-4c (ReservoirEchoAdapter): ✅ Merged (#176)
- P3-4b (CompressedCognitiveAttentionAdapter): ✅ Merged (#177)
- P3-4a (GraphMemoryAdapter): ✅ Merged (#180)
- P3-6 (Integration verification spring): ✅ Merged (#181)
- **P3-7 (Proposal routing and Decision Gate integration): ✅ Completed — `ProposalGenerator` trait with `SimulatedProposalGenerator` (default) and `LlmProposalGenerator` (feature-gated). 6 new tests proving non-authorizing generation, gate integration, and permission blocking.**

## Next action

**P3-8: Proposal routing CLI surface** — Add `--llm` or `--proposal-generator` flag to `arpagona orchestrator run` so operators can switch between `SimulatedProposalGenerator` and `LlmProposalGenerator` at the CLI. This integrates the C1 milestone (real LLM proposal-only mode) into the orchestrator context.

**Open:** PR pending (feat/p3-7-proposal-routing-decision-gate) — needs push + PR creation.
**DV backlog:** 0 open entries.
