# ARPAGONA sandboxed tool-runtime ambition

Status: active direction after Thibaud's correction: ARPAGONA Agent Core must stop giving powerless, generic answers and must be able to act inside a governed sandbox.

## Product goal

ARPAGONA should feel closer to Hermes/OpenClaw: the agent can inspect, edit, run safe local workflows, remember, and report evidence — while remaining alpha-safe and governance-first.

The target is not "LLM advice". The target is:

```text
intent → tool plan → decision gate → sandboxed execution → observation → audit → next action
```

## Non-negotiable safety doctrine

- No hidden autonomy.
- No unrestricted shell.
- No writes outside the workspace.
- No secrets exposed to the LLM.
- External effects still require explicit human approval.
- Mutating tools must simulate by default unless execution is explicit and governed.
- Every execution result must be structured enough for audit and failure-to-insight.

## Tool tiers

### Tier 0 — Perception, already present

- `read_file`
- `list_files`
- `search_text`

These are read-only and workspace-bounded.

### Tier 1 — Local sandboxed mutation, first slice implemented

- `write_file`

Properties:

- workspace-relative path only;
- absolute paths blocked;
- parent traversal blocked;
- sensitive files/dirs blocked;
- 256 KiB content cap;
- `simulate=true` by default;
- parent directory creation requires explicit flag;
- overwrites require explicit flag.

CLI demo:

```bash
arpagona tool demo write-file arpagona-sandbox-demo.txt "hello" --execute
```

### Tier 2 — Patch/edit tools, next

Needed for real coding/workflow value:

- `patch_file` / `replace_text`
- `append_file`
- `mkdir`
- possibly `delete_file`, but only in a trash/quarantine mode first

Safety:

- simulate-first diffs;
- exact-match edits by default;
- size caps;
- no `.git`, `.env`, `target`, `node_modules`, `.ssh` mutation;
- audit snapshot of before/after metadata.

### Tier 3 — Command execution, later and gated

Needed to approach OpenClaw/Hermes capability:

- `run_command` with allowlisted commands only;
- fixed working directory inside workspace;
- timeout;
- stdout/stderr caps;
- no shell expansion by default;
- no network commands until separate permission exists;
- no package install commands without human approval.

Example allowlist candidates:

- `cargo test`, `cargo check`, `cargo fmt`
- `python -m pytest`
- `npm test` / `pnpm test`
- project-defined scripts from a trusted manifest

### Tier 4 — Network/web/browser, not first

These are high value but easy to abuse. They should come after local sandbox maturity:

- HTTP GET/read-only fetch;
- web search;
- browser automation;
- external write/send actions only behind explicit approval and identity boundaries.

## First implementation slice completed

The tool runtime now includes a sandboxed `write_file` primitive. It is deliberately small but real: ARPAGONA can create/write a workspace file when execution is explicit.

Validation command:

```bash
cargo test -p arpagona-tool-runtime
```

Result observed: 41 tests passed.

## Next acceptance criteria

1. CLI exposes `write_file` in `tool list`, `tool inspect write_file`, and `tool demo write-file`.
2. Decision Gate explicitly treats mutation tools as higher risk than perception tools.
3. Chat/Ollama can propose a tool plan that uses local tools instead of answering generically.
4. `run` can perform a bounded observe/edit/validate loop in sandbox mode.
5. Audit spine records every executed tool result.
