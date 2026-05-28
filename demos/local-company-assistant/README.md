# ARPAGONA Agent Core — Local Company Assistant Demo Pack (E3)

A reusable, self-contained demo pack for commercial conversations. It demonstrates ARPAGONA's cognitive runtime analyzing a real-world small business scenario through governed, auditable, and entirely local tools.

## Quick Start

```bash
# Prerequisites: Rust toolchain, cargo build run once
bash demos/local-company-assistant/demo.sh
```

No network, no API keys, no local LLM. Works on any Linux/macOS machine with Rust.

## Scenario: Boulangerie du Marché

**Boulangerie du Marché** is a small family bakery in Lyon with 5 employees. The owner has gathered:

- [Customer feedback](samples/feedback-customers.md) — 3 detailed feedback cards from regular clients
- [Operations snapshot](samples/operations-snapshot.md) — opening hours, capacity, equipment status, key metrics
- [Staff suggestions](samples/staff-suggestions.md) — priorities from the baker, cashier, and pastry chef

The objective: analyze the situation and propose prioritized improvements, fully governed and audited.

## What the Demo Does (5 Phases)

| Phase | What Happens | Verification |
|-------|-------------|-------------|
| **1. Tool Discovery** | Lists available Tool Runtime tools, discovers sample documents | `read_file`, `list_files`, `search_text` available |
| **2. Cognitive Analysis** | Classifies domain, forms working memory, generates plan and improvement candidates | Output is `non_authorizing: true` |
| **3. Tool-Assisted Reading** | Reads 3 documents, searches for budget keywords | All 3 documents accessible, search returns results |
| **4. Governed Pipeline** | Assesses observations, proposes actions through Decision Gate, records audit events | `decision_count`, `audit_event_count`, `governance_warning` all present |
| **5. Operator Readback** | System status shows tool count, health, governance mode | Structured JSON output, no API server dependency |

## Demo Output

A successful run produces:

```
Tous les 11 tests ont réussi.
```

See [expected-output.md](expected-output.md) for detailed acceptance criteria and per-phase output.

## What This Demo Proves

| Claim | Evidence |
|-------|----------|
| **Read-only tool usage** | 3 tools — workspace-scoped, absolute paths blocked |
| **Cognitive analysis** | Working memory, plan steps, improvement candidates — all non-authorizing |
| **Governance pipeline** | `--assess --observe --govern` chain produces decisions, audit events, warnings |
| **No external effects** | No API server, no LLM calls, no file writes, no scheduler |
| **Operator visibility** | `status --json` shows tool count, health, governance mode |
| **Deterministic** | Same input → same output (no network, no randomness) |
| **Replayable** | Run the script any number of times — identical results |

## Governance and Audit Value

See [GOVERNANCE_VALUE.md](GOVERNANCE_VALUE.md) for a detailed explanation of why governance is a product feature, not a constraint — written for commercial and partnership conversations.

Key governance boundaries enforced by this demo:

- All tools are read-only, workspace-scoped, size-limited, and security-blocked
- Every cognitive proposal carries `non_authorizing: true`
- Every decision passes through Decision Gate → Audit
- No external state is modified
- Structured JSON output enables programmatic inspection

## Files

```
demos/local-company-assistant/
├── README.md                            ← This file — quick start + overview
├── GOVERNANCE_VALUE.md                  ← Governance & audit value for commercial use
├── expected-output.md                   ← Expected output with acceptance criteria
├── demo.sh                              ← Main self-validating demo script
├── test_debug.sh                        ← Quick debug helper (reads each file)
└── samples/
    ├── feedback-customers.md            ← 3 customer feedback cards (French)
    ├── operations-snapshot.md           ← Business operations data
    └── staff-suggestions.md             ← Employee priorities
```

## Adapting the Demo for Your Scenario

1. Replace the sample `.md` files in `samples/` with your own data
2. Update the `--objective` text in `demo.sh` (line 62)
3. Optionally change the domain (`--domain business`)
4. Update `test_debug.sh` with your expected search terms

The demo structure remains valid for any text-based business analysis scenario.

## Prerequisites

- Rust toolchain (`rustc`, `cargo`)
- `cargo build` must have run at least once (the demo script auto-builds if the binary is missing)
- No network, no API keys, no local LLM

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `read_file` returns empty | Absolute path used instead of relative | Use path relative to project root |
| `tool list` fails | Binary not built | Run `cargo build` once |
| Phase 4 yields 0 decisions | `--assess` missing before `--govern` | Check the CLI flag order |
| Tool count shows blank | Grep pattern in demo.sh misses space after colon | Should read: `"tool_runtime_tool_count": 3` |
| Phase 2 domain not "business" | LLM-powered cognitive run may disagree | The pure heuristic fallback classifies as `business` |
