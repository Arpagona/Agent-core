# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next supervised work run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (DEEP 2026-05-30 — sandboxed tool documentation + copy/move tools)

The sandboxed steroid-Hermes tool set has been documented and extended:

- Documentation now covers the full `tool` command family, `tool govern`, all sandboxed demo tools, and the cognitive observation pipeline.
- `copy_file` was added as a workspace-bounded, simulation-first copy operation with absolute-path, parent-traversal, sensitive-directory, overwrite and size guards.
- `move_file` / `rename` was added as a workspace-bounded, simulation-first move/rename operation with the same security boundaries.
- Code touched: `crates/tool-runtime/src/lib.rs`, `crates/cli/src/main.rs`.
- Docs touched: `docs/cli.md`, `docs/cognitive-tool-runtime.md`.
- Verification reported by PRs: `cargo fmt -- --check`, `cargo check`, `cargo test --workspace` green.

## Next action

Build the **First Useful Actor Lab**: one end-to-end governed local mission showing:

`user task -> proposed sandboxed file action -> simulation/diff -> explicit approval path -> execution -> audit/observation trace -> CLI readback`

Keep it small and demonstrable. Prefer one local workspace-file scenario such as appending a note or creating/updating a small markdown file. The goal is not another isolated tool; it is proof that the governed Hermes-like loop is useful.

Do **not** add unrestricted shell, browser, network, secrets access, file deletion, scheduler autonomy, or hidden autonomy.
