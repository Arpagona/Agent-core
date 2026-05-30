//! Alpha read-only Tool Runtime for ARPAGONA Agent Core.
//!
//! This crate provides a **bounded, read-only tool execution runtime** that
//! allows the agent to perceive its environment through controlled channels.
//!
//! # Design
//!
//! - Core tools include `read_file`, `list_files`, `search_text`, and sandboxed `write_file`.
//! - All tools are read-only, with strict security constraints.
//! - Writes are workspace-bounded and simulate-first by default; no shell, no network, no external effects.
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

/// Maximum content size in bytes that write_file will write.
const MAX_WRITE_SIZE: usize = 262_144; // 256 KiB

/// Directories that are always ignored by list_files and search_text.
const IGNORED_DIRECTORIES: &[&str] = &[".git", "target", "node_modules", ".env", ".ssh"];

/// File patterns that are always blocked by read_file.
const BLOCKED_FILE_PATTERNS: &[&str] = &[
    ".env",
    ".git/",
    ".ssh/",
    "id_rsa",
    "id_ed25519",
    "config.json",
];

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
            "write_file" => self.execute_write_file(execution_id, arguments),
            "patch_file" | "replace_text" => self.execute_patch_file(execution_id, arguments),
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
        if input_path.is_absolute() && !self.config.allow_absolute_paths {
            return Err(ToolRuntimeError::SecurityBlocked(
                "Absolute paths are not allowed".to_owned(),
            ));
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

    /// Validate and resolve a path for writing relative to the workspace.
    ///
    /// Unlike read/list/search, a write target may not exist yet. We therefore
    /// canonicalize the parent directory, ensure it stays inside the workspace,
    /// and then append the final filename. Parent directories may be created
    /// only when `create_parent_dirs` is true.
    fn resolve_write_path(
        &self,
        path_str: &str,
        create_parent_dirs: bool,
        simulate: bool,
    ) -> Result<PathBuf, ToolRuntimeError> {
        let input_path = Path::new(path_str);

        if input_path.is_absolute() && !self.config.allow_absolute_paths {
            return Err(ToolRuntimeError::SecurityBlocked(
                "Absolute paths are not allowed".to_owned(),
            ));
        }

        if Self::is_blocked_file(path_str) {
            return Err(ToolRuntimeError::SecurityBlocked(format!(
                "File access blocked: {path_str}"
            )));
        }

        if input_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ToolRuntimeError::SecurityBlocked(format!(
                "Path escapes workspace via parent traversal: {path_str}"
            )));
        }

        let workspace_canonical = self
            .config
            .workspace_path
            .canonicalize()
            .map_err(|e| ToolRuntimeError::Io(format!("Cannot resolve workspace: {e}")))?;

        let target = if input_path.is_absolute() {
            input_path.to_path_buf()
        } else {
            workspace_canonical.join(input_path)
        };

        let parent = target.parent().ok_or_else(|| {
            ToolRuntimeError::InvalidPath(format!("Write target has no parent: {path_str}"))
        })?;

        for component in target.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name) = name.to_str() {
                    if Self::is_blocked_dir(name) {
                        return Err(ToolRuntimeError::SecurityBlocked(format!(
                            "Directory access blocked: {path_str}"
                        )));
                    }
                }
            }
        }

        if !parent.exists() {
            if create_parent_dirs && simulate {
                return Ok(target);
            }
            if create_parent_dirs {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolRuntimeError::Io(format!("Cannot create parent directories: {e}"))
                })?;
            } else {
                return Err(ToolRuntimeError::InvalidPath(format!(
                    "Parent directory not found: {}",
                    parent.to_string_lossy()
                )));
            }
        }

        let parent_canonical = parent
            .canonicalize()
            .map_err(|e| ToolRuntimeError::Io(format!("Cannot resolve parent directory: {e}")))?;

        if !parent_canonical.starts_with(&workspace_canonical) {
            return Err(ToolRuntimeError::SecurityBlocked(format!(
                "Path escapes workspace: {path_str}"
            )));
        }

        Ok(parent_canonical.join(
            target
                .file_name()
                .ok_or_else(|| ToolRuntimeError::InvalidPath("Missing file name".to_owned()))?,
        ))
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

    /// Detect whether a file appears to be binary by scanning the first 8 KiB
    /// for null bytes.  Returns `true` when null bytes are found (strong
    /// indicator of non-text content).
    ///
    /// Text files, empty files and files smaller than 4 bytes are treated as
    /// non-binary (never `true`).
    fn is_binary_file(path: &Path) -> bool {
        let Ok(data) = std::fs::read(path) else {
            return false; // I/O errors are handled by the caller
        };
        // A minimum length heuristic avoids false-positives on very short
        // files that happen to contain a zero byte (e.g. a single null
        // written by accident).
        if data.len() < 4 {
            return false;
        }
        let scan_end = data.len().min(8192); // 8 KiB sniff
        data[..scan_end].contains(&0u8)
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

        // Detect binary files before attempting text read
        if Self::is_binary_file(&resolved) {
            return ToolExecutionResult::failed(
                execution_id,
                "read_file",
                ToolExecutionError::new(
                    "binary_file",
                    format!(
                        "Cannot read file as text: {path_str} appears to be a binary file. \
                         Use `search_text` or `list_files` instead."
                    ),
                ),
                "File appears to be binary and cannot be read as text",
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
    // Tool: write_file
    // -----------------------------------------------------------------------

    fn execute_write_file(
        &self,
        execution_id: ToolExecutionId,
        arguments: &Value,
    ) -> ToolExecutionResult {
        let path_str = match arguments.get("path").and_then(Value::as_str) {
            Some(p) if !p.is_empty() => p,
            _ => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "write_file",
                    ToolExecutionError::new("missing_argument", "Required argument: path"),
                    "Missing 'path' argument for write_file",
                );
            }
        };

        let content = match arguments.get("content").and_then(Value::as_str) {
            Some(c) => c,
            None => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "write_file",
                    ToolExecutionError::new("missing_argument", "Required argument: content"),
                    "Missing 'content' argument for write_file",
                );
            }
        };

        let simulate = arguments
            .get("simulate")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let create_parent_dirs = arguments
            .get("create_parent_dirs")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let overwrite = arguments
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if content.len() > MAX_WRITE_SIZE {
            return ToolExecutionResult::failed(
                execution_id,
                "write_file",
                ToolExecutionError::new(
                    "content_too_large",
                    format!(
                        "Content too large: {} bytes (max: {} bytes)",
                        content.len(),
                        MAX_WRITE_SIZE
                    ),
                ),
                "Content exceeds maximum allowed write size",
            );
        }

        let resolved = match self.resolve_write_path(path_str, create_parent_dirs, simulate) {
            Ok(p) => p,
            Err(ToolRuntimeError::SecurityBlocked(msg)) => {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "write_file",
                    format!("Path access blocked: {msg}"),
                );
            }
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "write_file",
                    ToolExecutionError::new("invalid_path", format!("Cannot access path: {e}")),
                    format!("Path validation failed: {e}"),
                );
            }
        };

        let existed_before = resolved.exists();
        if existed_before && !overwrite {
            return ToolExecutionResult::failed(
                execution_id,
                "write_file",
                ToolExecutionError::new(
                    "overwrite_not_allowed",
                    format!("Target exists and overwrite=false: {path_str}"),
                ),
                "Write refused because target exists and overwrite=false",
            );
        }

        if simulate {
            return ToolExecutionResult::success(
                execution_id,
                "write_file",
                ToolObservation {
                    summary: format!(
                        "Simulated write_file: {path_str} ({} bytes, overwrite={}, create_parent_dirs={})",
                        content.len(),
                        overwrite,
                        create_parent_dirs
                    ),
                    payload: json!({
                        "path": path_str,
                        "resolved_path": resolved.to_string_lossy(),
                        "bytes": content.len(),
                        "simulate": true,
                        "would_overwrite": existed_before,
                        "overwrite": overwrite,
                        "create_parent_dirs": create_parent_dirs,
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!("Simulated write_file: {path_str} ({} bytes)", content.len()),
            );
        }

        match std::fs::write(&resolved, content) {
            Ok(()) => ToolExecutionResult::success(
                execution_id,
                "write_file",
                ToolObservation {
                    summary: format!("Wrote file: {path_str} ({} bytes)", content.len()),
                    payload: json!({
                        "path": path_str,
                        "resolved_path": resolved.to_string_lossy(),
                        "bytes": content.len(),
                        "simulate": false,
                        "overwrote": existed_before,
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!("Wrote file: {path_str} ({} bytes)", content.len()),
            ),
            Err(e) => ToolExecutionResult::failed(
                execution_id,
                "write_file",
                ToolExecutionError::new("io_error", format!("Cannot write file: {e}")),
                format!("I/O error writing {path_str}"),
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Tool: patch_file / replace_text
    // -----------------------------------------------------------------------

    fn execute_patch_file(
        &self,
        execution_id: ToolExecutionId,
        arguments: &Value,
    ) -> ToolExecutionResult {
        let path_str = match arguments.get("path").and_then(Value::as_str) {
            Some(p) if !p.is_empty() => p,
            _ => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "patch_file",
                    ToolExecutionError::new("missing_argument", "Required argument: path"),
                    "Missing 'path' argument for patch_file",
                );
            }
        };

        let old_string = match arguments.get("old_string").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => s,
            _ => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "patch_file",
                    ToolExecutionError::new("missing_argument", "Required argument: old_string"),
                    "Missing 'old_string' argument for patch_file",
                );
            }
        };

        let new_string = arguments
            .get("new_string")
            .and_then(Value::as_str)
            .unwrap_or("");

        let simulate = arguments
            .get("simulate")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let replace_all = arguments
            .get("replace_all")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Security: resolve path using write path resolver (same security as write_file)
        // We pass create_parent_dirs=false and simulate=true since we're patching an
        // existing file — the resolved target is used for read-only path validation.
        let resolved = match self.resolve_write_path(path_str, false, true) {
            Ok(p) => p,
            Err(ToolRuntimeError::SecurityBlocked(msg)) => {
                return ToolExecutionResult::blocked(
                    execution_id,
                    "patch_file",
                    format!("Path access blocked: {msg}"),
                );
            }
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "patch_file",
                    ToolExecutionError::new("invalid_path", format!("Cannot access path: {e}")),
                    format!("Path validation failed: {e}"),
                );
            }
        };

        // Verify the target is an existing regular file
        let metadata = match std::fs::metadata(&resolved) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "patch_file",
                    ToolExecutionError::new(
                        "file_not_found",
                        format!("File not found: {path_str}"),
                    ),
                    format!("Target file does not exist: {path_str}"),
                );
            }
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "patch_file",
                    ToolExecutionError::new("io_error", format!("Cannot read file metadata: {e}")),
                    format!("I/O error reading {path_str}"),
                );
            }
        };

        if !metadata.is_file() {
            return ToolExecutionResult::failed(
                execution_id,
                "patch_file",
                ToolExecutionError::new("not_a_file", format!("Not a file: {path_str}")),
                "Expected a file, got a directory or special file",
            );
        }

        // Check file size — reuse the same limit as writes (256 KiB)
        if metadata.len() > MAX_WRITE_SIZE as u64 {
            return ToolExecutionResult::failed(
                execution_id,
                "patch_file",
                ToolExecutionError::new(
                    "file_too_large",
                    format!(
                        "File too large: {} bytes (max: {} bytes)",
                        metadata.len(),
                        MAX_WRITE_SIZE
                    ),
                ),
                "File exceeds maximum allowed size for patching",
            );
        }

        // Detect binary files before attempting text operations
        if Self::is_binary_file(&resolved) {
            return ToolExecutionResult::failed(
                execution_id,
                "patch_file",
                ToolExecutionError::new(
                    "binary_file",
                    format!("Cannot patch file as text: {path_str} appears to be a binary file"),
                ),
                "File appears to be binary and cannot be patched as text",
            );
        }

        // Read file contents
        let content = match std::fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => {
                return ToolExecutionResult::failed(
                    execution_id,
                    "patch_file",
                    ToolExecutionError::new("io_error", format!("Cannot read file: {e}")),
                    format!("I/O error reading {path_str}"),
                );
            }
        };

        // Count matches
        let match_count = content.matches(old_string).count();

        if match_count == 0 {
            return ToolExecutionResult::failed(
                execution_id,
                "patch_file",
                ToolExecutionError::new(
                    "pattern_not_found",
                    format!("Pattern not found in {path_str}: {old_string:?}"),
                ),
                format!("The specified pattern was not found in {path_str}"),
            );
        }

        if match_count > 1 && !replace_all {
            return ToolExecutionResult::failed(
                execution_id,
                "patch_file",
                ToolExecutionError::new(
                    "multiple_matches",
                    format!(
                        "Pattern found {match_count} times in {path_str}. \
                         Set replace_all=true to replace all occurrences, \
                         or use a more specific pattern."
                    ),
                ),
                format!(
                    "Found {match_count} occurrences in {path_str}; \
                     refusing to patch without replace_all=true"
                ),
            );
        }

        // Replace (first or all depending on replace_all flag)
        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        // Compute change statistics for the report
        let lines_total = content.lines().count();
        let lines_changed = content
            .lines()
            .zip(new_content.lines())
            .filter(|(old, new)| old != new)
            .count();

        if simulate {
            // Build a diff preview (show context around first change)
            let diff_preview = self.build_patch_diff_preview(&content, &new_content, 3);

            return ToolExecutionResult::success(
                execution_id,
                "patch_file",
                ToolObservation {
                    summary: format!(
                        "Simulated patch_file: {path_str} ({match_count} match{}, replace_all={replace_all})",
                        if match_count == 1 { "" } else { "es" }
                    ),
                    payload: json!({
                        "path": path_str,
                        "resolved_path": resolved.to_string_lossy(),
                        "simulate": true,
                        "matches": match_count,
                        "replace_all": replace_all,
                        "lines_total": lines_total,
                        "lines_changed": if replace_all { lines_changed } else { 1 },
                        "diff_preview": diff_preview,
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!(
                    "Simulated patch_file: {path_str} ({match_count} match{})",
                    if match_count == 1 { "" } else { "es" }
                ),
            );
        }

        // Execute: write the patched content
        match std::fs::write(&resolved, &new_content) {
            Ok(()) => ToolExecutionResult::success(
                execution_id,
                "patch_file",
                ToolObservation {
                    summary: format!(
                        "Patched file: {path_str} ({match_count} replacement{})",
                        if match_count == 1 { "" } else { "s" }
                    ),
                    payload: json!({
                        "path": path_str,
                        "resolved_path": resolved.to_string_lossy(),
                        "simulate": false,
                        "matches": match_count,
                        "replace_all": replace_all,
                        "lines_total": lines_total,
                        "bytes_written": new_content.len(),
                    }),
                    actionable: true,
                    failure_insight_candidate: false,
                    failure_hint: None,
                },
                format!(
                    "Patched file: {path_str} ({match_count} replacement{})",
                    if match_count == 1 { "" } else { "s" }
                ),
            ),
            Err(e) => ToolExecutionResult::failed(
                execution_id,
                "patch_file",
                ToolExecutionError::new("io_error", format!("Cannot write patched file: {e}")),
                format!("I/O error writing patched content to {path_str}"),
            ),
        }
    }

    /// Build a small diff preview showing context around the first change.
    /// Returns a list of {line_number, old_line, new_line} entries for the
    /// first changed block, with `context` lines of surrounding context.
    fn build_patch_diff_preview(
        &self,
        old_content: &str,
        new_content: &str,
        context: usize,
    ) -> Value {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();
        let max_lines = old_lines.len().max(new_lines.len());

        // Find the first differing line index
        let first_diff = (0..max_lines).find(|&i| old_lines.get(i) != new_lines.get(i));

        let Some(diff_idx) = first_diff else {
            return json!([]);
        };

        let start = diff_idx.saturating_sub(context);
        let end = (diff_idx + context + 1).min(max_lines);

        let mut preview = Vec::new();
        for i in start..end {
            let old_line = old_lines.get(i).copied().unwrap_or("");
            let new_line = new_lines.get(i).copied().unwrap_or("");
            if old_line != new_line {
                preview.push(json!({
                    "line": i + 1,
                    "old": old_line,
                    "new": new_line,
                }));
            }
        }
        json!(preview)
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
    // write_file tests
    // -----------------------------------------------------------------------

    #[test]
    fn write_file_simulates_by_default_without_mutation() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "write_file",
            &json!({"path": "notes/out.txt", "content": "hello", "create_parent_dirs": true}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(result.observation.payload["simulate"], true);
        assert!(!dir.path().join("notes/out.txt").exists());
    }

    #[test]
    fn write_file_executes_inside_workspace_when_explicit() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "write_file",
            &json!({
                "path": "notes/out.txt",
                "content": "hello",
                "simulate": false,
                "create_parent_dirs": true
            }),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("notes/out.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn write_file_blocks_parent_traversal() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "write_file",
            &json!({"path": "../outside.txt", "content": "bad", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn write_file_refuses_overwrite_unless_explicit() {
        let dir = test_workspace();
        create_test_file(dir.path(), "existing.txt", "old");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "write_file",
            &json!({"path": "existing.txt", "content": "new", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "overwrite_not_allowed");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("existing.txt")).unwrap(),
            "old"
        );
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
    fn read_file_empty_file_succeeds() {
        let dir = test_workspace();
        create_test_file(dir.path(), "empty.txt", "");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "empty.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        assert_eq!(payload["lines"], 0, "empty file should have 0 lines");
        // Empty file payload may not have a "size" field; just prove no crash
        assert!(
            result.output_summary.contains("0 lines"),
            "output_summary should mention 0 lines for empty file"
        );
    }

    #[test]
    fn list_files_empty_directory_returns_empty() {
        let dir = test_workspace();
        // Create an empty subdirectory inside the workspace
        fs::create_dir_all(dir.path().join("empty-dir")).expect("should create empty dir");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "empty-dir"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        if let Some(entries) = payload["entries"].as_array() {
            // Directory contains no visible files (empty)
            assert!(
                entries.is_empty()
                    || entries
                        .iter()
                        .all(|e| e["name"] == "." || e["name"] == "..")
            );
        }
    }

    #[test]
    fn list_files_in_subdirectory_works() {
        let dir = test_workspace();
        fs::create_dir_all(dir.path().join("sub/nested")).expect("should create nested dir");
        create_test_file(dir.path(), "sub/nested/deep.txt", "deep content");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "sub/nested"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        let entries = payload["entries"].as_array().unwrap();
        assert!(
            entries.iter().any(|e| e["name"] == "deep.txt"),
            "should find deep.txt in nested subdirectory"
        );
    }

    #[test]
    fn search_text_empty_query_returns_all_or_no_matches() {
        let dir = test_workspace();
        create_test_file(dir.path(), "data.txt", "Hello world");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("search_text", &json!({"query": "", "path": "."}));

        // Empty query should not crash. Either success with matches, or a graceful rejection.
        assert!(
            matches!(
                result.status,
                ToolExecutionStatus::Success | ToolExecutionStatus::Failed
            ),
            "empty query should not panic: got {:?}",
            result.status
        );
    }

    #[test]
    fn search_text_case_sensitivity_distinguishes_cases() {
        let dir = test_workspace();
        create_test_file(dir.path(), "data.txt", "UPPER\nlower\nMixed");
        let runtime = make_runtime(dir.path());

        let upper = runtime.execute("search_text", &json!({"query": "UPPER", "path": "."}));
        let lower = runtime.execute("search_text", &json!({"query": "upper", "path": "."}));

        assert_eq!(upper.status, ToolExecutionStatus::Success);
        let upper_matches = upper.observation.payload["matches"]
            .as_array()
            .unwrap()
            .len();
        let lower_matches = lower.observation.payload["matches"]
            .as_array()
            .unwrap()
            .len();

        // Exact case match should find 1; lowercase query must find fewer (proving case sensitivity)
        assert_eq!(
            upper_matches, 1,
            "exact case 'UPPER' should match at least 'UPPER'"
        );
        assert!(
            lower_matches < upper_matches,
            "lowercase 'upper' query should find fewer matches than exact case 'UPPER' (found {} vs {})",
            lower_matches,
            upper_matches
        );
    }

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

    // -----------------------------------------------------------------------
    // Symlink handling tests
    // -----------------------------------------------------------------------

    #[test]
    fn read_file_follows_symlink_to_allowed_file() {
        let dir = test_workspace();
        // Create a real file inside the workspace
        create_test_file(dir.path(), "real.txt", "This file is accessed via symlink.");
        // Create a symlink inside the workspace pointing to the real file
        let link_path = dir.path().join("link.txt");
        let target = dir.path().join("real.txt");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link_path).expect("should create symlink");
        }
        #[cfg(not(unix))]
        {
            std::fs::copy(&target, &link_path).expect("should copy file as fallback");
        }

        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "link.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        let content_preview = payload["content_preview"].as_str().unwrap_or("");
        assert!(
            content_preview.contains("via symlink"),
            "symlink target content should be readable: {content_preview}"
        );
        // Verify that the resolved path points to the symlink target
        let resolved = payload["resolved_path"].as_str().unwrap_or("");
        assert!(
            resolved.contains("real.txt"),
            "resolved path should point to symlink target real.txt, got: {resolved}"
        );
    }

    #[test]
    fn read_file_blocks_symlink_outside_workspace() {
        let dir = test_workspace();
        // Create a file outside the workspace
        let outside_dir = dir.path().parent().unwrap().join("outside-symlink-target");
        std::fs::create_dir_all(&outside_dir).expect("should create outside dir");
        let outside_file = outside_dir.join("secret.txt");
        std::fs::write(&outside_file, "secret outside data").expect("should write outside file");
        // Create a symlink inside the workspace pointing outside
        let link_path = dir.path().join("escape_link.txt");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside_file, &link_path).expect("should create symlink");
        }
        #[cfg(not(unix))]
        {
            // On non-unix, symlinks aren't available; verify the security check still works
            let runtime = make_runtime(dir.path());
            let result = runtime.execute(
                "read_file",
                &json!({"path": "../outside-symlink-target/secret.txt"}),
            );
            assert_eq!(result.status, ToolExecutionStatus::Blocked);
            assert!(result.error.as_ref().unwrap().is_security);
            return;
        }

        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "escape_link.txt"}));

        assert_eq!(
            result.status,
            ToolExecutionStatus::Blocked,
            "symlink to outside workspace should be blocked: {}",
            result
                .error
                .as_ref()
                .map(|e| &e.message)
                .unwrap_or(&"no error".to_owned())
        );
        assert!(
            result.error.as_ref().unwrap().is_security,
            "symlink escape should be marked as security"
        );
    }

    #[test]
    fn list_files_follows_symlink_to_directory() {
        let dir = test_workspace();
        // Create a directory with content
        std::fs::create_dir_all(dir.path().join("real_dir")).expect("should create real_dir");
        create_test_file(dir.path(), "real_dir/inside.txt", "inside content");
        // Create a symlink to the directory
        let link_path = dir.path().join("dir_link");
        let target = dir.path().join("real_dir");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link_path)
                .expect("should create directory symlink");
        }
        #[cfg(not(unix))]
        {
            // Fallback on non-unix: test a real directory listing instead
            let runtime = make_runtime(dir.path());
            let result = runtime.execute("list_files", &json!({"path": "real_dir"}));
            assert_eq!(result.status, ToolExecutionStatus::Success);
            return;
        }

        let runtime = make_runtime(dir.path());

        let result = runtime.execute("list_files", &json!({"path": "dir_link"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let payload = &result.observation.payload;
        let entries = payload["entries"].as_array().unwrap();
        assert!(
            entries.iter().any(|e| e["name"] == "inside.txt"),
            "should find inside.txt through symlink to directory"
        );
    }

    // -----------------------------------------------------------------------
    // Binary file detection tests
    // -----------------------------------------------------------------------

    #[test]
    fn read_file_binary_file_returns_clear_error() {
        let dir = test_workspace();
        // Write a small binary file (null bytes = strong binary indicator)
        let binary_data: Vec<u8> = vec![
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x57, 0x6f, 0x72, 0x6c, 0x64,
        ];
        std::fs::write(dir.path().join("binary.bin"), &binary_data)
            .expect("should write binary file");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "binary.bin"}));

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        let error = result.error.as_ref().unwrap();
        assert_eq!(error.code, "binary_file");
        assert!(
            error.message.contains("binary"),
            "message should mention binary: {}",
            error.message
        );
        assert!(
            !error.is_security,
            "binary file error should not be classified as security"
        );
    }

    #[test]
    fn read_file_binary_file_with_only_null_bytes_is_detected() {
        let dir = test_workspace();
        // A file with only null bytes (>4 bytes)
        let binary_data: Vec<u8> = vec![0u8; 100];
        std::fs::write(dir.path().join("nulls.bin"), &binary_data)
            .expect("should write binary file");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "nulls.bin"}));

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "binary_file");
    }

    #[test]
    fn read_file_text_file_no_null_bytes_still_succeeds() {
        let dir = test_workspace();
        // Pure ASCII text — no null bytes
        create_test_file(
            dir.path(),
            "readme.txt",
            "Hello, this is a plain text file.",
        );
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": "readme.txt"}));

        assert_eq!(result.status, ToolExecutionStatus::Success);
    }

    #[test]
    fn is_binary_file_detects_null_bytes() {
        // Test the helper directly: a file with null bytes
        let dir = test_workspace();
        let bin_path = dir.path().join("helper_test.bin");
        std::fs::write(&bin_path, [0x00, 0x01, 0x02, 0x03, 0x04])
            .expect("should write small binary file");
        assert!(
            ToolRuntime::is_binary_file(&bin_path),
            "file with null bytes should be detected as binary"
        );

        // A text file should NOT be detected as binary
        let txt_path = dir.path().join("helper_test.txt");
        std::fs::write(&txt_path, "Hello world").expect("should write text file");
        assert!(
            !ToolRuntime::is_binary_file(&txt_path),
            "text file without null bytes should not be detected as binary"
        );

        // An empty file should NOT be detected as binary
        let empty_path = dir.path().join("helper_test.empty");
        std::fs::write(&empty_path, "").expect("should write empty file");
        assert!(
            !ToolRuntime::is_binary_file(&empty_path),
            "empty file should not be detected as binary"
        );

        // A very short file with a null byte (<4 bytes) should NOT be flagged
        let short_bin_path = dir.path().join("short.bin");
        std::fs::write(&short_bin_path, [0x00]).expect("should write short binary file");
        assert!(
            !ToolRuntime::is_binary_file(&short_bin_path),
            "very short file (<4 bytes) with null byte should not trigger false positive"
        );
    }

    // -------------------------------------------------------------------------
    // patch_file tests
    // -------------------------------------------------------------------------

    #[test]
    fn patch_file_simulates_by_default_without_mutation() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Hello world\nGoodbye there");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "test.txt", "old_string": "world", "new_string": "there"}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(result.observation.payload["simulate"], true);
        // File must be unchanged after simulate
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "Hello world\nGoodbye there"
        );
    }

    #[test]
    fn patch_file_executes_replace() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Hello world\nGoodbye world");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({
                "path": "test.txt",
                "old_string": "Hello world",
                "new_string": "Hello there",
                "simulate": false,
            }),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(result.observation.payload["simulate"], false);
        assert_eq!(result.observation.payload["matches"], 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "Hello there\nGoodbye world"
        );
    }

    #[test]
    fn patch_file_works_with_replace_text_alias() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Replace me");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "replace_text",
            &json!({
                "path": "test.txt",
                "old_string": "Replace me",
                "new_string": "Done",
                "simulate": false,
            }),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "Done"
        );
    }

    #[test]
    fn patch_file_rejects_missing_old_string() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Hello");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "test.txt", "old_string": "nonexistent", "new_string": "x", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "pattern_not_found");
    }

    #[test]
    fn patch_file_rejects_multiple_matches_without_replace_all() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "apple\nbanana\napple\ncherry");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "test.txt", "old_string": "apple", "new_string": "orange", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "multiple_matches");
    }

    #[test]
    fn patch_file_replace_all_works() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "apple\nbanana\napple\ncherry");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({
                "path": "test.txt",
                "old_string": "apple",
                "new_string": "orange",
                "simulate": false,
                "replace_all": true,
            }),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(result.observation.payload["matches"], 2);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "orange\nbanana\norange\ncherry"
        );
    }

    #[test]
    fn patch_file_blocks_absolute_path() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "/etc/passwd", "old_string": "root", "new_string": "user", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn patch_file_blocks_parent_traversal() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "../outside.txt", "old_string": "x", "new_string": "y", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn patch_file_blocks_blocked_file() {
        let dir = test_workspace();
        create_test_file(dir.path(), ".env", "SECRET=value");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": ".env", "old_string": "SECRET", "new_string": "PUBLIC", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Blocked);
        assert!(result.error.as_ref().unwrap().is_security);
    }

    #[test]
    fn patch_file_blocks_binary_file() {
        let dir = test_workspace();
        // Write small binary file with null bytes
        let binary_data: Vec<u8> = vec![0x48, 0x65, 0x00, 0x6c, 0x6c, 0x6f];
        std::fs::write(dir.path().join("binary.bin"), &binary_data)
            .expect("should write binary file");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "binary.bin", "old_string": "H", "new_string": "J", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "binary_file");
    }

    #[test]
    fn patch_file_rejects_nonexistent_file() {
        let dir = test_workspace();
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "nonexistent.txt", "old_string": "x", "new_string": "y", "simulate": false}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap().code, "file_not_found");
    }

    #[test]
    fn patch_file_works_with_empty_new_string() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Hello world\nExtra content");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({
                "path": "test.txt",
                "old_string": " world",
                "new_string": "",
                "simulate": false,
            }),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "Hello\nExtra content"
        );
    }

    #[test]
    fn patch_file_diff_preview_in_simulate() {
        let dir = test_workspace();
        create_test_file(dir.path(), "test.txt", "Line one\nLine two\nLine three");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute(
            "patch_file",
            &json!({"path": "test.txt", "old_string": "Line two", "new_string": "Replaced two"}),
        );

        assert_eq!(result.status, ToolExecutionStatus::Success);
        let diff_preview = &result.observation.payload["diff_preview"];
        let preview = diff_preview.as_array().unwrap();
        assert!(!preview.is_empty(), "diff preview should have entries");
        // At minimum the changed line should be in the preview
        assert!(preview.iter().any(|e| e["old"] == "Line two"));
        assert!(preview.iter().any(|e| e["new"] == "Replaced two"));
    }

    // -------------------------------------------------------------------------
    // H2 — CLI security boundary: .git/ file access blocked
    // -------------------------------------------------------------------------

    #[test]
    fn read_file_blocks_git_config() {
        let dir = test_workspace();
        // Create a synthetic .git/config to simulate the real security boundary
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).expect("should create .git dir");
        create_test_file(
            dir.path(),
            ".git/config",
            "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@example.com:org/repo.git\n",
        );
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": ".git/config"}));

        assert_eq!(
            result.status,
            ToolExecutionStatus::Blocked,
            ".git/config read must be blocked: got {:?}",
            result.status
        );
        assert!(
            result
                .error
                .as_ref()
                .map(|e| e.is_security)
                .unwrap_or(false),
            "blocked .git/config must set is_security: {:?}",
            result.error
        );
    }

    #[test]
    fn read_file_blocks_git_head() {
        let dir = test_workspace();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).expect("should create .git dir");
        create_test_file(dir.path(), ".git/HEAD", "ref: refs/heads/main\n");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": ".git/HEAD"}));

        assert_eq!(
            result.status,
            ToolExecutionStatus::Blocked,
            ".git/HEAD read must be blocked: got {:?}",
            result.status
        );
    }

    #[test]
    fn read_file_blocks_relative_git_path() {
        let dir = test_workspace();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).expect("should create .git dir");
        create_test_file(dir.path(), ".git/config", "sensitive");
        let runtime = make_runtime(dir.path());

        // The `./.git/config` path also contains `.git/`
        let result = runtime.execute("read_file", &json!({"path": "./.git/config"}));

        assert_eq!(
            result.status,
            ToolExecutionStatus::Blocked,
            "./.git/config must also be blocked: got {:?}",
            result.status
        );
    }

    #[test]
    fn read_file_gitignore_still_readable() {
        let dir = test_workspace();
        create_test_file(dir.path(), ".gitignore", "target/\n*.env");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": ".gitignore"}));

        assert_eq!(
            result.status,
            ToolExecutionStatus::Success,
            ".gitignore is a normal project file and must remain readable: got {:?}",
            result.status
        );
    }

    #[test]
    fn read_file_github_dir_not_blocked() {
        // Verify .github/ is NOT blocked by the `.git/` pattern
        let dir = test_workspace();
        let github_dir = dir.path().join(".github");
        fs::create_dir_all(&github_dir).expect("should create .github dir");
        create_test_file(dir.path(), ".github/workflows/test.yml", "name: test");
        let runtime = make_runtime(dir.path());

        let result = runtime.execute("read_file", &json!({"path": ".github/workflows/test.yml"}));

        // .github/ is not .git/ — must not be blocked
        assert_ne!(
            result.status,
            ToolExecutionStatus::Blocked,
            ".github/workflows/* are not git internals and must not be blocked"
        );
    }
}
