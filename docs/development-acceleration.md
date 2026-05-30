# Development Acceleration — Cognitive Hermes-like Alpha

This document clarifies the current product-building bias for ARPAGONA Agent Core.

It complements `WHITEPAPER.md`, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md`, `docs/operating-doctrine.md`, `docs/failure-to-insight.md`, `docs/architecture.md` and `docs/roadmap.md`.

## 1. Current Intent

The project should move aggressively toward a functional **Hermes-like cognitive alpha** while preserving ARPAGONA's own architecture.

Hermes/OpenClaw are inspirations for practical agent-product ergonomics: local CLI entrypoints, developer workflow, explicit commands, scheduled loops, inspectable state and operator-friendly reporting.

ARPAGONA must go further by making cognitive architecture explicit:

- Rust-first runtime;
- Working Memory;
- Reservoir Echo for short-term continuity;
- Graph Memory for durable structured context;
- Compute Reservoir for local/cloud/worker routing;
- Reflection and Failure-to-Insight loops;
- CLI supervision as first Mission Control;
- progressive autonomy;
- governance as immune system, not primary identity.

The project must not drift into a pure compliance/audit runtime. Audit and Decision Gate matter because they make the cognitive ambition safe enough to use.

## 2. Product Bias: CLI Supervision First

The CLI is the first local Mission Control surface.

Near-term work should bias toward making the CLI useful for inspecting the cognitive system, rather than repeatedly adding abstract guards.

Preferred CLI progression:

1. show what the system is trying to do;
2. show tasks and proposed actions;
3. show decision and audit summaries;
4. show Graph Memory readback;
5. show Reservoir Echo / cognitive cycle state when available;
6. show Compute Reservoir allocation rationale when available;
7. show Failure-to-Insight artifacts and suggested improvements;
8. only then consider Mission Control Web expansion.

CLI commands must remain read-only unless a future command explicitly goes through the governed action path.

## 3. Aggressive but Bounded Development

The desired mode is aggressive iteration, not reckless expansion.

A good loop should ship one small usable increment that improves the cognitive runtime or its inspectability.

Default preference order:

1. useful read-only CLI supervision increment;
2. small Rust abstraction that improves cognitive continuity, memory, compute routing or reflection;
3. Graph Memory / Audit stabilization when it supports actual cognitive readback;
4. Runtime/API expansion only when it remains read-only or clearly governed;
5. execution/autonomy only after the governed path and reflection loops are ready.

## 4. Cognitive Direction to Preserve

The runtime should gradually become able to maintain and inspect a cognitive cycle:

```text
Input
-> Working Memory
-> Reservoir Echo
-> Graph Memory recall
-> Compute Reservoir allocation
-> ProposedAction
-> DecisionGate if needed
-> Audit
-> Reflection / Failure-to-Insight
```

Key requirements:

- each cycle should produce inspectable state;
- temporary salience must remain distinct from durable memory;
- readback must not be confused with authorization;
- compute allocation must not be confused with decision approval;
- failure insights must remain learning artifacts, not self-modification commands;
- human supervision remains required for sensitive action and structural changes.

## 5. Rippletide Direction to Preserve

The Rippletide-inspired direction is runtime enforcement.

It should be treated as a safety mechanism around the cognitive runtime.

The system must make it structurally difficult for an agent to act directly. Agents produce structured intent. The runtime evaluates intent against context, policies, permissions and risk before anything can affect the outside world.

Key requirements:

- every important action starts as a `ProposedAction`;
- applicable context should come from Graph Memory, not raw context stuffing;
- the Decision Gate decides approval, rejection, escalation or need for more context;
- important decisions produce causal audit traces;
- failures and corrections can be converted into durable Failure-to-Insight artifacts for future improvement;
- CLI/Mission Control surfaces inspect traces and decisions, but do not replace governance;
- readback is not approval;
- Tool Registry lookup is not approval;
- Graph Memory is not authorization.

## 6. Compute Reservoir Direction

The system should move toward real local/cloud delegation rather than using cloud reasoning for everything.

Compute Reservoir should become a cognitive router that chooses which resource should think, read, summarize, extract, draft or reason.

Hermes/cloud-like orchestration should be reserved for final judgment, planning and integration when justified.

LOCO/Ollama/local models should handle first-pass reading, summarization, extraction, draft generation and log analysis whenever practical.

Future Compute Reservoir work should make this explicit in code: choosing resources based on capability, cost, latency, data sensitivity, local availability, fallback strategy and observed performance.

## 7. What to Avoid Now

Avoid:

- reducing the project to audit/governance only;
- endless test-only stabilization when a cognitive/product-visible readback surface is possible;
- growing API/Runtime before CLI supervision is useful;
- treating CLI as a debug toy rather than the first local Mission Control surface;
- copying Hermes without preserving ARPAGONA's cognitive architecture;
- implementing execution before the Tool Registry, Decision Gate, Audit and reflection path are ready;
- letting Graph Memory, CLI, API or Runtime become hidden authorization layers.

## 8. Near-Term Success Target

A useful near-term alpha should let a human operator run local CLI commands to understand:

- what the system is trying to do;
- what tasks exist;
- what actions were proposed;
- what decisions were made;
- why they were made;
- what context or memory was involved;
- which policies and risks were involved;
- what audit events support the answer;
- what failure insight or suggested improvement emerged;
- what remains pending.

That is the practical bridge between the current Rust foundation and a future Mission Control Web interface.

## 9. Local Development Workflow

### 9.1. Smoke testing

Use `scripts/smoke-human-cli.sh` for a quick human-path smoke check:

```bash
# Quick smoke (pre-built binary, ~20s):
bash scripts/smoke-human-cli.sh

# Full smoke including orchestrator trace, compute routing, audit (requires pre-built binary, ~60s):
bash scripts/smoke-human-cli.sh --all

# Rebuild first, then smoke:
bash scripts/smoke-human-cli.sh --build --all
```

Each command has a 20-second timeout. Failures are reported individually. Exit code = number of failed tests.

### 9.2. Process doctor

Use `scripts/dev-process-doctor.sh` to inspect stale development processes:

```bash
# Read-only report:
bash scripts/dev-process-doctor.sh

# With kill prompts:
bash scripts/dev-process-doctor.sh --kill
```

The doctor inspects:
- running `arpagona-api-server` and `arpagona chat` processes;
- port occupancy (3000, 3001);
- cargo build/check contention.

### 9.3. CARGO_TARGET_DIR convention

When DEEP (Hermes) and a human operator or GONA both use the same repo, `cargo build` contention on the shared `target/` directory makes the CLI unusable for the second party. Two recommended approaches:

**Option A — dedicated binary path (preferred for smoke tests)**

Build once, then use `target/debug/arpagona` directly instead of `cargo run`:

```bash
# Agent/CI builds:
cargo build -p arpagona-cli

# Human smoke (does not trigger cargo):
target/debug/arpagona run "mon objectif"
target/debug/arpagona status
```

**Option B — per-profile target directories (advanced)**

Set `CARGO_TARGET_DIR` per agent profile to prevent build contention entirely:

```bash
# deep profile (Hermes):
export CARGO_TARGET_DIR=target/deep

# gona profile:
export CARGO_TARGET_DIR=target/gona

# human/operator:
export CARGO_TARGET_DIR=target/human
```

This prevents lock contention but requires separate builds per profile. Useful when DEEP is compiling while the operator wants to run.

**Convention: prefer Option A** unless build contention causes daily friction. Document the per-profile target choice in the agent's profile config if switching to Option B.
