# ARPAGONA Agent Core — Next Focus Loop Handoff

This file is the short-lived operational handoff from one scheduled focus-loop run to the next.

Every focus-loop run must read this file after the canonical context files, use it as the immediate continuity hint, and update it at the end of the run.

This file does not override safety, governance, `PROJECT_OBJECTIVES.md`, `PROJECT_STATUS.md` or `AGENT_FOCUS_LOOP.md`. It only captures the most concrete next step discovered by the previous run.

## Current next-pass instruction
## Current next-pass instruction

Next pass should: transition the demo snapshot approach to a Cargo feature-gated SurrealDB `kv-surrealkv` backend when the `surrealdb_unstable` cfg flag becomes stable or explore an alternative pure-Rust key-value backend.

Why: the demo snapshot path proves cross-invocation readback for the governed FailureInsight learning loop, but is limited to JSON file I/O. A real persistent backend would remove the snapshot intermediate step and make the persistence path native.

Proof to seek: `cargo test -- cross_invocation` passing (proves the current cross-process demo snapshot path works), plus either a feature-gated SurrealDB-backed test or an alternative pures-Rust KV backend test that also passes the cross-invocation proof.

Do not: add broad autonomous memory writing, provider/runtime direct memory mutation, external effects, scheduler expansion, Mission Control Web, MCP/browser automation, personal/sensitive memory, readback-as-authorization behavior, or always-on native/unstable backend requirements.

Why: the demo snapshot path proves cross-invocation readback for the governed FailureInsight learning loop, but is limited to JSON file I/O. A real persistent backend would remove the snapshot intermediate step and make the persistence path native.

Proof to seek: `cargo test -- cross_invocation` passing (proves the current cross-process demo snapshot path works), plus either a feature-gated SurrealDB-backed test or an alternative pures-Rust KV backend test that also passes the cross-invocation proof.
>>>>>>> origin/main

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
