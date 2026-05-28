# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-28 focus loop — E1 LLM demo variant complete)

**Open PRs needing human merge:**
- PR #139 (docs: fix DV-2026-05-28-002) — mergeable, all CI green
- PR #140 (fix: restore governance/readback regression assertions) — mergeable, all CI green
- PR #141 (fix: classify lexical parent-traversal as security, DV-2026-05-28-003) — mergeable, all CI green
- PR #142 (docs: P0 hygiene backlog alignment + H1 demo script) — mergeable, all CI green
- PR #143 (feat: E1 SME Documentary Assistant demo) — mergeable, all CI green; updated this run with demo-llm.sh

**Completed this run:**
- Created `demos/sme-documentary/demo-llm.sh` — LLM-assisted demo variant with 3 modes:
  - `mock` (default) — deterministic mock provider
  - `ollama` — real local qwen3.5:9b model
  - `both` — mock vs. Ollama comparison
- Verified: `--provider ollama` produces French-language structured [STATE]/[KEY GAP/RISK]/[RECOMMENDED NEXT STEP] synthesis for the SME business scenario
- Verified: `llm journal` persists provider/model/prompt/response summaries for every call
- Updated `demos/sme-documentary/README.md` with LLM demo instructions
- All verification passes: cargo fmt, cargo check, cargo test (172 tests)

**Next action:**

After PRs #139, #140, #141, #142, #143 are merged:

1. **Track E2 — Business/prospecting workflow demo** — create a second SME demo scenario (client qualification, proposal outline generation), with both standard and LLM-assisted variants.
2. **Run the E1 demos end-to-end on merged main** to verify all paths work post-merge:
   ```bash
   bash demos/sme-documentary/demo.sh
   bash demos/sme-documentary/demo-llm.sh mock
   bash demos/sme-documentary/demo-llm.sh ollama
   ```
