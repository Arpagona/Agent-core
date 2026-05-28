# ARPAGONA Agent Core — Business Prospecting Workflow Demo (E2)

A realistic business prospecting workflow demonstrating ARPAGONA's complete AI-assisted sales pipeline: from prospect qualification to governed action proposals, with full audit and operator visibility.

## Scenario

**NovaTech Consulting** is a digital transformation consulting firm prospecting a new client:

**Maison de la Culture Numérique (MCN)** — A Lyon-based cultural center seeking an integrated visitor management and workshop reservation system. Budget: 40 000–60 000 €. Deadline: September 2027.

The demo follows the complete business prospecting workflow:

```
Prospect Brief → Cognitive Analysis → Document Discovery
→ Assessment → Decision Gate → Audit → Operator Readback
```

## Prerequisites

- Rust toolchain (cargo, rustc)
- `cargo build` must have been run at least once

## Quick Start

### Standard demo (no LLM)

```bash
# From the repository root:
bash demos/business-prospecting/demo.sh
```

This demonstrates the full prospecting workflow using only the existing CLI commands and deterministic mock LLM output. No network, no API keys, no local model required.

## Demo Phases

### Phase 1 — Cognitive Analysis with LLM Synthesis

ARPGONA analyzes the prospect brief using the `cognitive run` command with `--domain business` and `--llm --provider mock`:

- **Working Memory** — domain classification (Business), sensitivity, complexity, assumptions
- **Cognitive Plan** — ordered analysis steps
- **LLM Synthesis** — structured proposal-only analysis with [STATE], [KEY GAP / RISK], [RECOMMENDED NEXT STEP] sections
- **Non-authorizing** — every proposed next action is `non_authorizing: true`

### Phase 2 — Document Discovery via Tool Runtime

The bounded read-only Tool Runtime performs three tasks:

- **`list_files`** — discovers documents in the prospect directory
- **`read_file`** — reads the prospect brief (MCN requirements, budget, timeline)
- **`search_text`** — searches for budget-related keywords across documents

All access is workspace-scoped, size-limited, and security-blocked.

### Phase 3 — Governed Cognitive Assessment

The `--assess --observe --govern` flags exercise the complete offline governance chain:

```
Assessment → FailureInsightCandidates → Decision Gate → Decision → AuditEvent
```

Each governance result contains:
- **ProposedAction** — action type, risk level, rationale
- **Decision** — approval status with audit trace
- **AuditEvent** — event type, before/after state, links

### Phase 4 — Follow-up Action Proposal

A business follow-up action is proposed and evaluated by the Decision Gate:

- **`action propose`** — creates a PendingDecision proposed action
- **`action evaluate`** — routes through Decision Gate for approval/rejection

### Phase 5 — Operator Readback Surfaces

Two operator surfaces confirm the complete chain:

- **`llm journal --json`** — LLM interaction journal with provider, model, prompt/response summaries
- **`status --json`** — Operator status surface with subsystem health

## Governance Boundaries

The demo preserves all ARPAGONA safety rules:

| Rule | Enforcement |
|------|-------------|
| Read-only tool access | All tools are workspace-scoped, size-limited, security-blocked |
| Non-authorizing output | Cognitive proposals have `non_authorizing: true` |
| Governance chain | Every decision passes through Decision Gate → Audit |
| No external effects | No API server, real LLM calls, file writes, or scheduler |
| Operator visibility | All output is inspectable via structured JSON |
| Deterministic mock | `--provider mock` for reproducible demos without API keys |

## Files

```
demos/business-prospecting/
├── README.md                          ← This file
├── demo.sh                            ← Main demo script
└── samples/
    ├── prospect-brief.md              ← MCN prospect brief (French)
    └── background-research.md         ← Market research context
```

## How This Differs from E1 (SME Documentary Assistant)

| Aspect | E1 — SME Documentary | E2 — Business Prospecting |
|--------|---------------------|---------------------------|
| Primary focus | Document analysis & governance demo | Business workflow demo |
| User story | Technical team evaluating a project | Sales team qualifying a prospect |
| Key actions | Read documents → Cognitive analysis → Governance | Prospect analysis → Doc discovery → Follow-up actions |
| Demo narrative | "Can the system analyze project requirements?" | "Can the system assist a full prospecting cycle?" |
| Output emphasis | Governance chain proof | Workflow completeness + sales readiness |

## Next Steps

1. **Real LLM integration:** Run with `--provider ollama` to use local qwen3.5:9b
2. **Multi-prospect comparison:** Extend demo to compare two prospects side-by-side
3. **Proposal generation:** Add a phase generating a structured commercial proposal outline
4. **E3 — Local company assistant demo pack:** Combine E1 + E2 into a reusable demo pack
5. **E4 — README: demo in 10 minutes:** One-page quickstart for human operators
