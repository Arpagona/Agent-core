# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (2026-05-28 focus loop — E1 SME Documentary Assistant Demo)

**Open PRs needing human merge (from previous run):**
- PR #139 (docs: fix DV-2026-05-28-002) — mergeable, all CI green
- PR #140 (fix: restore governance/readback regression assertions) — mergeable, all CI green
- PR #141 (fix: classify lexical parent-traversal as security, DV-2026-05-28-003) — mergeable, all CI green
- PR #142 (docs: P0 hygiene backlog alignment + H1 demo script) — mergeable, all CI green

**New this run:**
- Branch: `feat/e1-sme-documentary-demo`
- E1 SME Documentary Assistant demo created at `demos/sme-documentary/`
  - `README.md` — walkthrough of governed cognitive pipeline
  - `demo.sh` — end-to-end script (Phase 1-4: Tool Runtime → Cognitive Analysis → Governance → Readback)
  - `expected-output.md` — example output transcript
  - `samples/client-brief.md`, `project-requirements.md`, `commercial-proposition.md` — realistic SME business documents (Artisans du Sud — refonte e-commerce)
- DV-2026-05-28-001 fixed: `--exclude=PROJECT_STATUS.md` added to conflict-marker scan in `docs/daily-agent-validation.md`
- DAILY_VALIDATION_BACKLOG.md: cleaned up stale Open entries → moved to Closed

**Next action:**

After PRs #139, #140, #141, #142 are merged AND this branch's PR is merged:

1. **Run the E1 demo end-to-end** to verify it works with the merged codebase:
   ```bash
   bash demos/sme-documentary/demo.sh
   ```
2. **Extend E1 with an `--llm` variant** — create `demo-llm.sh` that runs the same scenario with `--llm --provider mock` (and optionally `--provider ollama`) for richer LLM-assisted cognitive synthesis.
3. **Track E2 — Business/prospecting workflow demo** — create a second SME demo scenario (e.g., client qualification, proposal outline generation).

If E2 is too large for one run, process the next open DV backlog item (DV-2026-05-28-005 — Ollama synthesis specificity).
