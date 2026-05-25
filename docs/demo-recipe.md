# Governed FailureInsight Learning Demo Recipe

This recipe walks a human operator through ARPAGONA's governed learning loop end-to-end,
using only local CLI commands and in-memory Graph Memory.

## Purpose

Demonstrate the complete non-negotiable governed path:

```
safe operational signal
→ ProposedAction (create_failure_insight_memory)
→ Decision Gate approval
→ decision audit event
→ approved local Graph Memory persistence
→ FailureInsight readback with decision/audit trace proof
→ cross-invocation readback via demo snapshot
```

## Prerequisites

- [Rust toolchain](https://rustup.rs) (latest stable)
- Repository cloned at `/home/thibaud/arpagona-agent-core` (or your local path)
- No running API server required — the demo runs entirely in-memory

## Step 1: Baseline verification

Before running the demo, confirm the codebase compiles and tests pass:

```bash
cd /home/thibaud/arpagona-agent-core
cargo fmt -- --check
cargo check
cargo test
```

Expected: all tests pass (approximately 134 tests across all crates).

## Step 2: Run the governed FailureInsight demo (human-readable)

```bash
cargo run -q --bin arpagona -- memory demo failure-insight
```

This single command simulates the entire governed learning loop:

1. **Signal** — creates a safe operational signal (`runtime_observation`)
2. **Proposal** — materializes a `create_failure_insight_memory` ProposedAction
3. **Decision Gate** — evaluates the proposal and approves it
4. **Audit** — records a decision audit event with causal trace links
5. **Persistence** — persists the approved FailureInsight artifact to in-memory Graph Memory
6. **Readback** — reads back the persisted artifact and proves causal trace linkage

Expected output includes:

```
FailureInsight memory demo
  signal_type:          runtime_observation
  signal_summary:       safe bounded FailureInsight learning signal
  correction_target:    memory
  provenance:           local in-memory demo
  proposed_action_id:   prop-act-...
  memory_write_kind:    create_failure_insight_memory
  decision_id:          decision-...
  decision_status:      approved
  decision_reason:      Approved: governed FailureInsight memory write ...
  audit_event_id:       audit-...
  persisted_failure_insight_id: insight-demo-governed-learning-loop
  readback_found:       true
  readback_audit_event_count: 1
  readback_relation_count:     2
  functional_alpha_chain:
    → safe operational signal
    → create_failure_insight_memory ProposedAction
    → Decision Gate approval
    → decision audit event
    → approved local Graph Memory persistence
    → FailureInsight readback with decision/audit trace proof
  repeatable_demo_recipe:
    → run the exact_local_command from the repository root
    → verify decision_status is approved before treating persistence as expected demo behavior
    → verify readback_found is true and readback_audit_event_count is at least 1
    → optionally rerun with --inspect-id insight-demo-governed-learning-loop
    → treat all output as local evidence only, not authorization or durable user memory
```

**What to verify:**
- `decision_status` is `approved`
- `readback_found` is `true`
- `readback_audit_event_count` is at least `1`
- `persisted_failure_insight_id` is set (not `None`)
- The functional alpha chain lists all 6 steps

## Step 3: Run the demo with JSON output

```bash
cargo run -q --bin arpagona -- memory demo failure-insight --json
```

Produces structured JSON with the same fields as the human-readable output.
This is useful for programmatic inspection or integration testing.

Key JSON fields to inspect:
- `signal.signal_type`: `"runtime_observation"`
- `decision_status`: `"approved"`
- `readback_found`: `true`
- `persisted_failure_insight_id`: `"insight-demo-governed-learning-loop"`
- `functional_alpha_chain`: array of 6 steps
- `repeatable_demo_recipe`: array of 5 steps
- `warning`: `"Local demo only: ..."`

## Step 4: Inspect the persisted FailureInsight artifact by ID

```bash
cargo run -q --bin arpagona -- memory demo failure-insight --json --inspect-id insight-demo-governed-learning-loop
```

This reruns the demo and inspects the specific FailureInsight artifact
by its canonical ID after persistence.

Expected differences from Step 3:
- `inspected_failure_insight` is present (not `null`)
- `inspected_failure_insight.found`: `true`
- `inspected_failure_insight.summary`: describes the correction target
- `inspected_failure_insight.decision_id`: matches the `decision_id` field
- `inspected_failure_insight.audit_event_id`: matches the `audit_event_id` field

This proves that the persisted artifact is linked to its decision and audit event.

## Step 5: Cross-invocation readback proof (snapshot)

This step proves that the governed learning loop output survives across
separate process invocations — the key property that distinguishes
durable memory from session-only state.

```bash
# Step 5a: Run the demo and write a snapshot to disk
cargo run -q --bin arpagona -- memory demo failure-insight --json --snapshot-path /tmp/arpagona-demo-snapshot.json

# Step 5b: Read back the snapshot in a separate process invocation
cargo run -q --bin arpagona -- memory demo snapshot-read /tmp/arpagona-demo-snapshot.json

# Step 5c: Read back with full JSON for programmatic inspection
cargo run -q --bin arpagona -- memory demo snapshot-read /tmp/arpagona-demo-snapshot.json --json
```

Snapshot readback output includes:
```
📸 Demo Snapshot Readback (file: /tmp/arpagona-demo-snapshot.json)

Evidence token: EVIDENCE_ONLY

Functional alpha chain achieved:
  → safe operational signal
  → create_failure_insight_memory ProposedAction
  → Decision Gate approval
  → decision audit event
  → approved local Graph Memory persistence
  → FailureInsight readback with decision/audit trace proof
```

**What this proves:**
- Process 1 creates a governed FailureInsight memory and writes the proof to disk
- Process 2 independently reads and displays that proof
- The readback contains the full functional alpha chain evidence
- The `evidence_only_token` prevents treating the readback as authorization

## Step 6: Automated integration test (CI-safe)

The cross-process integration test automates the snapshot proof in CI:

```bash
cargo test -- cross_invocation_demo_snapshot_proves_readback_across_process_invocations
```

This test:
1. Creates a temp directory
2. Runs `memory demo failure-insight --snapshot-path` as a subprocess
3. Runs `memory demo snapshot-read --json` as a separate subprocess
4. Asserts the readback contains the expected evidence token and functional alpha chain
5. Cleans up the temp directory

Another test proves that reading a nonexistent snapshot path returns an error:

```bash
cargo test -- snapshot_read_reports_missing_file_error
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                  Governed FailureInsight Learning Loop           │
├─────────────────────────────────────────────────────────────────┤
│ 1. Runtime observation (signal)                                 │
│ 2. → ProposedAction (create_failure_insight_memory)            │
│ 3. → Decision Gate (evaluate permission, risk, policy)         │
│ 4. → Decision (approved/blocked/needs_human)                    │
│ 5. → Audit event (causal trace links)                          │
│ 6. → Approved persistence (in-memory or JSON snapshot)         │
│ 7. → Readback proof (decision + audit trace linkage)           │
└─────────────────────────────────────────────────────────────────┘
```

## Safety invariants maintained

- All persistence is local and simulated/internal
- The `EVIDENCE_ONLY_TOKEN` prevents readback-as-authorization drift
- No real tool execution, shell access, or external side effects
- No scheduler, autonomy, MCP, or browser automation
- No hidden context injection into LLM prompts
- No broad autonomous memory writing
- No personal or sensitive memory is created
- No Decision Gate or audit behavior was changed
- All state is ephemeral (in-memory) or explicitly snapshot-based

## Limits of this demo

- The demo signal is hardcoded, not generated from real observations
- The persistence path is in-memory or JSON file (not durable SurrealDB)
- The CLI cannot yet accept a user-provided observation and route it through the governed path
- There is no real FailureInsight accumulation or learning across multiple cycles
- Readback is evidence only and must not be treated as authorization

## Next steps for a human operator

Once the demo is verified:

1. Read `PROJECT_STATUS.md` for current implementation status
2. Read `docs/failure-to-insight.md` for the full doctrine
3. Explore the CLI audit readback commands:
   ```bash
   cargo run -- audit decision-summary <decision-id>
   cargo run -- audit task-summary <task-id>
   cargo run -- audit workspace-summary <workspace-id>
   ```
4. Inspect Graph Memory alpha status:
   ```bash
   cargo run -- memory status
   cargo run -- memory proposals
   ```
5. Consider what the next architectural increment should be:
   - Accepting a real observation from CLI input
   - Transitioning to a SurrealDB-backed persistence backend
   - Adding CLI commands for listing all persisted FailureInsights
   - Creating a FailureInsight accumulation workflow across cycles

## Reference commands

| Step | Command | What it proves |
|------|---------|----------------|
| 2 | `memory demo failure-insight` | Complete governed loop (text) |
| 3 | `memory demo failure-insight --json` | Complete governed loop (JSON) |
| 4 | `memory demo failure-insight --inspect-id ... --json` | Persisted artifact inspection |
| 5a | `memory demo failure-insight --snapshot-path /tmp/x.json` | Cross-invocation write |
| 5b | `memory demo snapshot-read /tmp/x.json` | Cross-invocation readback |
| 6 | `cargo test -- cross_invocation` | Automated CI-safe proof |
