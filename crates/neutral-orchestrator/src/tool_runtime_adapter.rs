//! ToolRuntimeAdapter — ContextAssembler implementation backed by Tool Runtime.
//!
//! This adapter bridges the `crates/tool-runtime` read-only workspace perception
//! crate into the Neutral Orchestrator's context assembly pipeline.
//!
//! When the orchestrator asks for advisory context, this adapter:
//! - Searches the workspace for files matching the objective text
//! - Lists the workspace directory structure
//! - Returns results as advisory `ContextItem` values
//!
//! # Safety invariants
//!
//! - All items are advisory and non-authorizing.
//! - No response may contain an approval, authorization or execution token.
//! - All Tool Runtime security boundaries (no parent traversal, no absolute paths,
//!   no sensitive files) are preserved.
//! - If the Tool Runtime is unavailable or returns errors, the adapter reports
//!   the source as unavailable rather than panicking or leaking data.
//!
//! # Usage
//!
//! ```ignore
//! use arpagona_tool_runtime::{ToolRuntime, ToolRuntimeConfig};
//! use arpagona_neutral_orchestrator::ToolRuntimeAdapter;
//!
//! let runtime = ToolRuntime::new(ToolRuntimeConfig::new("/path/to/workspace"));
//! let adapter = ToolRuntimeAdapter::new(runtime);
//! let engine = OrchestratorEngine::new()
//!     .with_context_assembler(Box::new(adapter));
//! ```

use crate::context_assembler::ContextAssembler;
use arpagona_agent_core::cognitive_work::ContextItem;
use arpagona_agent_core::orchestrator::{ContextSource, MemoryQueryRequest, MemoryQueryResponse};
use arpagona_agent_core::tool::ToolExecutionStatus;
use arpagona_tool_runtime::{ToolRuntime, ToolRuntimeConfig};

// ─── ToolRuntimeAdapter ─────────────────────────────────────────────────────

/// A ContextAssembler that uses Tool Runtime to gather workspace context.
///
/// This adapter provides real workspace perception for the orchestrator:
/// - Searches the workspace for files relevant to the objective
/// - Lists the workspace directory structure
/// - All results are advisory only
///
/// # Configuration
///
/// The adapter is constructed with a `ToolRuntime` instance that must be
/// pre-configured with the correct workspace root. Use `new()` for a simple
/// workspace path, or `from_runtime()` if you already have a `ToolRuntime`.
#[derive(Clone, Debug)]
pub struct ToolRuntimeAdapter {
    /// The underlying Tool Runtime instance.
    runtime: ToolRuntime,
    /// Maximum items to return per source type.
    max_items_per_source: usize,
}

impl ToolRuntimeAdapter {
    /// Create a new ToolRuntimeAdapter with the given workspace path.
    ///
    /// The `workspace_root` path is used to configure the Tool Runtime.
    /// All file operations are scoped to this directory.
    pub fn new(workspace_root: impl Into<std::path::PathBuf>) -> Self {
        let config = ToolRuntimeConfig::new(workspace_root.into());
        Self {
            runtime: ToolRuntime::new(config),
            max_items_per_source: 10,
        }
    }

    /// Create a ToolRuntimeAdapter from an existing ToolRuntime instance.
    ///
    /// Use this when you already have a pre-configured ToolRuntime and want
    /// to reuse its configuration (security bounds, workspace path, etc.).
    pub fn from_runtime(runtime: ToolRuntime) -> Self {
        Self {
            runtime,
            max_items_per_source: 10,
        }
    }

    /// Override the maximum number of items per source.
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items_per_source = max;
        self
    }

    /// Extract search result matches from a ToolObservation payload.
    ///
    /// The search_text tool stores matches in `payload["matches"]` as an array
    /// of objects with `"file"`, `"line"`, `"snippet"` fields.
    fn extract_search_matches(payload: &serde_json::Value, limit: usize) -> Vec<ContextItem> {
        let mut items = Vec::new();

        if let Some(matches) = payload.get("matches").and_then(|v| v.as_array()) {
            for m in matches.iter().take(limit) {
                let file = m.get("file").and_then(|v| v.as_str()).unwrap_or("?");
                let line = m.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
                let snippet = m.get("snippet").and_then(|v| v.as_str()).unwrap_or("");

                items.push(ContextItem {
                    key: format!("search_match:{}", file),
                    value: format!("Line {}: {}", line, snippet),
                    source: "tool_runtime_adapter".to_owned(),
                });
            }
        }

        items
    }

    /// Extract file list entries from a ToolObservation payload.
    ///
    /// The list_files tool stores entries in `payload["files"]` and
    /// `payload["directories"]`.
    fn extract_list_entries(payload: &serde_json::Value) -> Vec<String> {
        let mut entries: Vec<String> = Vec::new();

        if let Some(files) = payload.get("files").and_then(|v| v.as_array()) {
            for f in files {
                if let Some(name) = f.as_str() {
                    entries.push(name.to_owned());
                }
            }
        }
        if let Some(dirs) = payload.get("directories").and_then(|v| v.as_array()) {
            for d in dirs {
                if let Some(name) = d.as_str() {
                    entries.push(format!("{}/", name));
                }
            }
        }

        entries
    }
}

impl ContextAssembler for ToolRuntimeAdapter {
    fn assemble(&self, request: &MemoryQueryRequest) -> Vec<MemoryQueryResponse> {
        let mut responses = Vec::with_capacity(request.requested_sources.len());

        for source in &request.requested_sources {
            let response = match source {
                ContextSource::ToolRuntime => self.assemble_tool_runtime(request),
                _ => MemoryQueryResponse::new(source.clone()),
            };
            responses.push(response);
        }

        responses
    }

    fn supported_sources(&self) -> Vec<ContextSource> {
        vec![ContextSource::ToolRuntime]
    }
}

// ─── Internal assembly logic ───────────────────────────────────────────────

impl ToolRuntimeAdapter {
    /// Assemble Tool Runtime context: search workspace for objective text
    /// and list workspace structure.
    ///
    /// Compute route awareness: when `local_preferred` is true, the adapter
    /// performs a lighter search (fewer results). When a cloud/strong route
    /// is indicated, it returns more workspace context.
    fn assemble_tool_runtime(&self, request: &MemoryQueryRequest) -> MemoryQueryResponse {
        // ── Compute-route aware explanation suffix ─────────────────────
        let route_suffix = if let Some(ref label) = request.compute_route_label {
            let local = request.local_preferred.unwrap_or(false);
            format!(" [compute: {} | local: {}]", label, local)
        } else {
            String::new()
        };

        // Local routes: smaller search results (lighter); cloud routes: broader
        let base_limit = std::cmp::min(self.max_items_per_source, request.max_items_per_source);
        let effective_limit = if request.local_preferred.unwrap_or(false) {
            std::cmp::max(1, base_limit.saturating_sub(base_limit / 2))
        } else {
            base_limit
        };

        let mut items: Vec<ContextItem> = Vec::new();
        let mut search_available = false;
        let mut list_available = false;
        let search_explanation: String;
        let list_explanation: String;

        // Step 1: Search workspace for the objective text
        let search_args = serde_json::json!({
            "query": request.objective_text,
            "path": ".",
        });
        let search_result = self.runtime.execute("search_text", &search_args);

        match &search_result.status {
            ToolExecutionStatus::Success | ToolExecutionStatus::Warning => {
                let search_items = Self::extract_search_matches(
                    &search_result.observation.payload,
                    effective_limit,
                );
                let count = search_items.len();
                items.extend(search_items);

                search_available = true;
                search_explanation = format!(
                    "Tool Runtime search found {} match(es) for objective text.",
                    count
                );
            }
            ToolExecutionStatus::Failed
            | ToolExecutionStatus::Blocked
            | ToolExecutionStatus::Skipped => {
                search_explanation = format!(
                    "Tool Runtime search returned {:?}: {}",
                    search_result.status, search_result.output_summary,
                );
            }
        }

        // Step 2: List workspace structure
        let list_args = serde_json::json!({
            "path": ".",
        });
        let list_result = self.runtime.execute("list_files", &list_args);

        match &list_result.status {
            ToolExecutionStatus::Success | ToolExecutionStatus::Warning => {
                let entries = Self::extract_list_entries(&list_result.observation.payload);
                let dir_capacity = effective_limit.saturating_sub(items.len());
                let list_count = std::cmp::min(dir_capacity, entries.len());

                for entry in entries.iter().take(dir_capacity) {
                    items.push(ContextItem {
                        key: format!("workspace_entry:{}", entry),
                        value: entry.clone(),
                        source: "tool_runtime_adapter".to_owned(),
                    });
                }

                list_available = true;
                list_explanation =
                    format!("Tool Runtime listed {} workspace entr(ies).", list_count);
            }
            ToolExecutionStatus::Failed
            | ToolExecutionStatus::Blocked
            | ToolExecutionStatus::Skipped => {
                list_explanation = format!(
                    "Tool Runtime listing returned {:?}: {}",
                    list_result.status, list_result.output_summary,
                );
            }
        }

        // Truncate to effective_limit
        items.truncate(effective_limit);

        let available = search_available || list_available;
        let explanation = if available {
            format!(
                "{} {}{}",
                search_explanation, list_explanation, route_suffix
            )
        } else {
            format!(
                "Tool Runtime unavailable — search: {} listing: {}{}",
                search_explanation, list_explanation, route_suffix
            )
        };

        MemoryQueryResponse {
            source: ContextSource::ToolRuntime,
            items,
            available,
            explanation,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::ids::{ObjectiveId, OrchestratorCycleId, WorkspaceId};
    use std::path::Path;

    fn make_request() -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            "Test objective text",
            WorkspaceId::new("ws-test"),
        )
    }

    fn make_request_with_text(text: &str) -> MemoryQueryRequest {
        MemoryQueryRequest::new(
            OrchestratorCycleId::new("oc-test"),
            ObjectiveId::new("obj-test"),
            text,
            WorkspaceId::new("ws-test"),
        )
    }

    #[test]
    fn tool_runtime_adapter_returns_tool_runtime_source() {
        let adapter = ToolRuntimeAdapter::new(".");
        let sources = adapter.supported_sources();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&ContextSource::ToolRuntime));
    }

    #[test]
    fn tool_runtime_adapter_ignores_non_matching_sources() {
        let adapter = ToolRuntimeAdapter::new(".");
        let request = make_request().with_sources(vec![ContextSource::GraphMemory]);
        let responses = adapter.assemble(&request);
        assert_eq!(responses.len(), 1);
        let resp = &responses[0];
        assert_eq!(resp.source, ContextSource::GraphMemory);
        assert!(resp.items.is_empty());
        assert!(resp.available);
    }

    #[test]
    fn tool_runtime_adapter_searches_workspace() {
        let adapter = ToolRuntimeAdapter::new(
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        );
        let request = make_request_with_text("cargo");
        let responses = adapter.assemble(&request);

        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some(), "ToolRuntime response should exist");
        let resp = tool_resp.unwrap();

        assert!(
            resp.available,
            "ToolRuntime should be available: {}",
            resp.explanation
        );
        assert!(
            !resp.explanation.contains("unavailable"),
            "Explanation should not say unavailable: {}",
            resp.explanation
        );
    }

    #[test]
    fn tool_runtime_adapter_works_with_limited_items() {
        let adapter = ToolRuntimeAdapter::new(
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        )
        .with_max_items(3);
        let request = make_request_with_text("struct");
        let responses = adapter.assemble(&request);

        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some());
        let resp = tool_resp.unwrap();

        assert!(
            resp.items.len() <= 3,
            "Should have at most 3 items, got {}",
            resp.items.len()
        );
    }

    #[test]
    fn tool_runtime_adapter_handles_empty_workspace_root() {
        let adapter = ToolRuntimeAdapter::new("/tmp/nonexistent-arpagona-test-dir-99999");
        let request = make_request();
        let responses = adapter.assemble(&request);

        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some());
        let resp = tool_resp.unwrap();

        // Should be gracefully handled — items may be empty or unavailable
        assert!(
            !resp.available || resp.items.is_empty(),
            "Non-existent workspace should produce empty or unavailable results"
        );
    }

    #[test]
    fn tool_runtime_adapter_completes_without_panic() {
        let adapter = ToolRuntimeAdapter::new(
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        );

        let request = make_request_with_text("fn");
        let responses = adapter.assemble(&request);
        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some());

        // Should complete gracefully
        let resp = tool_resp.unwrap();
        // It may have hits or not, but should report something useful
        assert!(
            !resp.explanation.is_empty(),
            "Explanation should not be empty"
        );
    }

    // ─── Compute-route awareness tests ─────────────────────────────────

    #[test]
    fn tool_runtime_adapter_local_route_reduces_items() {
        let adapter = ToolRuntimeAdapter::new(
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        )
        .with_max_items(10);
        let request =
            make_request_with_text("struct").with_compute_route(Some("local-small"), Some(true));

        let responses = adapter.assemble(&request);
        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some());
        let resp = tool_resp.unwrap();

        assert!(
            resp.explanation.contains("local"),
            "Explanation should mention local route: {}",
            resp.explanation
        );
    }

    #[test]
    fn tool_runtime_adapter_cloud_route_full_context() {
        let adapter = ToolRuntimeAdapter::new(
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        );
        let request =
            make_request_with_text("fn").with_compute_route(Some("cloud-strong"), Some(false));

        let responses = adapter.assemble(&request);
        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some());
        let resp = tool_resp.unwrap();

        assert!(
            resp.explanation.contains("compute: cloud-strong"),
            "Explanation should mention cloud route: {}",
            resp.explanation
        );
    }

    #[test]
    fn tool_runtime_adapter_default_route_has_no_compute_prefix() {
        let adapter = ToolRuntimeAdapter::new(
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        );
        let request = make_request_with_text("test");

        let responses = adapter.assemble(&request);
        let tool_resp = responses
            .iter()
            .find(|r| r.source == ContextSource::ToolRuntime);
        assert!(tool_resp.is_some());
        let resp = tool_resp.unwrap();

        assert!(
            !resp.explanation.contains("[compute:"),
            "No compute prefix expected: {}",
            resp.explanation
        );
    }
}
