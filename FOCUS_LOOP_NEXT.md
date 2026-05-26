# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next scheduled focus-loop run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Next action

Next pass should: create `scripts/demo-full-loop.sh` — a single repeatable shell script that runs the complete governed loop in one command: `cognitive run --assess --observe --govern --json`, producing structured governance readback without requiring an API server.

Why: P3 governance chain now has integration tests proving offline `--govern` works; the next step is making it trivially demonstrable in one invocation. P7 in the milestone queue explicitly calls for this script.

Proof to seek: `bash scripts/demo-full-loop.sh` exits 0 and prints valid JSON containing `governance_results`, `decision_count > 0`, `audit_event_count > 0`, and the `governance_warning`. After the script, `cargo fmt -- --check && cargo test --workspace` still passes.

Do not: add new CLI flags, API endpoints, persistence, runtime behavior, Decision Gate changes, or LLM calls. The script must only invoke existing CLI commands.
