# Arpagona MCP Server — Native Model Context Protocol Server

## Overview

The `arpagona-mcp-server` crate implements a **native MCP (Model Context Protocol) server** in Rust. External AI agents (Claude Desktop, Cursor, VS Code, Cline, Roo Code, etc.) can connect via **stdio** or **HTTP/SSE transport** to discover and call Arpagona's governed tools, resources, and prompts.

MCP is an open protocol (initiated by Anthropic, 2025-11-25) that standardises how AI applications expose tools, resources and prompts to LLM clients.

## Feature Matrix

| Feature | Phase | Status |
|---|---|---|
| Transport stdio (newline-delimited JSON-RPC 2.0) | A1 | ✅ |
| `initialize` handshake with capabilities | A1 | ✅ |
| `tools/list` — 3 read-only tools | A1 | ✅ |
| `tools/call` — execute tool via ToolRuntime | A1 | ✅ |
| DecisionGate governance before `tools/call` | A2 | ✅ |
| Governance audit store (JSON-lines persistence) | A2 | ✅ |
| HTTP/SSE transport via Axum endpoint `/mcp` | A3 | ✅ |
| Resources (`resources/list`, `resources/read`, `resources/templates/list`) | A4 | ✅ |
| Prompts (`prompts/list`, `prompts/get`) | A4 | ✅ |
| Notifications (`notifications/tools/list_changed`) | A5 | ✅ |
| Protocol hardening and correctness tests | A5 | ✅ |
| Operator documentation and usage examples | A6 | 🔜 |

## Architecture

```
                    ┌──────────────────────────────────┐
                    │   AI Client                       │
                    │  (Claude Desktop, Cursor,         │
                    │   Cline, Roo Code, etc.)          │
                    └─────┬──────────────┬──────────────┘
                          │ stdio         │ HTTP/SSE (Axum :3001/mcp)
                          ▼               ▼
                    ┌──────────────────────────────────┐
                    │  arpagona mcp-server               │
                    │  ┌────────────────────────────┐   │
                    │  │ Transport Layer              │   │
                    │  │ - stdio (server.rs)          │   │
                    │  │ - HTTP/SSE (http_transport.rs)│   │
                    │  └──────────┬─────────────────┘   │
                    │             ▼                      │
                    │  ┌────────────────────────────┐   │
                    │  │ Protocol Dispatch            │   │
                    │  │ - initialize                 │   │
                    │  │ - tools/list / tools/call    │   │
                    │  │ - resources/list/read        │   │
                    │  │ - prompts/list/get            │   │
                    │  │ - notifications/*             │   │
                    │  └──────────┬─────────────────┘   │
                    │             │                      │
                    │  ┌──────────▼─────────────────┐   │
                    │  │ Governance Layer             │   │
                    │  │ - DecisionGate evaluation    │   │
                    │  │ - GovernanceDecision enum    │   │
                    │  │ - Audit store persistence    │   │
                    │  └──────────┬─────────────────┘   │
                    │             │                      │
                    │  ┌──────────▼─────────────────┐   │
                    │  │ Tool Runtime (read-only)     │   │
                    │  │ - read_file                  │   │
                    │  │ - list_files                 │   │
                    │  │ - search_text                │   │
                    │  └────────────────────────────┘   │
                    └──────────────────────────────────┘
```

## Tools Exposed

| MCP Tool | Description | Input Schema |
|---|---|---|
| `read_file` | Read a file within the workspace | `{ "path": "..." }` |
| `list_files` | List files and directories in a path | `{ "path": "..." }` (optional) |
| `search_text` | Search for text patterns in files | `{ "query": "...", "path": "..." }` |

All tools are **read-only** with strict security constraints:
- No absolute paths
- No parent traversal (`../`)
- No sensitive files (`.env`, `.ssh`, etc.)
- Size and result limits enforced by Tool Runtime

## Governance (Phase 2)

Every `tools/call` invocation passes through the DecisionGate before execution:

```
ToolCall -> evaluate_tool_call() -> DecisionGate -> Decision -> Audit -> execute/reject
```

The governance layer produces one of:
- **Approved** — tool is read-only and the client has the required permission
- **Blocked** — tool is not read-only or permission is missing (not overridable)
- **RequiresOverride** — permission is missing but the action type supports override
- **ApprovedByOverride** — override password matched

All governance decisions are recorded in the audit store and persisted as JSON-lines files.

### Override mechanism

Sensitive operations (non-read-only tools) can be overridden using the `ARPAGONA_OVERRIDE_PASSWORD_HASH` environment variable. The `--override-password` CLI flag or MCP initialization option provides the password.

**Important:** `ProposeToolUse` actions are classified as `NotOverridable` by the override engine. Only explicitly governed action types can be overridden.

## Resources (Phase 4)

The server exposes MCP Resources for inspectable system state:

### Static resources

| URI | Description |
|---|---|
| `arpagona://server/info` | Server name, version, protocol compliance |
| `arpagona://tools/list` | Current tool catalogue with descriptions |
| `arpagona://governance/summary` | Governance decision summary (recent counts by status) |
| `arpagona://audit/recent` | Most recent audit records (last 10) |

### Resource templates

| URI Template | Description |
|---|---|
| `arpagona://audit/{index}` | Individual audit record by index |

Resource annotations mark all resources as `read-only` with `server` priority.

## Prompts (Phase 4)

The server exposes MCP Prompts for structured interaction with external agents:

| Prompt Name | Description | Arguments |
|---|---|---|
| `summarize-context` | Summarise the current workspace and tool state for an incoming agent | `context: string` (optional) |
| `assess-governance` | Assess governance state: decision counts, status distribution, recent audit trail | none |
| `suggest-next-steps` | Suggest next cognitive steps based on the current state | `objective: string` |

Each prompt returns structured `PromptMessage` content with role `"assistant"`.

## Notifications (Phase 5)

The server broadcasts MCP notifications when state changes:

| Notification | Trigger | Payload |
|---|---|---|
| `notifications/tools/list_changed` | Server startup (to inform reconnecting clients) | `{ "method": "notifications/tools/list_changed" }` |

The notification channel uses `tokio::sync::broadcast` and supports multiple concurrent receivers. HTTP/SSE clients receive notifications as SSE events. Stdio clients receive them as JSON-RPC 2.0 notifications on the read stream.

## Transports

### Stdio transport (default, Phase 1)

The server reads newline-delimited JSON-RPC 2.0 requests from stdin and writes responses to stdout. Stderr is reserved for diagnostics.

```bash
arpagona mcp-server
```

### HTTP/SSE transport (Phase 3)

The server exposes an Axum-based HTTP endpoint at port 3001. SSE (Server-Sent Events) provides real-time notification streaming.

Start with:
```bash
arpagona mcp-server --http
# or
cargo run -p arpagona-cli -- mcp-server --http
```

The HTTP endpoint at `/mcp` handles JSON-RPC requests via POST. The SSE endpoint at `/mcp/sse` streams notifications.

## Usage

### From the CLI

```bash
# Start the MCP server (stdio transport, waits for client)
arpagona mcp-server

# With custom workspace and server identity
arpagona mcp-server --workspace /path/to/project --name "my-arpagona" --version "1.0.0"

# With HTTP/SSE transport
arpagona mcp-server --http

# With audit logging
arpagona mcp-server --audit-path /tmp/arpagona-audit.jsonl
```

### From MCP Clients

**Claude Desktop** (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "arpagona": {
      "command": "arpagona",
      "args": ["mcp-server", "--workspace", "/path/to/project"],
      "env": {}
    }
  }
}
```

**VS Code with Cline/Roo Code**:
```json
{
  "mcpServers": {
    "arpagona": {
      "command": "arpagona",
      "args": ["mcp-server"]
    }
  }
}
```

### HTTP/SSE client example

```bash
# Connect to SSE stream
curl -N http://localhost:3001/mcp/sse

# In another terminal, send a JSON-RPC request
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }'
```

## Protocol

The server implements a compliant subset of the MCP specification (2025-11-25).

### initialize

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": {},
      "resources": {},
      "prompts": {},
      "notifications": {}
    },
    "serverInfo": {
      "name": "arpagona-mcp",
      "version": "0.1.0"
    }
  }
}
```

### tools/list

Returns the 3 read-only tools with JSON Schema input definitions and annotations (`readOnlyHint: true`).

### tools/call

Executes a tool via the Tool Runtime with DecisionGate governance. Returns:

- **Approved (success)**: `content` with `[{type: "text", text: "..."}]` and `isError: false`
- **Approved (error)**: `content` with `[{type: "text", text: "Error: ..."}]` and `isError: true`
- **Blocked**: `isError: true` with descriptive message including the governance decision summary

### Error codes

| Error | Code | Description |
|---|---|---|
| Unknown method | -32601 | Method not recognised before initialization |
| Invalid params | -32602 | Malformed input parameters |
| Internal error | -32603 | Tool execution or governance failure |

## Code Structure

```
crates/mcp-server/
├── Cargo.toml
└── src/
    ├── lib.rs              — Public API, re-exports
    ├── types.rs            — JSON-RPC 2.0 + MCP protocol types (52+ types)
    ├── transport.rs        — Stdio transport (read/write NDJSON)
    ├── server.rs           — Server lifecycle, dispatch, handlers, notifications
    ├── http_transport.rs   — Axum HTTP/SSE transport (routes, SSE broadcast)
    ├── audit_store.rs      — Governance audit store (in-memory + JSON-lines persistence)
    └── governance.rs       — Tool call evaluation (DecisionGate integration)
```

## Security Model

1. **All tools are read-only** — no write, delete, shell, or network tools exposed via MCP
2. **Every tool call is governed** — `tools/call` always passes through DecisionGate
3. **Path restrictions** — absolute paths and parent traversal blocked at runtime
4. **Sensitive file blocks** — `.env`, `.ssh`, `.git/config` and similar blocked at runtime
5. **Audit trail** — every governance decision persisted in audit store
6. **No secrets in MCP responses** — governance decisions do not leak override passwords or internal tokens
7. **Initialisation required** — all method calls before `initialize` return errors

## Testing

```bash
# All MCP server tests (52 unit tests across all modules)
cargo test -p arpagona-mcp-server

# Full workspace (must pass before merging any MCP PR)
cargo test --workspace
```

Test coverage includes:
- Stdio transport serialisation roundtrips
- HTTP transport lifecycle (init → tools/list → tools/call)
- SSE notification broadcast and receipt
- Governance decision outcomes (approved, blocked, override)
- Audit store persistence across restart
- Resource and prompt list/read handlers
- Notification format and broadcast correctness
- Error handling for uninitialised clients and unknown methods

## Detailed Phase History

| Phase | PR | What |
|---|---|---|
| A1 | #105 | Native MCP crate, stdio transport, tools/list, tools/call |
| A2 | #107, #110 | DecisionGate governance for tools/call, audit store |
| A3 | #112 | Axum HTTP/SSE transport at `/mcp` |
| A4 | merged | Resources (server info, tool catalogue, audit summaries) + Prompts (summarize, assess, suggest) |
| A5 | merged | Notifications (tools/list_changed), protocol hardening |
| A6 | 🔜 | Operator documentation, examples, client smoke tests |

## Related Documentation

- [`docs/cli.md`](cli.md) — CLI usage including `arpagona mcp-server` command
- [`docs/governance.md`](governance.md) — Decision Gate architecture and override mechanism
- [`docs/tool-registry.md`](tool-registry.md) — Declarative tool catalogue
- [`docs/daily-agent-validation.md`](daily-agent-validation.md) — Daily validation including MCP smoke tests
