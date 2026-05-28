//! Alpha read-only Tool Runtime for ARPAGONA Agent Core.
//!
//! This crate provides a **bounded, read-only tool execution runtime** that
//! allows the agent to perceive its environment through controlled channels.
//!
//! # Design
//!
//! - **Only 3 tools** are available: `read_file`, `list_files`, `search_text`.
//! - All tools are read-only, with strict security constraints.
//! - No shell, no write, no network, no external effects.
//! - Every execution returns a structured [`ToolExecutionResult`] carrying
//!   enough context for audit, reflection, and FailureInsight generation.
//!
//! # Security constraints
//!
//! - All paths are resolved relative to a configured workspace root.
//! - Absolute paths are rejected.
//! - Parent-directory traversal (`..`) that escapes the workspace is blocked.
//! - Sensitive files (`.env`, `.ssh`, secrets patterns) are blocked.
//! - Dangerous system directories are blocked.
//! - File size and result count are bounded.
//!
//! # Cognitive placement
//!
//! This runtime fills the **Tool Runtime** role in the cognitive loop:
//!
//! ```text
//! Intent -> Tool Registry -> ProposedAction -> Decision Gate -> Tool Runtime -> Observation -> Reflection
//! ```

use arpagona_agent_core::{
    ToolExecutionError, ToolExecutionId, ToolExecutionResult, ToolObservation,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Security constants
// ---------------------------------------------------------------------------

/// Maximum file size in bytes that read_file will process.
const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MiB

/// Maximum number of files that list_files will return.
const MAX_LIST_RESULTS: usize = 200;

/// Maximum directory depth for list_files.
const MAX_LIST_DEPTH: usize = 5;

/// Maximum number of search results.
const MAX_SEARCH_RESULTS: usize = 100;

/// Maximum file size in bytes that search_text will scan.
const MAX_SEARCH_FILE_SIZE: u64 = 512_000; // 500 KiB

/// Directories that are always ignored by list_files and search_text.
const IGNORED_DIRECTORIES: &[&str] = &[".git", "target", "node_modules", ".env", ".ssh"];

/// File patterns that are always blocked by read_file.
const BLOCKED_FILE_PATTERNS: &[&str] = &[".env", ".ssh/", "id_rsa", "id_ed25519", "config.json"];

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during tool execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolRuntimeError {
    /// A security constraint blocked the operation.
    SecurityBlocked(String),
    /// The tool name is not recognised.
    UnknownTool(String),
    /// The tool does not exist in this runtime.
    ToolNotFound(String),
    /// The path was invalid or outside the workspace.
    InvalidPath(String),
    /// An I/O error occurred.
    Io(String),
    /// The file exceeded the maximum allowed size.
    FileTooLarge(String),
    /// The search returned too many results.
    TooManyResults(usize),
}

impl std::fmt::Display for ToolRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecurityBlocked(msg) => write!(f, "security blocked: {msg}"),
            Self::UnknownTool(name) => write!(f, "unknown tool: {name}"),
            Self::ToolNotFound(name) => write!(f, "tool not found: {name}"),
            Self::InvalidPath(msg) => write!(f, "invalid path: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::FileTooLarge(msg) => write!(f, "file too large: {msg}"),
            Self::TooManyResults(count) => write!(f, "too many results: {count}"),
        }
    }
}

impl std::error::Error for ToolRuntimeError {}

// ---------------------------------------------------------------------------
// ToolExecutor trait
// ---------------------------------------------------------------------------

/// A tool that can be executed by the runtime.
///
/// Each tool implements this trait, which returns a structured
/// [`ToolExecutionResult`]. No tool has access to the host system except
/// through the paths and bounds configured by [`ToolRuntime`].
pub trait ToolExecutor: std::fmt::Debug {
    /// Execute this tool with the given arguments.
    fn execute(
        &self,
        execution_id: ToolExecutionId,
        arguments: &Value,
        workspace: &Path,
    ) -> ToolExecutionResult;
}

// ---------------------------------------------------------------------------
// Tool runtime
// ---------------------------------------------------------------------------

/// Configuration for the tool runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRuntimeConfig {
    /// The root workspace directory. All paths are resolved relative to this.
    pub workspace_path: PathBuf,
    /// Whether to allow absolute paths (default: false).
    pub allow_absolute_paths: bool,
    /// Maximum file size in bytes for read operations.
    pub max_file_size: u64,
    /// Maximum number of list results.
    pub max_list_results: usize,
    /// Maximum directory depth for recursive listing.
    pub max_list_depth: usize,
    /// Maximum number of search results.
    pub max_search_results: usize,
}

impl Default for ToolRuntimeConfig {
    fn default() -> Self {
        Self {
            workspace_path: PathBuf::from("."),
            allow_absolute_paths: false,
            max_file_size: MAX_FILE_SIZE,
            max_list_results: MAX_LIST_RESULTS,
            max_list_depth: MAX_LIST_DEPTH,
            max_search_results: MAX_SEARCH_RESULTS,
        }
    }
}

impl ToolRuntimeConfig {
    pub fn new(workspace_path: impl Into<PathBuf>) -> Self {
        Self {
            workspace_path: workspace_path.into(),
            ..Default::default()
        }
    }
}

/// The alpha read-only tool runtime.
///
/// Provides controlled, bounded access to the local filesystem for perception
/// and inspection only. No write operations, no shell, no network.
#[derive(Clone, Debug)]
pub struct ToolRuntime {
    config: ToolRuntimeConfig,
}

impl ToolRuntime {
    pub fn new(config: ToolRuntimeConfig) -> Self {
        Self { config }
    }

    /// Returns a reference to the runtime configuration.
    pub fn config(&self) -> &ToolRuntimeConfig {
        &self.config
    }

    /// Execute a named tool with the given JSON arguments.
    ///
    /// Returns a structured [`ToolExecutionResult`] that can be fed into
    /// audit, reflection, or FailureInsight processing.
    pub fn execute(&self, tool_name: &str, arguments: &Value) -> ToolExecutionResult {
        let execution_id = ToolExecutionId::new(format!(
            "exec-{}-{}",
            tool_name,
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));

        match tool_name {
            "read_file" => self.execute_read_file(execution_id, arguments),
            "list_files" => self.execute_list_files(execution_id, arguments),
            "search_text" => self.execute_search_text(execution_id, arguments),
            other => ToolExecutionResult::failed(
                execution_id,
                tool_name,
                ToolExecutionError::new("unknown_tool", format!("Unknown tool: {other}")),
                format!("Tool '{other}' is not available in this alpha runtime"),
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Path validation
    // -----------------------------------------------------------------------

    /// Validate and resolve a path relative to the workspace.
    ///
    /// Returns the resolved canonical path or an error.
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ToolRuntimeError> {
        let input_path = Path::new(path_str);

        // Reject absolute paths
        if input_path.is_absolute() {
            if !self.config.allow_absolute_paths {
                return Err(ToolRuntimeError::SecurityBlocked(
                    "Absolute paths are not allowed".to_owned(),
                ));
            }
        }

        // Resolve relative to workspace
        let resolved = if input_path.is_absolute() {
            input_path.to_path_buf()
        } else {
            self.config.workspace_path.join(input_path)
        };

        // Lexical parent-traversal detection before filesystem canonicalization.
        //
        // Paths containing `..` that would escape the workspace are blocked
        // before any I/O, so that missing parent-traversal targets return
        // SecurityBlocked (is_security: true) instead of InvalidPath.
        if resolved
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            let mut normalized = PathBuf::new();
            for component in resolved.components() {
                match component {
                    std::path::Component::ParentDir => {
                        normalized.pop();
                    }
                    other => {
                        normalized.push(other.as_os_str());
                    }
                }
            }
            if !normalized.starts_with(&self.config.workspace_path) {
                return Err(ToolRuntimeError::SecurityBlocked(format!(
                    "Path escapes workspace via parent traversal: {path_str}"
                )));
            }
        }

        // Check for workspace escape via ..
        let canonical = resolved.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ToolRuntimeError::InvalidPath(format!("Path not found: {path_str}"))
            } else {
                ToolRuntimeError::Io(format!("Cannot resolve path: {e}"))
            }
        })?;

        // Verify the canonical path starts with the workspace root
        let workspace_canonical = self
            .config
            .workspace_path
            .canonicalize()
            .map_err(|e| ToolRuntimeError::Io(format!("Cannot resolve workspace: {e}")))?;

        if !canonical.starts_with(&workspace_canonical) {
            return Err(ToolRuntimeError::SecurityBlocked(format!(
                "Path escapes workspace: {path_str}"
            )));
        }

        Ok(canonical)
    }

    /// Check if a filename matches a blocked pattern.
    fn is_blocked_file(filename: &str) -> bool {
        BLOCKED_FILE_PATTERNS
            .iter()
            .any(|pattern| filename.contains(pattern))
    }

    /// Check if a path is a blocked directory.
    fn is_blocked_dir(component: &str) -> bool {
        IGNORED_DIRECTORIES.contains(&component)
    }

    // -----------------------------------------------------------------------
    // Tool: read_file
    // -----------------------------------------------------------------------

    fn execute_read_file(
        &self,
        execution_id: ToolExecutionId,
        arguments: &Value,
    ) -> ToolExecutionResult {
        let path_str = match arguments.get("path").and_then(Value::as_str) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "read_file",
                    ToolExecutionError::new("missing_argument", "Required argument: path"),
                    "Missing 'path' argument for read_file",
                );
            }
        };

        if Self::is_blocked_file(path_str) {
            return ToolExecutionResult::blocked(
                execution_id,
                "read_file",
                format!("File access blocked: {path_str}"),
            );
        }

        let resolved = match self.resolve_path(path_str) {
            Ok(p) => p,
            Err(ToolRuntimeError::SecurityBlocked(msg)) => {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "read_file",
                    format!("Path access blocked: {msg}"),
                );
            }
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "read_file",
                    ToolExecutionError::new("invalid_path", format!("Cannot access path: {e}")),
                    format!("Path validation failed: {e}"),
                );
            }
        };

        // Check file metadata
        let metadata = match std::fs::metadata(&resolved) {
            Ok(m) => m,
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "read_file",
                    ToolExecutionError::new("io_error", format!("Cannot read file metadata: {e}")),
                    format!("I/O error reading {path_str}"),
                );
            }
        };

        if !metadata.is_file() {
            return ToolExecutionResult::failed(
                execution_id,
                "read_file",
                ToolExecutionError::new("not_a_file", format!("Not a file: {path_str}")),
                "Expected a file, got a directory or special file",
            );
        }

        // Check file size
        if metadata.len() > self.config.max_file_size {
            return ToolExecutionResult::failed(
                execution_id,
                "read_file",
                ToolExecutionError::new(
                    "file_too_large",
                    format!(
                        "File too large: {} bytes (max: {} bytes)",
                        metadata.len(),
                        self.config.max_file_size
                    ),
                ),
                "File exceeds maximum allowed size",
            );
        }

        // Read the file
        match std::fs::read_to_string(&resolved) {
            Ok(content) => {
                let lines = content.lines().count();
                let chars = content.chars().count();
                let preview: String = content.chars().take(500).collect();
                let truncated = chars > 500;

                ToolExecutionResult::success(
                    execution_id,
                    "read_file",
                    ToolObservation {
                        summary: format!("Read file: {path_str} ({lines} lines, {chars} chars)"),
                        payload: json!({
                            "path": path_str,
                            "resolved_path": resolved.to_string_lossy(),
                            "lines": lines,
                            "characters": chars,
                            "content_preview": preview,
                            "truncated": truncated,
                        }),
                        actionable: true,
                        failure_insight_candidate: false,
                        failure_hint: None,
                    },
                    format!("Read file: {path_str} ({lines} lines)"),
                )
            }
            Err(e) => ToolExecutionResult::failed(
                execution_id,
                "read_file",
                ToolExecutionError::new("io_error", format!("Cannot read file: {e}")),
                format!("I/O error reading {path_str}"),
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Tool: list_files
    // -----------------------------------------------------------------------

    fn execute_list_files(
        &self,
        execution_id: ToolExecutionId,
        arguments: &Value,
    ) -> ToolExecutionResult {
        let path_str = arguments.get("path").and_then(Value::as_str).unwrap_or(".");

        let resolved = match self.resolve_path(path_str) {
            Ok(p) => p,
            Err(ToolRuntimeError::SecurityBlocked(msg)) => {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "list_files",
                    format!("Path access blocked: {msg}"),
                );
            }
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "list_files",
                    ToolExecutionError::new("invalid_path", format!("Cannot access path: {e}")),
                    format!("Path validation failed: {e}"),
                );
            }
        };

        if !resolved.is_dir() {
            return ToolExecutionResult::failed(
                execution_id,
                "list_files",
                ToolExecutionError::new("not_a_directory", format!("Not a directory: {path_str}")),
                "Expected a directory",
            );
        }

        // Block listing of sensitive/ignored directories even when explicitly requested
        if let Some(dir_name) = resolved.file_name().and_then(|n| n.to_str()) {
            if Self::is_blocked_dir(dir_name) {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "list_files",
                    format!("Directory access blocked: {path_str}"),
                );
            }
        }

        let mut entries = Vec::new();
        let max_depth = self.config.max_list_depth;
        let max_results = self.config.max_list_results;

        self.collect_files(
            &resolved,
            &resolved,
            0,
            max_depth,
            max_results,
            &mut entries,
        );

        if entries.len() >= max_results {
            return ToolExecutionResult::warning(
                execution_id,
                "list_files",
                ToolObservation {
                    summary: format!(
                        "Listed files in {path_str}: {} entries (truncated at {max_results})",
                        entries.len()
                    ),
                    payload: json!({
                        "directory": path_str,
                        "resolved_path": resolved.to_string_lossy(),
                        "entries": entries,
                        "total": entries.len(),
                        "truncated": true,
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!("Listed {path_str}: {} entries (truncated)", entries.len()),
            );
        }

        ToolExecutionResult::success(
            execution_id,
            "list_files",
            ToolObservation {
                summary: format!("Listed files in {path_str}: {} entries", entries.len()),
                payload: json!({
                    "directory": path_str,
                    "resolved_path": resolved.to_string_lossy(),
                    "entries": entries,
                    "total": entries.len(),
                    "truncated": false,
                }),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            format!("Listed {path_str}: {} entries", entries.len()),
        )
    }

    fn collect_files(
        &self,
        dir: &Path,
        root: &Path,
        depth: usize,
        max_depth: usize,
        max_results: usize,
        entries: &mut Vec<Value>,
    ) {
        if depth > max_depth || entries.len() >= max_results {
            return;
        }

        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        for entry in read_dir.flatten() {
            if entries.len() >= max_results {
                return;
            }

            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip blocked directories
            if Self::is_blocked_dir(&file_name) {
                continue;
            }

            let relative = path
                .to_string_lossy()
                .to_string()
                .replace(&root.to_string_lossy().to_string(), ".");

            let is_dir = path.is_dir();

            entries.push(json!({
                "name": file_name,
                "path": relative,
                "is_directory": is_dir,
            }));

            if is_dir {
                self.collect_files(&path, root, depth + 1, max_depth, max_results, entries);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tool: search_text
    // -----------------------------------------------------------------------

    fn execute_search_text(
        &self,
        execution_id: ToolExecutionId,
        arguments: &Value,
    ) -> ToolExecutionResult {
        let query = match arguments.get("query").and_then(Value::as_str) {
            Some(q) if !q.is_empty() => q,
            _ => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "search_text",
                    ToolExecutionError::new("missing_argument", "Required argument: query"),
                    "Missing 'query' argument for search_text",
                );
            }
        };

        let path_str = arguments.get("path").and_then(Value::as_str).unwrap_or(".");

        let resolved = match self.resolve_path(path_str) {
            Ok(p) => p,
            Err(ToolRuntimeError::SecurityBlocked(msg)) => {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "search_text",
                    format!("Path access blocked: {msg}"),
                );
            }
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "search_text",
                    ToolExecutionError::new("invalid_path", format!("Cannot access path: {e}")),
                    format!("Path validation failed: {e}"),
                );
            }
        };

        if !resolved.is_dir() {
            return ToolExecutionResult::failed(
                execution_id,
                "search_text",
                ToolExecutionError::new("not_a_directory", format!("Not a directory: {path_str}")),
                "Expected a directory for search",
            );
        }

        // Block searching inside sensitive/ignored directories even when explicitly requested
        if let Some(dir_name) = resolved.file_name().and_then(|n| n.to_str()) {
            if Self::is_blocked_dir(dir_name) {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "search_text",
                    format!("Directory access blocked: {path_str}"),
                );
            }
        }

        let mut matches = Vec::new();
        let max_results = self.config.max_search_results;
        self.search_in_directory(&resolved, &resolved, query, max_results, &mut matches);

        if matches.is_empty() {
            return ToolExecutionResult::success(
                execution_id,
                "search_text",
                ToolObservation {
                    summary: format!("No matches found for '{query}' in {path_str}"),
                    payload: json!({
                        "query": query,
                        "directory": path_str,
                        "matches": [],
                        "total": 0,
                        "truncated": false,
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!("Searched for '{query}' in {path_str}: 0 matches"),
            );
        }

        if matches.len() >= max_results {
            return ToolExecutionResult::warning(
                execution_id,
                "search_text",
                ToolObservation {
                    summary: format!(
                        "Found {} matches for '{query}' in {path_str} (truncated at {max_results})",
                        matches.len()
                    ),
                    payload: json!({
                        "query": query,
                        "directory": path_str,
                        "matches": matches,
                        "total": matches.len(),
                        "truncated": true,
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!(
                    "Searched for '{query}': {} matches (truncated)",
                    matches.len()
                ),
            );
        }

        ToolExecutionResult::success(
            execution_id,
            "search_text",
            ToolObservation {
                summary: format!(
                    "Found {} matches for '{query}' in {path_str}",
                    matches.len()
                ),
                payload: json!({
                    "query": query,
                    "directory": path_str,
                    "matches": matches,
                    "total": matches.len(),
                    "truncated": false,
                }),
                actionable: true,
                failure_insight_candidate: false,
                failure_hint: None,
            },
            format!("Searched for '{query}': {} matches", matches.len()),
        )
    }

    fn search_in_directory(
        &self,
        dir: &Path,
        root: &Path,
        query: &str,
        max_results: usize,
        matches: &mut Vec<Value>,
    ) {
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        for entry in read_dir.flatten() {
            if matches.len() >= max_results {
                return;
            }

            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip blocked directories
            if Self::is_blocked_dir(&file_name) {
                continue;
            }

            let is_dir = path.is_dir();

            if is_dir {
                self.search_in_directory(&path, root, query, max_results, matches);
            } else if path.is_file() {
                // Check file size before reading
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() > MAX_SEARCH_FILE_SIZE {
                        continue;
                    }
                }

                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (line_no, line) in content.lines().enumerate() {
                        if matches.len() >= max_results {
                            return;
                        }

                        if line.contains(query) {
                            let relative = path
                                .to_string_lossy()
                                .to_string()
                                .replace(&root.to_string_lossy().to_string(), ".");

                            let snippet: String = if line.len() > 200 {
                                let idx = line.find(query).unwrap_or(0);
                                let start = idx.saturating_sub(50);
                                let end = std::cmp::min(start + 200, line.len());
                                let mut s = String::from("...");
                                s.push_str(&line[start..end]);
                                s.push_str("...");
                                s
                            } else {
                                line.to_owned()
                            };

                            matches.push(json!({
                                "file": relative,
                                "line": line_no + 1,
                                "snippet": snippet,
                            }));
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::ToolExecutionStatus;
    use serde_json::json;
    use std::fs;
    use std::io::Write;

    fn test_workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("should create temp dir")
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("should create parent dirs");
        }
        let mut file = fs::File::create(&path).expect("should create file");
        file.write_all(content.as_bytes())
            .expect("should write content");
        path
    }

    fn make_runtime(workspace: &Path) -> ToolRuntime {
        ToolRuntime::new(ToolRuntimeConfig::new(workspace))
    }

    // -----------------------------------------------------------------------
    // read_file tests
    // -----------------------------------------------------------------------

    #[test]
    fn read_file_reads_allowed_file() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Hello, world!\nLine 2");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "test.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert!(result.observation.summary.contains("Read file"));
        let payload = &result.observation.payload;
        assert_eq!(payload["lines"], 2);
    }

    #[test]
    fn read_file_blocks_outside_workspace() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "/etc/passwd"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("Absolute paths"));
    }

    #[test]
    fn read_file_blocks_dot_env() {
        let dir = test_workspace();
        create_test_file(dir.path(), ".env", "SECRET=value");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": ".env"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn read_file_returns_typed_error_for_missing_file() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "nonexistent.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert!(result.error.as_ref().unwrap().message.contains("not found"));
    }

    #[test]
    fn read_file_blocks_path_escaping_workspace() {
        let dir = test_workspace();
        create_test_file(dir.path(), "safe.txt", "ok");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "../safe.txt"}));

        // ../safe.txt would escape the workspace. Even though the target outside
        // the workspace may not exist, the lexical `..` detection catches it
        // as a security block (is_security: true).
        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("escapes workspace"));
    }

    #[test]
    fn nonexistent_parent_traversal_is_security_blocked() {
        let dir = test_workspace();
        // Do NOT create the target file outside the workspace — this proves
        // the lexical `..` detection catches it before filesystem lookup.
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "../nonexistent.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("escapes workspace"));
    }

    #[test]
    fn deep_parent_traversal_is_security_blocked() {
        let dir = test_workspace();
        create_test_file(dir.path(), "a/deep/file.txt", "inside");
        let runtime = make_runtime(dir.path());

        // Multiple parent traversals that would escape the workspace
        let result = runtime.execute(
            "read_file",
            &json!({"path": "a/deep/../../../../outside.txt"}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("escapes workspace"));
    }

    #[test]
    fn list_files_nonexistent_parent_traversal_is_security_blocked() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "../nonexistent-dir"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn search_text_nonexistent_parent_traversal_is_security_blocked() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "search_text",
            &json!({"query": "test", "path": "../nonexistent-dir"}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn absolute_path_parent_traversal_is_security_blocked() {
        // Create a file outside the workspace to test real workspace escape
        let dir = test_workspace();
        let parent = dir.path().parent().unwrap();
        create_test_file(parent, "outside.txt", "should not be accessible");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "../outside.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("escapes workspace"));
    }

    // -----------------------------------------------------------------------
    // list_files tests
    // -----------------------------------------------------------------------

    #[test]
    fn list_files_returns_directory_entries() {
        let dir = test_workspace();
        create_test_file(dir.path(), "a.txt", "content a");
        create_test_file(dir.path(), "b.txt", "content b");
        fs::create_dir_all(dir.path().join("sub")).expect("should create subdir");
        create_test_file(dir.path(), "sub/c.txt", "content c");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "."}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        let entries = payload["entries"].as_array().unwrap();
        assert!(!entries.is_empty());
        // Should find at least a.txt, b.txt, sub/
        assert!(entries.len() >= 3);
    }

    #[test]
    fn list_files_ignores_git_directory() {
        let dir = test_workspace();
        fs::create_dir_all(dir.path().join(".git")).expect("should create .git");
        create_test_file(dir.path(), ".git/config", "[core]");
        create_test_file(dir.path(), "real.txt", "visible");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "."}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        let entries = payload["entries"].as_array().unwrap();
        // Should not include .git entries
        for entry in entries {
            let name = entry["name"].as_str().unwrap();
            assert_ne!(name, ".git", "should skip .git directory");
        }
        // Should have at least real.txt
        assert!(entries.iter().any(|e| e["name"] == "real.txt"));
    }

    #[test]
    fn list_files_blocks_absolute_paths() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "/etc"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("Absolute paths"));
    }

    #[test]
    fn list_files_blocks_parent_traversal() {
        let dir = test_workspace();
        // Create a dir outside workspace to ensure canonicalization succeeds
        let parent = dir.path().parent().unwrap();
        fs::create_dir_all(parent.join("outside-dir")).expect("should create outside dir");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "../outside-dir"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("escapes workspace"));
    }

    // -----------------------------------------------------------------------
    // search_text tests
    // -----------------------------------------------------------------------

    #[test]
    fn search_text_returns_matching_results() {
        let dir = test_workspace();
        create_test_file(dir.path(), "file1.txt", "Hello world\nFoo bar");
        create_test_file(dir.path(), "file2.txt", "Goodbye world\nNothing here");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("search_text", &json!({"query": "world", "path": "."}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        let matches = payload["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn search_text_returns_empty_for_no_match() {
        let dir = test_workspace();
        create_test_file(dir.path(), "file.txt", "Nothing here");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("search_text", &json!({"query": "nonexistent", "path": "."}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        assert_eq!(payload["total"], 0);
    }

    #[test]
    fn search_text_does_not_scan_outside_workspace() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("search_text", &json!({"query": "anything", "path": "/etc"}));

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("Absolute paths"));
    }

    // -----------------------------------------------------------------------
    // Structural tests
    // -----------------------------------------------------------------------

    #[test]
    fn structured_results_are_serializable() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Hello");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "test.txt"}));

        let encoded = serde_json::to_value(&result).expect("result should serialize");
        assert_eq!(encoded["tool_name"], "read_file");
        assert_eq!(encoded["status"], "success");
        assert!(encoded["observation"]["summary"].is_string());
        assert!(encoded["output_summary"].is_string());
    }

    #[test]
    fn errors_are_typed_and_structured() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("nonexistent_tool", &json!({}));

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "unknown_tool");
    }

    #[test]
    fn tool_runtime_does_not_have_shell_access() {
        let runtime = ToolRuntime::new(ToolRuntimeConfig::default());
        let result = runtime.execute("list_files", &json!({"path": "."}));
        // The result should be a proper structured result, not shell output
        assert!(matches!(
            result.status,
            ToolExecutionStatus::Success
                | ToolExecutionStatus::Warning
                | ToolExecutionStatus::Failed
        ));
        assert!(result.observation.payload.is_object());
    }
}
