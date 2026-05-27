//! Arpagona MCP Server — Native Model Context Protocol server in Rust.
//!
//! This crate implements a native MCP (Model Context Protocol) server for
//! ARPAGONA Agent Core. External AI agents (Claude Desktop, Cursor, VS Code,
//! etc.) can connect to discover and call Arpagona's tools.
//!
//! # Phase 1
//!
//! - **Transport:** stdio (newline-delimited JSON-RPC 2.0)
//! - **Handlers:** `initialize`, `tools/list`, `tools/call`
//! - **Tools:** read-only tools from the Tool Registry + Tool Runtime
//!   (read_file, list_files, search_text)
//! - **Governance:** direct execution (governance via DecisionGate deferred
//!   to Phase 2)
//!
//! # Usage
//!
//! ```rust,no_run
//! use arpagona_mcp_server::{McpServer, McpServerConfig};
//!
//! let mut server = McpServer::new(McpServerConfig {
//!     workspace_path: "/path/to/workspace".to_owned(),
//!     ..Default::default()
//! });
//! server.run().unwrap();
//! ```
//!
//! The server reads JSON-RPC requests from stdin and writes responses to
//! stdout, making it compatible with any MCP client that supports the
//! stdio transport.

pub mod governance;
pub mod server;
pub mod transport;
pub mod types;

pub use server::{McpServer, McpServerConfig};
pub use types::*;
