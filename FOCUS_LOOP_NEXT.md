# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction

Next pass should: add an opt-in local Graph Memory persistence feature or demo snapshot path that leaves plain `cargo check`/`cargo test` green by default.

Why: direct always-on SurrealDB persistent backends are currently blocked: `kv-surrealkv` needs `surrealdb_unstable`, and `kv-rocksdb`/`File` failed local verification on native `zstd-sys`/clang headers.

Proof to seek: `cargo fmt -- --check && cargo check && cargo test`, plus an explicit feature-gated or demo command/test proving FailureInsight readback across a separate persistence/readback step.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, MCP/browser automation, personal/sensitive memory, readback-as-authorization behavior, or always-on native/unstable backend requirements.

## Required update at the end of every run

Replace the instruction above with a concrete next-pass instruction in this shape:

```text
Next pass should: <one concrete action>.
Why: <one sentence explaining the blocker or opportunity>.
Proof to seek: <exact command, test, readback or file that should confirm progress>.
Do not: <specific unsafe or distracting thing to avoid next time>.
```

Keep it short, specific and executable. Do not write a vague roadmap.
