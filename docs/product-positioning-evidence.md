# ARPAGONA Agent Core — Product Positioning Evidence (E5)

> **Track E — Step E5**
>
> Product positioning evidence: extract 3–5 claims the demo proves, map claims to implementation evidence, avoid overclaiming autonomy or AGI, prepare language usable for ARPAGONA Agent communication.

This document synthesises the technical progress from Tracks C, D, and E into reusable product-positioning evidence. It is designed for commercial conversations, investor discussions, partnership proposals and product documentation.

---

## One-Line Positioning

> **ARPAGONA is a local-first cognitive agent runtime where every action is proposed, gated, decided and audited — never executed directly.**

Alternative one-liners (shorter):

> **A Rust-based cognitive runtime with governance as its immune system.**

> **Local-first agent cognition. Bounded execution. Full audit traceability.**

---

## Claim 1 — Complete offline governed cognitive pipeline

### What the demos prove

ARPAGONA can run a complete cognitive agent pipeline — from file discovery through cognitive analysis, Decision Gate evaluation, audit recording and operator readback — with **zero network, zero API keys, zero LLM dependencies**:

```
Tool Runtime Read → Cognitive Analysis → Assessment
→ Decision Gate → Decision → Audit → Operator Readback
```

### Evidence in the repository

| Evidence | Location | Status |
|----------|----------|--------|
| E1 demo (SME Documentary Assistant) runs 4-proof offline phases | `demos/sme-documentary/demo.sh` | ✅ Proven in CI |
| E2 demo (Business Prospecting) runs 5-phase sales workflow | `demos/business-prospecting/demo.sh` | ✅ Proven in CI |
| E3 demo (Local Company Assistant) runs 5-phase pack with self-validation | `demos/local-company-assistant/demo.sh` | ✅ 11 tests pass |
| E4 demo-in-10-minutes README | `demos/local-company-assistant/README.md` | ✅ Published |
| All demos pass without network, API keys or local LLM | CI runs, scripts | ✅ Reproducible |

### Claim usage

> *"ARPAGONA can analyse business documents, propose actions, gate every decision through a safety layer and produce a full audit trail — entirely offline, on any machine with Rust toolchain, in under 60 seconds."*

### Boundaries (what we do not claim)

- ❌ Not autonomous — no cycle executes without the governance chain
- ❌ Not AGI — the cognitive analysis is deterministic heuristic + optional LLM-assisted proposal
- ❌ Not production-ready — alpha quality, experimental crate boundaries
- ✅ The chain is reproducible, demonstrable and independently verifiable

---

## Claim 2 — Read-only safe perception with bounded tool runtime

### What the demos prove

The Tool Runtime provides **workspace-scoped, security-blocked, size-limited** read-only tools:

- **read_file** — blocked on `../`, `/absolute/paths`, `.git`, `.env`, files > 10 KB
- **list_files** — skips `.git/`, `target/`, `node_modules/`, `.ssh/`
- **search_text** — bounded to 50 results, 5 KB per file

This is not a placeholder. The blocking is enforced at the code level: lexical `../` escape detection runs before `canonicalize()`, and sensitive paths return `Blocked` with `is_security: true`.

### Evidence in the repository

| Evidence | Location | Status |
|----------|----------|--------|
| Tool Runtime path-escape blocking with security flag | `crates/tool-runtime/src/lib.rs` | ✅ Safety boundary tests pass |
| `.git` and `.env` blocking | `crates/tool-runtime/src/lib.rs` | ✅ 13 unit tests |
| Size limits and result bounds | `crates/tool-runtime/src/lib.rs` | ✅ Functional test coverage |
| CLI surfaces: `tool list`, `inspect`, `demo` commands | `crates/cli/src/main.rs` | ✅ CLI tests pass |
| Safety boundary tests in daily validation | `docs/daily-agent-validation.md` Phase E | ✅ Documented protocol |

### Claim usage

> *"ARPAGONA can safely read business documents without risk of data exfiltration, modification or path escape. The Tool Runtime enforces workspace scope at the filesystem level — not just at the prompt level."*

### Boundaries (what we do not claim)

- ❌ Not a general-purpose execution sandbox — no shell, browser, MCP, email, write tools exist
- ❌ Not a security-hardened production tool — alpha quality, experimental blocking logic
- ✅ Bounded, readable, tested behaviour for the documented use case

---

## Claim 3 — Non-authorizing cognitive analysis with mandatory governance

### What the demos prove

Every cognitive proposal in ARPAGONA carries **`non_authorizing: true`**. The system does not execute analysis results. Instead:

1. The cognitive engine produces proposals (Working Memory, Plan, Proposed Next Action, Improvement Candidates)
2. Every proposal is structurally marked as non-authorizing
3. Any action requiring side effects must pass through Decision Gate
4. Blocked actions produce audit events — they are not silently dropped

The Decision Gate is not a configuration toggle that can be skipped. It is an architectural invariant: the `--govern` flag invokes the same gate code that governance-level execution would use.

### Evidence in the repository

| Evidence | Location | Status |
|----------|----------|--------|
| `non_authorizing: true` on every `ProposedNextAction` | `crates/core/src/cognitive_work.rs` | ✅ 24 cognitive work tests |
| Decision Gate crate with policy evaluation | `crates/decision-gate/src/lib.rs` | ✅ Alpha governance logic |
| CLI `--assess --observe --govern` chain | `crates/cli/src/main.rs` | ✅ 40+ CLI tests |
| Governed FailureInsight demo loop | CLI: `memory demo failure-insight` | ✅ Snapshot chain proven |
| Warning on non-governed runs | CLI output contains `governance_warning` | ✅ Acceptance criteria |

### Claim usage

> *"ARPAGONA analyses and proposes without acting. Every suggestion is marked non-authorizing, and every executable path requires an explicit Decision Gate evaluation. This is not a prompt instruction — it is a compiled runtime invariant."*

### Boundaries (what we do not claim)

- ❌ Not zero-risk — a future governance bypass bug would be a critical issue
- ❌ Not a human approval system — Decision Gate decisions are deterministic, not human-in-the-loop
- ✅ The non-authorizing invariant is enforceable, testable and independently verifiable

---

## Claim 4 — Complete audit traceability

### What the demos prove

Every governance decision produces a structured audit trail:

- **Audit events** with IDs, timestamps, before/after state and source links
- **Decision records** showing proposed action, approval status and rationale
- **Governance warnings** when something needs operator attention
- **Snapshot persistence** — the governed loop output survives across process invocations

The audit chain is not an afterthought. It is the terminal output of four different CLI commands (`status --json`, `memory demo failure-insight`, `llm journal`, `tool list --json`) and is structurally suitable for programmatic consumption.

### Evidence in the repository

| Evidence | Location | Status |
|----------|----------|--------|
| Audit event recording and readback | `crates/graph-memory/src/audit_store.rs` | ✅ Tests pass |
| Snapshot persistence: write → read → list | `crates/graph-memory/src/demo_snapshot.rs` | ✅ 4 snapshot tests |
| Cross-invocation description propagation | CLI `memory demo snapshot-read` | ✅ Integration test |
| LLM interaction journal | CLI `llm journal --json` | ✅ 38 llm tests |
| Decision-scoped audit readback | CLI `audit decision` commands | ✅ Alpha proof |

### Claim usage

> *"Every decision ARPAGONA makes is traceable. Operators can inspect what was proposed, why it was evaluated that way, what was decided, and what audit trail was produced — all through structured JSON output, with snapshot persistence across process invocations."*

### Boundaries (what we do not claim)

- ❌ Not a compliance-audit system — no retention policies, no cryptographic signing, no SIEM integration
- ❌ Not a security audit — the audit is alpha quality and does not detect model-level manipulation
- ✅ The audit trail is inspectable, machine-readable and structurally complete for the documented use case

---

## Claim 5 — Layered cognitive architecture (beyond single-model agents)

### What the demos prove

ARPAGONA is not a thin wrapper around a single LLM. It implements five distinct cognitive layers, each as its own crate:

| Layer | Role | Implementation |
|-------|------|----------------|
| **Working Memory** | Active context for current cycle | `crates/core/src/cognitive_work.rs` — pure types |
| **Reservoir Echo** | Short-term volatile continuity | `crates/core/src/reservoir_echo.rs` — activation/decay |
| **Holographic Memory** | Non-authorizing pattern resonance | `crates/holographic-memory` — 22+ tests, SQLite persistence |
| **Graph Memory** | Structured durable memory with provenance | `crates/graph-memory` — SurrealDB adapter, audit |
| **Compute Reservoir** | Cognitive resource routing | `crates/compute-reservoir` — allocation types, deterministic |

No other local-first agent runtime publicly documents this architecture separation.

### Evidence in the repository

| Evidence | Location | Status |
|----------|----------|--------|
| Working Memory types and heuristic engine | `crates/core/src/cognitive_work.rs` | ✅ 24 tests |
| Reservoir Echo: pulses, activation, decay, reinforcement | `crates/core/src/reservoir_echo.rs` | ✅ Core tests |
| Holographic Memory: deterministic signatures, resonance, SQLite persistence | `crates/holographic-memory/src/lib.rs` | ✅ 22+ tests |
| Graph Memory: SurrealDB persistence, audit, FailureInsight | `crates/graph-memory/src/lib.rs` | ✅ 24 tests |
| Compute Reservoir: allocation, capability routing | `crates/compute-reservoir/src/lib.rs` | ✅ Alpha pure crate |
| Architecture documentation separating all layers | `docs/architecture.md`, `PROJECT_OBJECTIVES.md` | ✅ Published |

### Claim usage

> *"ARPAGONA separates working context, short-term continuity, pattern resonance, durable memory and compute routing into five distinct cognitive layers — each with its own crate, tests and documented boundaries. This architecture is designed for progressive capability growth, not single-LLM dependency."*

### Boundaries (what we do not claim)

- ❌ Not a proven cognitive architecture — the layers are experimental and alpha-quality
- ❌ Not a replacement for vector databases, embeddings or neural retrievers — Holographic Memory is symbolic, not learned
- ❌ Not a general-purpose multi-agent framework — no inter-agent messaging, no distributed orchestration
- ✅ The architecture layering is explicit, testable and independently verifiable

---

## Summary Evidence Table

| # | Claim | Primary Demo | Crate/Code Evidence | Test Count (approx.) |
|---|-------|-------------|---------------------|----------------------|
| 1 | Complete offline governed cognitive pipeline | E1, E2, E3 — all 5-phase demos | `demo.sh` scripts, CLI `--assess --observe --govern` | 11 (E3 validation tests) |
| 2 | Read-only safe perception with bounded tool runtime | E1 Phase 1, E3 Phase 1 | `crates/tool-runtime/src/lib.rs` | 13 (tool-runtime crate) |
| 3 | Non-authorizing cognitive analysis with mandatory governance | E1 Phase 2, E3 Phase 2 | `crates/core/src/cognitive_work.rs`, `crates/decision-gate` | 24 (cognitive work) |
| 4 | Complete audit traceability | E3 Phase 4–5, snapshot chain | `crates/graph-memory/src/audit_store.rs`, `demo_snapshot.rs` | 24 (graph-memory crate) |
| 5 | Layered cognitive architecture | Architecture docs, crate-level tests | All 5 cognitive crates (see above) | 100+ across all crates |

---

## Anti-Claims — What We Explicitly Do Not Claim

| Do not claim | Why | What we say instead |
|--------------|-----|---------------------|
| AGI or general intelligence | Deterministic + LLM-assisted proposals, not general reasoning | "cognitive runtime" — structured analysis bounded by governance |
| Production-ready or enterprise-grade | Alpha quality, experimental layers, no SLAs | "alpha" — demonstrable, inspectable, evolving |
| Autonomous agent | All actions are proposed, gated and audited | "governed cognitive pipeline" — no autonomous execution |
| Multi-agent coordination | No inter-agent messaging runtime exists | "single cognitive runtime" — one agent, bounded tools |
| Vector-database retrieval | Holographic Memory is symbolic pattern resonance | "symbolic associative memory" — deterministic, no embeddings required |
| Security-hardened sandbox | Tool Runtime is alpha blocking logic | "bounded perception" — workspace-scoped, size-limited |
| Cloud-scale or distributed | Local-first architecture | "local-first" — single machine, no network dependencies |
| LLM-as-a-service wrapper | ARPAGONA is a multi-layer cognitive runtime | "cognitive runtime not LLM proxy" — working memory, reservoir echo, holographic memory, graph memory, compute reservoir are separate layers |

---

## Language for Different Audiences

### Technical audience (developers, architects)

> *"ARPAGONA Agent Core is a Rust-based cognitive agent runtime with five layered systems (Working Memory, Reservoir Echo, Holographic Memory, Graph Memory, Compute Reservoir), a bounded read-only Tool Runtime, a Decision Gate that enforces governance at compile-runtime level, and complete audit traceability with snapshot persistence. The 5-phase E3 demo runs entirely offline and produces 11 passing test assertions."*

### Business audience (SME owners, department heads)

> *"ARPAGONA reads business documents, analyses situations and proposes next steps — without being able to execute anything on its own. Every suggestion is checked by a governance layer, and every decision leaves an audit trail. The demo runs on any laptop with no internet, no API keys and no configuration."*

### Commercial / partnership audience

> *"ARPAGONA fills the gap between black-box AI agents and fully manual processes. It offers governed, auditable, local-first cognition for sensitive business data. The E3 demo pack is self-contained, reusable for any business scenario, and demonstrates the full proposal → gate → decision → audit chain with zero external dependencies."*

### Regulated-industry audience (compliance, legal, finance)

> *"ARPAGONA does not execute actions. It proposes them. Every proposal is evaluated by deterministic governance rules. Every decision produces a structured audit event. The operator can inspect what was proposed, why and what was decided at any point — without trusting a black-box model's safety prompt."*

---

## Where This Document Fits

| Phase | Product | Status |
|-------|---------|--------|
| E1 | SME Documentary Assistant demo | ✅ Complete |
| E2 | Business Prospecting workflow demo | ✅ Complete |
| E3 | Local Company Assistant demo pack | ✅ Complete |
| E4 | README: demo in 10 minutes | ✅ Complete |
| **E5** | **Product positioning evidence** | **✅ This document** |

**Next step after E5:** H1 — Production hardening pass (edge-case tests, error handling, regression tests, audit readability, dependency cleanup).

---

*This document is non-authorizing. It describes verifiable technical evidence. It does not constitute a product guarantee, security certification or performance claim. All evidence references the current repository state at the time of writing.*
