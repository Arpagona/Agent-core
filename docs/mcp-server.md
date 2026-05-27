# Arpagona MCP Server — Native Model Context Protocol Server

## Overview

The `arpagona-mcp-server` crate implements a **native MCP (Model Context Protocol) server** in Rust. External AI agents (Claude Desktop, Cursor, VS Code, Cline, etc.) can connect via the **stdio transport** to discover and call Arpagona's tools.

MCP is an open protocol (initiated by Anthropic) that standardises how AI applications expose tools, resources and prompts to LLM clients.

## Architecture

```
                    ┌─────────────────────────────┐
                    │   AI Client                  │
                    │  (Claude Desktop, Cursor,    │
                    │   VS Code, Cline, etc.)      │
                    └──────────┬──────────────────┘
                               │ JSON-RPC 2.0 over stdio
                               ▼
                    ┌─────────────────────────────┐
                    │  arpagona mcp-server          │
                    │  ┌───────────────────────┐   │
                    │  │ Transport (stdio)      │   │
                    │  └──────────┬────────────┘   │
                    │             ▼                 │
                    │  ┌───────────────────────┐   │
                    │  │ Protocol Dispatch      │   │
                    │  │ - initialize           │   │
                    │  │ - tools/list           │   │
                    │  │ - tools/call           │   │
                    │  └──────────┬────────────┘   │
                    │             ▼                 │
                    │  ┌───────────────────────┐   │
                    │  │ Tool Runtime           │   │
                    │  │ (read-only)            │   │
                    │  └───────────────────────┘   │
                    └─────────────────────────────┘
```

## Phase 1 Features

| Feature | Status |
|---|---|
| Transport stdio (newline-delimited JSON-RPC 2.0) | ✅ |
| `initialize` handshake with capabilities | ✅ |
| `tools/list` — 3 read-only tools | ✅ |
| `tools/call` — execute tool via ToolRuntime | ✅ |
| Governance via DecisionGate | 🔜 Phase 2 |
| HTTP/SSE transport | 🔜 Phase 2+ |
| Resources (snapshots, audit events) | 🔜 Phase 3 |
| Prompts (cognitive work loop templates) | 🔜 Phase 3 |

## Tools Exposed

| MCP Tool | Description | Input |
|---|---|---|
| `read_file` | Read a file within the workspace | `{ "path": "..." }` |
| `list_files` | List files and directories in a path | `{ "path": "..." }` (optional) |
| `search_text` | Search for text patterns in files | `{ "query": "...", "path": "..." }` |

All tools are **read-only** with strict security constraints:
- No absolute paths
- No parent traversal
- No sensitive files (`.env`, `.ssh`, etc.)
- Size and result limits

## Usage

### From the CLI

```bash
# Start the MCP server (waits for client connection on stdin)
arpagona mcp-server

# With custom workspace
arpagona mcp-server --workspace /path/to/project

# With custom server name
arpagona mcp-server --name "my-arpagona" --version "1.0.0"
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

## Protocol

The server implements a minimal but compliant subset of the MCP specification (2025-11-25):

### initialize

Server returns:
```json
{
  "protocol_version": "2025-11-25",
  "capabilities": {
    "tools": {}
  },
  "server_info": {
    "name": "arpagona-mcp",
    "version": "0.1.0"
  }
}
```

### tools/list

Returns the 3 read-only tools with JSON Schema input definitions.

### tools/call

Executes a tool via the Tool Runtime and returns:
- On success: `content` with `[{type: "text", text: "..."}, {type: "json", json: {...}}]`
- On error: `content` with `[{type: "text", text: "Error: ..."}]` and `isError: true`

## Code Structure

```
crates/mcp-server/
├── Cargo.toml
└── src/
    ├── lib.rs        — Public API, re-exports
    ├── types.rs      — JSON-RPC 2.0 + MCP protocol types
    ├── transport.rs  — stdio transport (read/write NDJSON)
    └── server.rs     — Server lifecycle, dispatch, handlers
```

## Testing

```bash
# Unit tests for types, transport, server handlers
cargo test -p arpagona-mcp-server

# Full workspace tests (all existing tests must pass)
cargo test --workspace
```

## Future Phases

| Phase | What |
|---|---|
| **2** | Governance (DecisionGate before tools/call), dynamic tool discovery from Tool Registry |
| **3** | HTTP/SSE transport via Axum endpoint `/mcp` |
| **4** | Resources (snapshots, audit events as MCP resources) |
| **5** | Prompts (cognitive work loop templates) |
| **6** | Tool change notifications (`notifications/tools/list_changed`) |
