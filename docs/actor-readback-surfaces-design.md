# ARPAGONA Actor Readback Surfaces — Command Design Spec

> Prepared by DEEP per GONA direction (2026-05-30).
> GO for implementation granted by GONA.
> Implemented on branch: `feat/actor-readback-surfaces`
> PR opened; awaiting merge.
>
> **Decision Gate wording (for Thibaud approval):**
> - GO/NO-GO requested: implement three read-only inspection surfaces under `arpagona actor` — `status`, `memory`, `journal`.
> - Allowed: reading local agent config/env, reading the in-memory LLM journal, reading graph memory configuration, human-readable + JSON output, NON_AUTHORIZING_READBACK warnings, `--interaction-type` filter for journal, `--limit` for journal.
> - Forbidden: no mutation paths, no new external providers, no autonomy escalation, no Decision Gate bypass, no secrets exposure, no file writes, no API server queries for readback.

## 1. Objective

Add three read-only inspection surfaces under the `arpagona actor` command that expose actor memory, status, and LLM journal state. These are the next approved roadmap brick (per GONA direction), providing operator readback capability without expanding the actor's mutation surface.

## 2. Command Interface

```text
arpagona actor status          # Show read-only actor status readback
  --json                       # Structured JSON output

arpagona actor memory          # Show read-only actor memory readback
  --json                       # Structured JSON output

arpagona actor journal         # Show read-only actor journal readback
  --limit <N>                  # Max entries to show (default: 10)
  --interaction-type <type>    # Filter by interaction type
  --json                       # Structured JSON output
```

### Exit codes

Commands always exit 0 on success. Errors (e.g., lock contention on the journal mutex) are propagated as `Err` to the CLI framework.

## 3. Subcommand Details

### `arpagona actor status`

Read-only summary of the actor's current configuration and journal state.

**Output fields (text mode):**
- `agent_id`: resolved from `ARPAGONA_AGENT_ID` env or default (`agent-alpha`)
- `agent_kind`: resolved from `ARPAGONA_AGENT_KIND` env or `"unknown"`
- `workspace_id`: resolved from `ARPAGONA_WORKSPACE_ID` env or default (`workspace-alpha`)
- `api_url`: resolved from `ARPAGONA_API_URL` env or default (`http://127.0.0.1:3000`)
- `total_entries`: count of all LLM journal entries
- `direct_tool_calls`: count of DirectToolCall entries (actor-run interactions)
- `governance_entries`: count of entries with Decision Gate outcomes

**Design constraints:**
- Pure readback — reads env vars and journal state, no mutations
- No API server queries — fully offline
- Includes `NON_AUTHORIZING_READBACK` warning

### `arpagona actor memory`

Read-only inspection of the actor's graph memory state.

**Output fields (text mode):**
- `graph_memory_support_compiled`: always `true` (core crate always compiled)
- `configured_backend`: from `ARPAGONA_GRAPH_MEMORY_BACKEND` env or `"not configured"`
- `memory_active`: whether the backend is set to `surrealdb`
- Alpha limits listing: what's available vs not implemented
- Access methods: pointers to existing commands for deeper inspection

**Design constraints:**
- No mutation of facts, episodes, or observations
- No API server queries — fully offline
- Includes `NON_AUTHORIZING_READBACK` warning

### `arpagona actor journal`

Read-only LLM journal readback from the actor's in-memory ring buffer, filtered to actor-run interactions by default.

**Filtering behavior:**
- Default (no `--interaction-type`): shows only `DirectToolCall` entries with `objective = "actor_run"` — the actor-specific journal trail
- With `--interaction-type`: filters by partial match on interaction type string (`synthesis`, `tool_call_intent`, `direct_tool_call`)
- With `--interaction-type governance`: shows entries with Decision Gate outcomes (proposed_actions + decision_gate_outcomes)

**Output fields per entry (text mode):**
- Index, interaction type, timestamp
- Provider, model, objective preview
- Prompt summary (truncated to 120 chars)
- Response summary (truncated to 120 chars)
- Decision Gate outcome JSON (if present)
- Risk level (if present)

**Design constraints:**
- Read-only — locks the journal for reading but does not write
- No API server — reads global in-memory journal
- Includes `NON_AUTHORIZING_READBACK` warning

## 4. File Changes

### `crates/cli/src/main.rs`

- **ActorSubcommand enum** (line ~165): Added `Status(ActorStatusArgs)`, `Memory(ActorMemoryArgs)`, `Journal(ActorJournalArgs)` variants
- **Args structs** (line ~237): Added `ActorStatusArgs`, `ActorMemoryArgs`, `ActorJournalArgs` with `--json`, `--limit`, `--interaction-type`
- **Dispatch** (line ~2345): Added 3 new match arms calling `actor_status_readback`, `actor_memory_readback`, `actor_journal_readback`
- **Functions** (line ~11755): Added 3 new functions:
  - `actor_status_readback(args)` — 87 lines
  - `actor_memory_readback(args)` — 72 lines
  - `actor_journal_readback(args)` — 87 lines
- **Tests** (line ~17098): Added 9 new parse tests for all new subcommands

### New file: `docs/actor-readback-surfaces-design.md`

This design document.

## 5. Architecture

```text
User CLI input
  │
  ├─ arpagona actor status
  │   └─ actor_status_readback()
  │       ├─ Read envvars (agent_id, kind, workspace, api_url)
  │       ├─ Read global_llm_journal() (read-only lock)
  │       └─ Print text/JSON with NON_AUTHORIZING_READBACK
  │
  ├─ arpagona actor memory
  │   └─ actor_memory_readback()
  │       ├─ Read envvar (ARPAGONA_GRAPH_MEMORY_BACKEND)
  │       └─ Print text/JSON with alpha limits + NON_AUTHORIZING_READBACK
  │
  └─ arpagona actor journal
      └─ actor_journal_readback()
          ├─ Read global_llm_journal() (read-only lock)
          ├─ Filter by interaction_type or default to actor-run
          └─ Print text/JSON with NON_AUTHORIZING_READBACK
```

## 6. Capabilities Added

| Capability | Existing | New |
|---|---|---|
| Actor status inspection | `arpagona status` (system-wide) | `arpagona actor status` (actor-focused) |
| Actor memory overview | `arpagona memory status` (graph-memory focused) | `arpagona actor memory` (actor-focused) |
| Actor journal readback | `arpagona llm journal` (all entries) | `arpagona actor journal` (actor-run filtered) |
| Structured JSON output | Via `--json` on existing commands | Via `--json` on all new commands |
| Interaction type filter | `arpagona action supervise --interaction-type` | `--interaction-type` on journal readback |
| NON_AUTHORIZING_READBACK | On memory, audit, status commands | On all 3 new commands |

## 7. Capabilities NOT Added

- No mutation paths (no write/gate/approve paths)
- No new external providers
- No API server queries for readback data
- No autonomy escalation
- No Decision Gate bypass
- No secrets exposure
- No file writes
- No multi-actor scope isolation
- No persistent graph memory queries

## 8. Tests

### Parse tests (9 total)

| Test | Coverage |
|---|---|
| `actor_status_parses_with_defaults` | Default parse: `arpagona actor status` |
| `actor_status_parses_with_json` | JSON flag: `--json` |
| `actor_memory_parses_with_defaults` | Default parse: `arpagona actor memory` |
| `actor_memory_parses_with_json` | JSON flag: `--json` |
| `actor_journal_parses_with_defaults` | Default: limit=10, no filter, no json |
| `actor_journal_parses_with_limit` | Custom limit: `--limit 5` |
| `actor_journal_parses_with_interaction_type` | Type filter: `--interaction-type direct_tool_call` |
| `actor_journal_parses_with_json` | JSON flag: `--json` |
| `actor_journal_parses_with_all_options` | All options combined |
