# Cognitive Tool Runtime — ARPAGONA Agent Core

> **Status**: Alpha — experimental tool runtime with read-only perception and sandboxed mutation.
> **Scope**: Local, bounded, simulation-first for mutations.
> **Safety**: No shell, no network, no browser, no secrets access, no external effects. Mutations require explicit `--execute` flag.

## Why tools are necessary for cognition

An agent without perception is an agent without a world. Before ARPAGONA can act on its environment, it must be able to **perceive** it. The Tool Runtime is the first bridge between the agent's abstract intentions and the concrete filesystem reality it operates in.

Without tools, an agent is limited to:
- Reasoning about what it already knows (pure memory recall)
- Proposing actions it cannot execute (abstract governance)
- Generating text that cannot be grounded in actual file state

With the Tool Runtime, the agent can:
- **Read** files to understand context
- **List** directories to discover structure
- **Search** text to find relevant content
- **Write** files to save bounded output (sandboxed, simulation-first)
- **Patch** files for targeted text replacement (sandboxed, simulation-first)
- **Append** to files for bounded log-style updates (sandboxed, simulation-first)
- **Create directories** for organised workspace output (sandboxed, simulation-first)

These are the channels of perception and bounded action. They make the cognitive loop more than a thought experiment.

## Why execution must be controlled

Tools are powerful. An agent that can read files can also read secrets. An agent that can write files could overwrite critical configuration.

The Tool Runtime enforces strict, immutable boundaries:

- **No absolute paths** — all paths must be relative to the workspace
- **No parent traversal** — `..` that escapes the workspace is blocked
- **No sensitive files** — `.env`, `.ssh/`, key files are always blocked
- **No dangerous directories** — `.git`, `target`, `node_modules`, `.env`, `.ssh` are always skipped
- **Size limits** — files larger than 1 MiB (read) or 500 KiB (search) are rejected; writes capped at 256 KiB
- **Result limits** — list caps at 200 entries, search caps at 100 matches
- **Simulation-first** — mutation tools default to simulation mode (no real disk writes); `--execute` required for actual mutation

These constraints make the runtime safe for alpha experimentation. They can be relaxed later under governance, never silently.

## Architecture overview

```
┌──────────────────────────────────────────────────────────────────┐
│                       Cognitive Loop                             │
│                                                                  │
│  Objective → Working Memory → Tool Intent →                     │
│  Tool Registry → Decision Gate →                                │
│  ┌─────────────────────────────────────────────────────┐        │
│  │  Tool Runtime (YOU ARE HERE)                        │        │
│  │                                                      │        │
│  │  Perception:           Mutation (sim-first):         │        │
│  │  ┌──────────┐         ┌───────────┐                │        │
│  │  │read_file │         │write_file │                │        │
│  │  ├──────────┤         ├───────────┤                │        │
│  │  │list_files│         │patch_file │                │        │
│  │  ├──────────┤         ├───────────┤                │        │
│  │  │search_txt│         │append_file│                │        │
│  │  └──────────┘         ├───────────┤                │        │
│  │                       │mkdir      │                │        │
│  │                       └───────────┘                │        │
│  └─────────────────────────────────────────────────────┘        │
│  Observation → Audit → Reflection / FailureInsight              │
└──────────────────────────────────────────────────────────────────┘
```

### Component roles

| Component | Role |
|-----------|------|
| **Tool Intent** | What tool, why, what risk, what fallback |
| **Tool Registry** | Declarative catalogue of available tools |
| **Decision Gate** | May this tool be used for this purpose? |
| **Tool Runtime** | Execute the allowed tool within bounds |
| **Observation** | Structured result of execution |
| **Reflection** | Learn from success or failure |

## The seven tools

### Perception tools (read-only)

#### `read_file`

- **Cognitive role**: Perception, Inspection
- **Arguments**: `path` (required)
- **Returns**: File content preview (first 500 chars), line count, character count
- **Security**: Blocks absolute paths, parent traversal, sensitive files, large files (> 1 MiB)

#### `list_files`

- **Cognitive role**: Perception
- **Arguments**: `path` (optional, default `.`)
- **Returns**: List of entries with name, path, is_directory
- **Security**: Skips `.git`, `target`, `node_modules`, `.env`, `.ssh`; max depth 5; max 200 results

#### `search_text`

- **Cognitive role**: Inspection
- **Arguments**: `query` (required), `path` (optional, default `.`)
- **Returns**: Matching results with file path, line number, snippet
- **Security**: Same dir exclusions; max file size 500 KiB; max 100 results

### Sandboxed mutation tools (simulation-first)

#### `write_file`

- **Cognitive role**: Transformation
- **Arguments**: `path` (required), `content` (required), `simulate` (optional, default `true`)
- **Returns**: Write confirmation with target path and byte count; or simulation preview
- **Security**: Blocks absolute paths, parent traversal, sensitive files; max content 256 KiB; overwrite requires explicit `overwrite: true` flag; default simulation mode

#### `patch_file`

- **Cognitive role**: Transformation
- **Arguments**: `path` (required), `old_string` (required), `new_string` (required), `replace_all` (optional), `simulate` (optional, default `true`)
- **Returns**: Patch confirmation with match count and affected lines; or simulation preview
- **Security**: Same path restrictions as write_file; operations bounded to workspace; default simulation mode

#### `append_file`

- **Cognitive role**: Transformation
- **Arguments**: `path` (required), `content` (required), `create_if_missing` (optional, default `true`), `simulate` (optional, default `true`)
- **Returns**: Append confirmation with final file size; or simulation preview
- **Security**: Same path restrictions; max content 256 KiB; default simulation mode

#### `mkdir`

- **Cognitive role**: Transformation
- **Arguments**: `path` (required), `parents` (optional, default `false`), `simulate` (optional, default `true`)
- **Returns**: Directory creation confirmation; or simulation preview
- **Security**: Blocks absolute paths, parent traversal, dangerous directory names (`.git`, `target`, etc.); default simulation mode

## How Hermes inspired the tool selection

Hermes Agent demonstrates that a capable agent needs a rich set of tools: file read/write, shell, web search, browser automation, and more. The key insight from Hermes is that **the agent knows which tool to use** — it has the context, the objective, and the reasoning capacity.

ARPAGONA formalises this into typed, auditable structures:

- `ToolIntent` captures *why* the agent chose this tool
- `ToolExecutionResult` captures *what happened* in machine-readable form
- `ToolObservation` marks whether the result is usable or should become a `FailureInsight`

The current 7 tools mirror Hermes' `read_file`, `search_files`, `list_files`, and `write_file` capabilities — but through a governed, typed, auditable, simulation-first channel.

## Why read-only → simulation-first

The first heartbeat of the cognitive runtime is deliberately **perception only**, then **bounded simulation-first mutation**:

1. **Safety**: Read-only perception cannot damage the workspace. Mutations default to simulation so no disk writes happen without explicit intent.
2. **Foundation**: You cannot act wisely on an environment you have not perceived.
3. **Learning**: Every successful or failed perception generates an observation that feeds the Failure-to-Insight pipeline.
4. **Trust**: A simulation-first runtime can be deployed without exposing the host to accidental side effects. The `--execute` flag is a human-verifiable gate.

The cognitive loop must *see* before it can *touch*, and must *simulate* before it can *act*.

## What remains non-executable

The following cognitive roles exist as concepts but are **not executable** in the current alpha Tool Runtime:

- **Shell access** — run arbitrary commands, start processes
- **Communication** — send emails, messages, notifications (needs identity + consent)
- **Browser automation** — web interaction
- **Network access** — external API calls
- **Secrets management** — credential/API key access

These capabilities are declared in the Tool Registry with `is_non_executable = true` and must go through future governed design.

## What comes next

The Tool Runtime is designed to be the foundation for:

1. **ToolExecutionResult → Audit** — every execution is an auditable event
2. **ToolExecutionResult → FailureInsight** — failed observations become learning
3. **Working Memory** — observations feed the reservoir/compute state
4. **Graph Memory** — structured observations can become durable facts
5. **Holographic Memory** — execution patterns reinforce or decay
6. **Compute Reservoir** — execution trace echoes

Each of these integrations lives in a future crate or module. The Tool Runtime itself stays focused:

> Tools are not merely utilities. In a cognitive runtime, tools are perception and action channels. They must be explicit, governed, observable and learnable.
