//! HTTP/SSE transport for the MCP server.
//!
//! Provides an Axum-based HTTP server with two endpoints for remote MCP clients:
//!
//! - `POST /mcp` — Receive JSON-RPC 2.0 requests, return JSON-RPC responses
//! - `GET /mcp/sse` — Server-Sent Events stream for server-to-client notifications
//!
//! # Architecture
//!
//! The HTTP transport wraps the synchronous [`McpServer`] behind an `Arc<Mutex<>>`
//! so it can be shared across Axum handler invocations. A `broadcast` channel
//! enables notifications (e.g. `tools/list_changed`) to be pushed to all
//! connected SSE clients.
//!
//! # Usage
//!
//! ```rust,no_run
//! use arpagona_mcp_server::{McpServer, McpServerConfig, http_transport::mcp_router};
//!
//! let server = McpServer::new(McpServerConfig {
//!     workspace_path: "/path/to/workspace".to_owned(),
//!     ..Default::default()
//! });
//!
//! let router = mcp_router(server);
//! // Bind and serve with `tokio::net::TcpListener` and `axum::serve`
//! // in an async context. See axum documentation for details.
//! ```
//!
//! # MCP Protocol Transport (2025-11-25)
//!
//! MCP clients connect via SSE to discover the POST endpoint, then send
//! JSON-RPC requests over HTTP POST. The server responds synchronously
//! and may push notifications over the SSE connection.
//!
//! 1. Client opens `GET /mcp/sse`
//! 2. Server sends `event: endpoint\ndata: /mcp\n\n`
//! 3. Client sends JSON-RPC requests via `POST /mcp`
//! 4. Server may push notifications over the open SSE connection

use crate::server::McpServer;
use crate::types::JsonRpcRequest;
use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Shared state for the HTTP MCP server.
///
/// Wraps the synchronous [`McpServer`] in an `Arc<Mutex<>>` for safe access
/// from Axum's async handler threads. The `notification_tx` broadcast channel
/// allows any handler to push events to all connected SSE clients.
pub struct McpHttpServerState {
    /// The underlying MCP server (shared across requests).
    pub server: Arc<Mutex<McpServer>>,
    /// Broadcast channel for pushing notifications to SSE clients.
    pub notification_tx: broadcast::Sender<String>,
}

/// Type alias for the shared Axum state.
pub type SharedState = Arc<McpHttpServerState>;

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

/// Create an Axum [`Router`] with MCP HTTP/SSE endpoints.
///
/// Wraps the provided [`McpServer`] in shared state with a broadcast
/// notification channel. Returns a router with routes:
///
/// - `POST /mcp` — JSON-RPC request handler
/// - `GET /mcp/sse` — SSE notification stream
pub fn mcp_router(server: McpServer) -> Router {
    let (notification_tx, _) = broadcast::channel(100);
    let mut locked_server = server;
    locked_server.set_notification_channel(notification_tx.clone());
    let state = Arc::new(McpHttpServerState {
        server: Arc::new(Mutex::new(locked_server)),
        notification_tx,
    });

    Router::new()
        .route("/mcp", post(handle_mcp_post))
        .route("/mcp/sse", get(handle_mcp_sse))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

/// Send a notification to all connected SSE clients.
///
/// The notification is broadcast to every SSE client. Clients that are too
/// far behind (lagged) are silently disconnected.
pub fn send_notification(state: &McpHttpServerState, message: &str) {
    let _ = state.notification_tx.send(message.to_owned());
}

// ---------------------------------------------------------------------------
// Handler: POST /mcp
// ---------------------------------------------------------------------------

/// Handle a POST request to `/mcp`.
///
/// Deserializes the request body as a JSON-RPC 2.0 request, dispatches it
/// through the [`McpServer`], and returns the JSON-RPC response.
///
/// ## Errors
///
/// Returns HTTP 400 with a JSON-RPC parse error if the body is not a valid
/// JSON-RPC request. Returns HTTP 500 if the response serialization fails.
async fn handle_mcp_post(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    // Parse the incoming JSON as a JsonRpcRequest
    let req: JsonRpcRequest = match serde_json::from_value(body.clone()) {
        Ok(req) => req,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    }
                })),
            );
        }
    };

    // Dispatch through the MCP server
    let mut server = state.server.lock().unwrap();
    let msg = server.handle_request_to_message(&req);

    // Serialize the response
    match serde_json::to_value(&msg) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32603,
                    "message": format!("Internal error: {e}")
                }
            })),
        ),
    }
}

// ---------------------------------------------------------------------------
// Handler: GET /mcp/sse
// ---------------------------------------------------------------------------

/// Handle a GET request to `/mcp/sse`.
///
/// Opens a Server-Sent Events stream for server-to-client communication.
///
/// ## Event sequence
///
/// 1. `event: endpoint\ndata: /mcp` — tells the client where to send POST
///    requests
/// 2. Subsequent `data` events are MCP notifications (e.g.
///    `tools/list_changed`)
async fn handle_mcp_sse(
    State(state): State<SharedState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.notification_tx.subscribe();

    // Build a stream that first emits the endpoint event, then relays
    // broadcast notifications
    let endpoint_event = Ok(Event::default().event("endpoint").data("/mcp"));

    let notification_stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(msg) => Some(Ok(Event::default().data(msg))),
        Err(_) => None, // lagged — drop silently
    });

    // Prepend the endpoint event to the notification stream
    let stream = futures::stream::iter(vec![endpoint_event]).chain(notification_stream);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::McpServerConfig;
    use axum::http;
    use serde_json::json;
    use tower::util::ServiceExt;

    fn make_test_server() -> McpServer {
        McpServer::new(McpServerConfig {
            workspace_path: ".".to_owned(),
            ..Default::default()
        })
    }

    fn body_to_value(response: axum::response::Response) -> Value {
        let body_bytes = futures::executor::block_on(async {
            axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap()
        });
        serde_json::from_slice(&body_bytes).unwrap()
    }

    fn body_to_string(response: axum::response::Response) -> String {
        let body_bytes = futures::executor::block_on(async {
            axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap()
        });
        String::from_utf8(body_bytes.to_vec()).unwrap_or_default()
    }

    fn post(router: axum::Router, uri: &str, body: Value) -> axum::response::Response {
        futures::executor::block_on(
            router.oneshot(
                axum::http::Request::builder()
                    .method(http::Method::POST)
                    .uri(uri)
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(axum::body::Body::from(body.to_string()))
                    .unwrap(),
            ),
        )
        .unwrap()
    }

    #[test]
    fn test_sse_receives_endpoint_event() {
        let server = make_test_server();
        let router = mcp_router(server);

        // SSE requires a Tokio runtime — use tokio::runtime::Runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt
            .block_on(
                router.oneshot(
                    axum::http::Request::builder()
                        .method(http::Method::GET)
                        .uri("/mcp/sse")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                ),
            )
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response);
        assert!(
            body.contains("event: endpoint"),
            "SSE should start with endpoint event"
        );
        assert!(
            body.contains("data: /mcp"),
            "SSE endpoint event should point to /mcp"
        );
    }

    #[test]
    fn test_post_initialize() {
        let server = make_test_server();
        let router = mcp_router(server);

        let response = post(
            router,
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
        );

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_value(response);

        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        assert!(body["result"]["capabilities"]["tools"].is_object());
        assert_eq!(body["result"]["server_info"]["name"], "arpagona-mcp");
    }

    #[test]
    fn test_post_tools_list_after_init() {
        let server = make_test_server();
        let router = mcp_router(server);

        // Initialize
        let response = post(
            router.clone(),
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
        );
        assert_eq!(response.status(), StatusCode::OK);

        // Then list tools
        let response = post(
            router,
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
        );

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_value(response);

        let tools = body["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"read_file"));
    }

    #[test]
    fn test_post_unknown_method() {
        let server = make_test_server();
        let router = mcp_router(server);

        let response = post(
            router,
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "unknown_method",
                "params": {}
            }),
        );

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_value(response);

        assert_eq!(body["id"], 42);
        assert_eq!(body["error"]["code"], -32601);
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown_method"));
    }

    #[test]
    fn test_post_malformed_json() {
        let server = make_test_server();
        let router = mcp_router(server);

        let response = post(
            router,
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 1
                // missing "method" field
            }),
        );

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_to_value(response);

        assert_eq!(body["error"]["code"], -32700);
    }

    #[test]
    fn test_tools_list_before_init_returns_error() {
        let server = make_test_server();
        let router = mcp_router(server);

        let response = post(
            router,
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            }),
        );

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_value(response);

        assert!(body["error"].is_object(), "Should return an error");
        assert_eq!(body["error"]["code"], -32000);
    }
}
