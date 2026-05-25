# Cognitive Tool Runtime — ARPAGONA Agent Core

> **Status**: Alpha — experimental read-only tool runtime for perception and inspection.
> **Scope**: Local, bounded, read-only execution only.
> **Safety**: No shell, no write, no network, no external effects.

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

These are the first channels of perception. They make the cognitive loop more than a thought experiment.

## Why execution must be controlled

Tools are powerful. An agent that can read files can also read secrets. An agent that can list directories can discover sensitive paths. An agent that can search text can extract patterns it should not see.

The Tool Runtime enforces strict, immutable boundaries:

- **No absolute paths** — all paths must be relative to the workspace
- **No parent traversal** — `..` that escapes the workspace is blocked
- **No sensitive files** — `.env`, `.ssh/`, key files are always blocked
- **No dangerous directories** — `.git`, `target`, `node_modules`, `.env`, `.ssh` are always skipped
- **Size limits** — files larger than 1 MiB (read) or 500 KiB (search) are rejected
- **Result limits** — list caps at 200 entries, search caps at 100 matches

These constraints make the runtime safe for alpha experimentation. They can be relaxed later under governance, never silently.

## Architecture overview

```
┌─────────────────────────────────────────────────────────────┐
│                  Cognitive Loop                             │
│                                                             │
│  Objective → Working Memory → Tool Intent →                │
│  Tool Registry → ProposedAction → Decision Gate →          │
│  ┌──────────────────────────────────────────────────┐       │
│  │  Tool Runtime (YOU ARE HERE)                     │       │
│  │  ┌──────────┐  ┌───────────┐  ┌────────────┐   │       │
│  │  │read_file │  │list_files │  │search_text │   │       │
│  │  └──────────┘  └───────────┘  └────────────┘   │       │
│  └──────────────────────────────────────────────────┘       │
│  Observation → Audit → Reflection / FailureInsight          │
└─────────────────────────────────────────────────────────────┘
```

### Component roles

| Component | Role |
|-----------|------|
| **Tool Intent** | What tool, why, what risk, what fallback |
| **Tool Registry** | Declarative catalogue of available tools |
| **ProposedAction** | Intention + Governance metadata |
| **Decision Gate** | May this tool be used for this purpose? |
| **Tool Runtime** | Execute the allowed tool within bounds |
| **Observation** | Structured result of execution |
| **Reflection** | Learn from success or failure |

## The three read-only tools

### `read_file`

- **Cognitive role**: Perception, Inspection
- **Arguments**: `path` (required)
- **Returns**: File content preview (first 500 chars), line count, character count
- **Security**: Blocks absolute paths, parent traversal, sensitive files, large files

### `list_files`

- **Cognitive role**: Perception
- **Arguments**: `path` (optional, default `.`)
- **Returns**: List of entries with name, path, is_directory
- **Security**: Skips `.git`, `target`, `node_modules`, `.env`, `.ssh`; max depth 5; max 200 results

### `search_text`

- **Cognitive role**: Inspection
- **Arguments**: `query` (required), `path` (optional, default `.`)
- **Returns**: Matching results with file path, line number, snippet
- **Security**: Same dir exclusions; max file size 500 KiB; max 100 results

## How Hermes inspired the tool selection

Hermes Agent demonstrates that a capable agent needs a rich set of tools: file read/write, shell, web search, browser automation, and more. The key insight from Hermes is that **the agent knows which tool to use** — it has the context, the objective, and the reasoning capacity.

ARPAGONA formalises this into typed, auditable structures:

- `ToolIntent` captures *why* the agent chose this tool
- `ToolExecutionResult` captures *what happened* in machine-readable form
- `ToolObservation` marks whether the result is usable or should become a `FailureInsight`

The current 3 read-only tools mirror Hermes' `read_file`, `search_files`, and `list_files` capabilities — but through a governed, typed, auditable channel.

## Why read-only first

The first heartbeat of the cognitive runtime is deliberately **perception only**:

1. **Safety**: Perception cannot damage the workspace. It can only observe.
2. **Foundation**: You cannot act wisely on an environment you have not perceived.
3. **Learning**: Every successful or failed perception generates an observation that feeds the Failure-to-Insight pipeline.
4. **Trust**: A read-only runtime can be deployed without exposing the host to accidental or malicious side effects.

The cognitive loop must *see* before it can *touch*.

## What remains non-executable

The following cognitive roles exist as concepts but are **not executable** in the current alpha Tool Runtime:

- **Transformation** — write files, rename, modify (needs Decision Gate + permissions)
- **Execution** — run commands, start processes (needs sandboxing)
- **Communication** — send emails, messages, notifications (needs identity + consent)

These roles are declared in the Tool Registry as `is_non_executable = true`.

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
