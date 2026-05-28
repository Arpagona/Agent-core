# ARPAGONA Agent Core — SME Documentary Assistant Demo (E1)

A realistic SME business document analysis scenario demonstrating the full governed cognitive pipeline of the ARPAGONA cognitive runtime.

## Scenario

**Artisans du Sud** — A cooperative of 12 craftspeople in Toulon, France, needs to migrate from a static WordPress site to a complete e-commerce platform with multi-workshop stock management, product customization, and a loyalty program. Budget: 15 000–20 000 €. Deadline: 3 months.

Three documents are provided as sample inputs:

| Document | Description |
|----------|-------------|
| `samples/client-brief.md` | SME client brief (French) — context, objectives, constraints |
| `samples/project-requirements.md` | Functional specifications by consulting firm |
| `samples/commercial-proposition.md` | ARPAGONA commercial proposal draft |

## Purpose

This demo proves that ARPAGONA Agent Core can perform the complete offline governed cognitive pipeline:

```
Tool Runtime Read → Cognitive Analysis → Assessment
→ Decision Gate → Decision → Audit → Operator Readback
```

All without an API server, LLM calls, or external side effects.

## Prerequisites

- Rust toolchain (cargo, rustc)
- `cargo build` must have been run at least once (or `cargo check`)

## Quick Start

### Standard demo (no LLM)

```bash
# From the repository root:
bash demos/sme-documentary/demo.sh

# Or from the demo directory:
cd demos/sme-documentary && bash demo.sh
```

### LLM-assisted demo

```bash
# Deterministic mock provider (no model required):
bash demos/sme-documentary/demo-llm.sh
bash demos/sme-documentary/demo-llm.sh mock

# Real local model via Ollama (requires qwen3.5:9b):
bash demos/sme-documentary/demo-llm.sh ollama

# Compare mock vs. Ollama in one run:
bash demos/sme-documentary/demo-llm.sh both
```

## What the Demo Does

### Phase 1 — Tool Runtime: Read-Only Document Discovery

The bounded read-only Tool Runtime demonstrates its three tools:

- **`list_files`** — discovers available documents in the samples directory
- **`read_file`** — reads the client brief and project requirements
- **`search_text`** — searches for budget-related keywords across documents

All access is workspace-scoped, size-limited, and security-blocked against path escape.

### Phase 2 — Cognitive Analysis (Proposal-Only Mode)

The `cognitive run` command processes a realistic SME objective:

```
"Évaluer la faisabilité du projet e-commerce Artisans du Sud
 (budget, périmètre, risques) à partir des documents fournis"
```

The analysis produces:
- **Working Memory** — domain classification, sensitivity, complexity
- **Cognitive Plan** — ordered steps with required observations
- **Proposed Next Action** — non-authorizing proposal (RequestContext or StopWithReport)
- **Improvement Candidates** — identified gaps and weaknesses

### Phase 3 — Governed Analysis Pipeline

The `--assess --observe --govern` flags exercise the complete offline governance chain:

```
Assessment → FailureInsightCandidates → Decision Gate → Decision → AuditEvent
```

Each governance result contains:
- **ProposedAction** — action type, risk level, rationale
- **Decision** — approval status with audit trace
- **AuditEvent** — event type, before/after state, links

### Phase 4 — Operator Readback Surfaces

- **System status** via `status --json` — tools, decision gate, LLM provider
- **LLM interaction journal** via `llm journal` — provider, model, prompt, response
- **Optional LLM-assisted run** with `--llm --provider mock` for deterministic output

## Governance Boundaries

The demo preserves all ARPAGONA safety rules:

| Rule | Enforcement |
|------|-------------|
| Read-only tool access | All tools are workspace-scoped, size-limited, security-blocked |
| Non-authorizing output | Cognitive proposals have `non_authorizing: true` |
| Governance chain | Every decision passes through Decision Gate → Audit |
| No external effects | No API server, LLM calls, file writes, or scheduler |
| Operator visibility | All output is inspectable via structured JSON |

## Expected Output

The demo produces structured JSON output for every command, processed through Python formatting helpers for human readability. Key metrics:

```
decision_count:      ≥1 (governed mode)
audit_event_count:   ≥1 (governed mode)
assessed:            true
governed:            true (when --govern is used)
cognitive_observations: ≥1 (when --observe is used)
```

See `expected-output.md` for a complete example transcript.

## Next Steps

1. ✅ **Real LLM integration:** `demo-llm.sh` now supports `--provider mock` (deterministic) and `--provider ollama` (local qwen3.5:9b) with full integration.
2. **Product scenario expansion:** Use different SME domains (legal document review, technical audit, market analysis).
3. **E2 — Business/prospecting workflow demo:** Create a second SME demo scenario (client qualification, proposal outline generation).
4. **Web Mission Control:** Once CLI supervision surfaces are proven, extend to read-only Web UI.

## Files

```
demos/sme-documentary/
├── README.md              ← This file
├── demo.sh                ← Main demo script
├── expected-output.md     ← Example output transcript
└── samples/
    ├── client-brief.md
    ├── project-requirements.md
    └── commercial-proposition.md
```
