# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived handoff for the next supervised work run.

It must contain one concrete next action only. The runtime milestone queue and long-term rules live in `AGENT_FOCUS_LOOP.md`.

## Current status (GONA 2026-05-30 — merge queue clear, actor-lab next)

The sandboxed steroid-Hermes queue is clear:

- PR #229 merged: sandboxed tool documentation for the 7-tool set, `tool govern`, and `tool demo observe`.
- PR #230 merged: `copy_file` and `move_file` / `rename` added as workspace-bounded, simulation-first tools.
- The active sandboxed set is now focused on useful local workspace action without shell/browser/network/secrets/file deletion.

## Next action

Build the **First Useful Actor Lab**: one end-to-end governed local mission showing:

`user task -> proposed sandboxed file action -> simulation/diff -> explicit approval path -> execution -> audit/observation trace -> CLI readback`

Keep it small and demonstrable. Prefer one local workspace-file scenario such as appending a note or creating/updating a small markdown file. The goal is not another isolated tool; it is proof that the governed Hermes-like loop is useful.

Do **not** add unrestricted shell, browser, network, secrets access, file deletion, scheduler autonomy, or hidden autonomy.
