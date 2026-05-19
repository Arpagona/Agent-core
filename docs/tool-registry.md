# Tool Registry

`crates/tool-registry` is the alpha minimal declarative catalogue for ARPAGONA Agent Core tools.

It exists to describe tools before any real tool execution is allowed. It does not execute tools, call providers, access the filesystem, access secrets, open shells, drive browsers, run schedulers, integrate MCP, or create autonomous runtime behavior.

## Role in the governed flow

The Tool Registry belongs before the Decision Gate in the future controlled execution path:

```text
ProposedAction -> ToolRegistry lookup -> DecisionGate -> Human approval if needed -> Controlled execution -> Audit -> Graph update
```

A registry lookup is only descriptive. It can answer whether a tool declaration exists, what permissions and risks are declared, and whether the declaration is enabled, disabled or deprecated. It does not approve execution.

## Current alpha surface

The crate currently exposes:

- `ToolId` through the core domain type;
- `ToolDefinition` through the core domain type;
- `ToolPermission` as a registry-local alias for `Permission`;
- `ToolRiskLevel` as a registry-local alias for `RiskLevel`;
- `ToolCapability` for declarative capabilities;
- `ToolKind` for broad tool categories;
- `ToolSchema` for JSON-compatible input/output schema declarations;
- `ToolGovernance` for human-readable governance metadata;
- `RegisteredTool` for catalogue entries;
- `ToolRegistry` for an in-memory declaration catalogue;
- `ToolLookup` and `ToolLookupStatus` for descriptive lookup results;
- `ToolRegistryError` for duplicate or missing catalogue entries.

Supported behavior:

- register a tool declaration;
- reject duplicate tool identifiers;
- check whether a declaration exists;
- look up a declaration by id;
- list enabled declarations;
- disable a declaration.

## Explicit non-goals

The alpha Tool Registry must not implement:

- real tool execution;
- shell access;
- provider calls;
- filesystem modification tools;
- browser automation;
- MCP integration;
- scheduler behavior;
- autonomous loops;
- secrets access;
- authorization decisions;
- Decision Gate replacement;
- Compute Reservoir replacement;
- Audit replacement.

## Boundary with other components

- Decision Gate decides whether a proposed action is allowed, blocked, rerouted, or requires human validation.
- Tool Registry only declares what tools exist and what their static governance metadata says.
- Compute Reservoir chooses how to think or process a task. It does not grant execution rights.
- Audit records important proposals, decisions, approvals, failures and future controlled execution outcomes.
- Graph Memory stores durable context and traces. It is not an execution layer.

## Stability

Status: alpha minimal.

The crate is useful as a governance-first foundation, but its schema conventions and integration points are not yet stable. It should remain pure Rust and dependency-light until Tool Registry, Decision Gate and Audit are jointly stabilized.

Recommended next step: keep hardening declarative semantics and tests before wiring the registry into API, CLI or runtime surfaces.
