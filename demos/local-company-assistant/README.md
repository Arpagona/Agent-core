# ARPAGONA Agent Core — Local Company Assistant Demo Pack (E3)

A reusable, self-contained demo pack for commercial conversations. It demonstrates ARPAGONA's cognitive runtime analyzing a real-world small business scenario through governed, auditable, and entirely local tools.

## Scenario: Boulangerie du Marché

**Boulangerie du Marché** is a small family bakery in Lyon with 5 employees. The owner has gathered:

- **Customer feedback** — 3 detailed feedback cards from regular clients
- **Operations snapshot** — opening hours, capacity, equipment status, key metrics
- **Staff suggestions** — priorities from the baker, cashier, and pastry chef

The objective: analyze the situation and propose prioritized improvements, fully governed and audited.

## What this demo proves

| Claim | Evidence |
|-------|----------|
| **Read-only tool usage** | 3 tools in action (list_files, read_file, search_text) — workspace-scoped, path-escape blocked |
| **Cognitive analysis** | Working memory, plan steps, improvement candidates — all non-authorizing |
| **Governance pipeline** | `--assess --observe --govern` chain produces decisions, audit events, governance warnings |
| **No external effects** | No API server, no LLM calls, no file writes, no scheduler |
| **Operator visibility** | Status readback shows tool count, system health; structured JSON everywhere |
| **Deterministic** | Same input → same output (no network, no randomness) |

## Quick Start

```bash
# From the repository root:
bash demos/local-company-assistant/demo.sh
```

### Prerequisites

- Rust toolchain (cargo, rustc)
- `cargo build` must have been run at least once

No network. No API keys. No local LLM.

## Files

```
demos/local-company-assistant/
├── README.md                            ← This file
├── demo.sh                              ← Main demo script (self-validating)
└── samples/
    ├── feedback-customers.md            ← 3 customer feedback cards (French)
    ├── operations-snapshot.md           ← Business operations data
    └── staff-suggestions.md             ← Employee priorities
```

## How it works

### Phase 1 — Tool Runtime Discovery

The bounded read-only Tool Runtime discovers available documents, proving workspace-scoped access with security blocking against path escape.

### Phase 2 — Cognitive Analysis

`cognitive run --domain business` processes the objective through:
- Domain classification (Business)
- Working memory formation
- Cognitive plan generation
- Improvement candidate identification
- Non-authorizing next action proposal

### Phase 3 — Tool-Assisted Observation

Three read-only tools inspect the sample documents:
- `read_file` on customer feedback, operations data, staff suggestions
- `search_text` for budget-related keywords (coût, €, prix)
- All results are observations with `failure_insight_candidate` flags

### Phase 4 — Governed Pipeline

The `--assess --observe --govern` chain:
1. Assess cognitive observations for improvement candidates
2. Observe through Tool Runtime
3. Propose actions through Decision Gate
4. Record decisions and audit events
5. All governance output carries explicit non-authorizing warnings

### Phase 5 — Operator Readback

System status shows tool count, health, and governance mode — all read-only, all locally inspectable.

## Governance Boundaries

| Rule | How this demo enforces it |
|------|--------------------------|
| Read-only tool access | All tools are workspace-scoped, size-limited, security-blocked |
| Non-authorizing output | Cognitive proposals have `non_authorizing: true` |
| Governance chain | Every decision passes through Decision Gate → Audit |
| No external effects | No API server, LLM calls, file writes, or scheduler |
| Operator visibility | Structured JSON output for every command |

## Expected Output

The demo is self-validating — it reports pass/fail for each phase. A successful run produces:

```
Phase 1 — Découverte des documents (Tool Runtime)   ✅
Phase 2 — Analyse cognitive (proposal-only)          ✅
Phase 3 — Lecture des documents via Tool Runtime     ✅
Phase 4 — Pipeline gouverné                           ✅
Phase 5 — Lecture opérateur                           ✅
```

Each phase produces structured JSON with decision_count, audit_event_count, and governance warnings.

## Reuse

To adapt this demo for your own scenario:

1. Replace the sample `.md` files in `samples/` with your own data
2. Update the `--objective` text in `demo.sh` line 70
3. Optionally change the domain (`--domain business`)

The demo structure remains valid for any text-based business analysis scenario.
