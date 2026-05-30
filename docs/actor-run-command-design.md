# ARPAGONA Actor Run — Command Design Spec

> Prepared by DEEP per GONA direction (2026-05-30).
> GO for implementation granted by GONA and Thibaud.
> Implemented on branch: `feat/actor-run-command`
> PR opened; awaiting merge.
>
> **Decision Gate wording (for Thibaud approval):**
> - GO/NO-GO requested: implement ONLY top-level `arpagona actor run` with deterministic parser and four bounded file tools.
> - Allowed: `append_file`/`read_file`/`list_files`/`search_text`, workspace-bounded, simulation-first, `--approve` for append execution.
> - Forbidden: LLM interpretation, shell, network, browser, secrets, delete, scheduler, status queues/dashboard/API expansion.

## 1. Objective

Add a top-level `arpagona actor run "<task>"` command that proves one vertical governed loop: parse natural language intent -> risk label -> Decision Gate -> simulation -> explicit --approve -> execution -> readback -> observation/journal.

This is the productized evolution of the demo-only `tool demo actor-lab` path (PR #232). Instead of hardcoded append_file, it uses deterministic NL parsing to route to one of four bounded local file tools.

## 2. Command Interface

```text
arpagona actor run "<natural language task>"
  --approve           # Skip simulation-only mode, execute approved actions
  --json              # Structured JSON output
  --workspace <path>  # Workspace root (default: current directory)
```

### Examples

```bash
# Simulation-only (recommended first run)
arpagona actor run "append a note about Q3 planning to docs/notes.md"

# Execute after reviewing simulation
arpagona actor run "append a note about Q3 planning to docs/notes.md" --approve

# Read-only
arpagona actor run "read docs/notes.md"

# List and search
arpagona actor run "list files in docs/"
arpagona actor run "search for milestone in docs/"

# JSON structured output
arpagona actor run "append hello to test.txt" --json
```

## 3. CLI Hierarchy Change

Add new variant to `Command` enum in `crates/cli/src/main.rs`:

```rust
#[derive(Debug, Subcommand)]
enum Command {
    // ... existing variants ...
    /// Run a governed local actor mission from a natural language task.
    /// Uses deterministic (no LLM) parsing to route to bounded file tools.
    /// Simulates first; execution requires --approve.
    Actor(ActorCommand),
}

#[derive(Debug, Args)]
struct ActorCommand {
    #[command(subcommand)]
    command: ActorSubcommand,
}

#[derive(Debug, Subcommand)]
enum ActorSubcommand {
    /// Parse a natural language task and run it through the governed
    /// simulation -> approval -> execution -> readback loop.
    Run(ActorRunArgs),
}
```

### ActorRunArgs

```rust
#[derive(Debug, Args)]
struct ActorRunArgs {
    /// Natural language task description.
    /// Examples:
    ///   "append meeting notes to docs/log.md"
    ///   "read docs/README.md"
    ///   "list files in src/"
    ///   "search for FIXME in lib/"
    task: String,

    /// Explicitly approve the simulated proposal and execute.
    #[arg(long)]
    approve: bool,

    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,

    /// Workspace root for file tools (default: current directory).
    #[arg(long, default_value = ".")]
    workspace: String,
}
```

## 4. NL -> Intent Parsing (Deterministic)

### Core Types

```rust
struct ActorIntent {
    tool: String,              // "append_file" | "read_file" | "list_files" | "search_text"
    arguments: serde_json::Value,
    risk_level: RiskLevel,     // Informational for reads, Low for writes
    rationale: String,
    display_summary: String,   // Human-readable description of what was parsed
}

enum IntentParseError {
    UnrecognizedTask(String),
    MissingArgument(String),
    AmbiguousTask(String),
}
```

### Pattern matching rules

Ordered by priority (first match wins):

| Pattern | Tool | Argument extraction | Risk |
|---------|------|-------------------|------|
| `append <content> to <path>` | `append_file` | content: matched text, path: file path, create_parent_dirs: true, create_if_missing: true, simulate: (dynamic) | Low |
| `append <content> at <path>` | `append_file` | (same as above) | Low |
| `add <content> to <path>` | `append_file` | (same as above) | Low |
| `read <path>` | `read_file` | path: extracted path | Informational |
| `show <path>` | `read_file` | path: extracted path | Informational |
| `list files` | `list_files` | path: workspace root | Informational |
| `list files in <path>` | `list_files` | path: extracted path | Informational |
| `list directory <path>` | `list_files` | path: extracted path | Informational |
| `search for <pattern> in <path>` | `search_text` | pattern: extracted text, path: extracted path | Informational |
| `search <pattern> in <path>` | `search_text` | (same) | Informational |
| `find <pattern> in <path>` | `search_text` | (same) | Informational |
| `search <pattern>` | `search_text` | pattern: extracted text, path: workspace root | Informational |
| `find <pattern>` | `search_text` | pattern: extracted text, path: workspace root | Informational |

### Implementation approach

Use standard `str` operations (no external regex dependency):

```rust
fn parse_intent(task: &str) -> Result<ActorIntent, IntentParseError> {
    let task = task.trim();
    let lower = task.to_lowercase();

    // Append patterns: "append <content> to <path>", "append <content> at <path>", "add <content> to <path>"
    if let Some(rest) = lower.strip_prefix("append ")
        .or_else(|| lower.strip_prefix("add "))
    {
        if let Some(to_pos) = rest.rfind(" to ").or_else(|| rest.rfind(" at ")) {
            let content = &task["append ".len()..to_pos + task.len() - rest.len()];
            let path = &task[to_pos + task.len() - rest.len() + 4..];
            // "to" could appear in content; use rfind to match last occurrence
            return Ok(ActorIntent {
                tool: "append_file".to_owned(),
                arguments: serde_json::json!({
                    "content": content.trim(),
                    "path": path.trim(),
                    "create_parent_dirs": true,
                    "create_if_missing": true,
                }),
                risk_level: RiskLevel::Low,
                rationale: format!("Append content to file at {path}"),
                display_summary: format!("Append to {path}"),
            });
        }
    }

    // Read patterns: "read <path>", "show <path>"
    if let Some(path) = lower.strip_prefix("read ")
        .or_else(|| lower.strip_prefix("show "))
    {
        return Ok(ActorIntent {
            tool: "read_file".to_owned(),
            arguments: serde_json::json!({ "path": path.trim() }),
            risk_level: RiskLevel::Informational,
            rationale: format!("Read file at {path}"),
            display_summary: format!("Read {path}"),
        });
    }

    // List patterns: "list files", "list files in <path>", "list directory <path>"
    if lower.starts_with("list files") || lower.starts_with("list directory") {
        let path = lower.strip_prefix("list files in ")
            .or_else(|| lower.strip_prefix("list directory "))
            .map(|p| p.trim())
            .unwrap_or("");
        return Ok(ActorIntent {
            tool: "list_files".to_owned(),
            arguments: serde_json::json!({ "path": path }),
            risk_level: RiskLevel::Informational,
            rationale: format!("List files in {path}"),
            display_summary: if path.is_empty() {
                "List files".to_owned()
            } else {
                format!("List files in {path}")
            },
        });
    }

    // Search patterns: "search for <pattern> in <path>", "search <pattern> in <path>", etc.
    if lower.starts_with("search ") || lower.starts_with("find ") {
        let after_keyword = if lower.starts_with("search for ") {
            &task[11..]
        } else if lower.starts_with("search ") {
            &task[7..]
        } else {
            &task[5..]
        };
        if let Some(in_pos) = after_keyword.rfind(" in ") {
            let pattern = after_keyword[..in_pos].trim();
            let path = after_keyword[in_pos + 4..].trim();
            return Ok(ActorIntent {
                tool: "search_text".to_owned(),
                arguments: serde_json::json!({
                    "pattern": pattern,
                    "path": path,
                }),
                risk_level: RiskLevel::Informational,
                rationale: format!("Search for '{pattern}' in {path}"),
                display_summary: format!("Search for '{pattern}'"),
            });
        }
        // No "in" clause — search whole workspace
        return Ok(ActorIntent {
            tool: "search_text".to_owned(),
            arguments: serde_json::json!({
                "pattern": after_keyword.trim(),
                "path": "",
            }),
            risk_level: RiskLevel::Informational,
            rationale: format!("Search for '{}' in workspace", after_keyword.trim()),
            display_summary: format!("Search for '{}'", after_keyword.trim()),
        });
    }

    Err(IntentParseError::UnrecognizedTask(task.to_owned()))
}
```

Notes:
- This parser uses only `str::starts_with`, `str::strip_prefix`, `str::rfind`, and string slicing — no external crate needed.
- Uses `rfind(" to ")` (rightmost occurrence) to handle content that contains " to " naturally (e.g., "append 'welcome to the team' to docs/notes.md").
- Case-insensitive matching uses `to_lowercase()` on the input, but extracts original-cased content for arguments.
- Priority order: append > read > list > search (first match wins, same as before).

## 5. Governance Loop (Phase Sequence)

### Phase 1 — Intent -> ProposedAction

1. Parse NL task -> ActorIntent
2. Create ToolCallIntent with tool, arguments, rationale, risk_level
3. Run `govern_tool_call(&intent, &[Permission::ProposeToolUse])` -> (decision, proposed_action)

### Phase 2 — Simulation/dry-run

1. If decision is Approved: run tool with `"simulate": true` in arguments
2. If simulation succeeds: present result to user (or prepare for approval)
3. If simulation fails: report error, do not offer execution path

> **Read-only tool behavior (read_file, list_files, search_text):** These tools have no meaningful mutation to simulate. They pass through the same ProposeToolUse Decision Gate for governance visibility. Their "simulation" is the actual read operation — the output is clearly labeled **informational / non-authorizing**. The user can see results without needing `--approve`. No file mutation occurs, and no separate execution step is required after simulation.

### Phase 3 — Explicit approval

1. If `--approve` is present AND decision is Approved AND simulation succeeded:
   - Re-run tool with `"simulate": false`
2. If `--approve` is NOT present:
   - Print "Approval: missing — simulation only. Rerun with --approve to execute."
   - Include exact command the user should run

### Phase 4 — Readback + Observation

1. After execution (if applicable): run read_file on the modified path
2. Generate CognitiveObservation from the tool execution result
3. Run assess_observation for completeness/usefulness assessment

### Phase 5 — Journaling

1. Record the full trace in the LLM Journal (via `global_llm_journal().lock().unwrap().add_direct_tool_call(...)`)
2. Include: parsed intent, tool, arguments, decision, simulation result, execution result, readback, observation, assessment

## 6. Output Format

### Human-readable (default)

```text
[WARNING — Actor Run is a sandboxed governed local mission. Simulation first; execution requires --approve.]

=== Actor Run ===
Task: "append meeting notes to docs/log.md"

--- 1. Intent interpretation ---
Tool:   append_file
Path:   docs/log.md
Content: "Meeting notes\n"
Risk:   Low

--- 2. Decision Gate ---
Decision:    Approved
Decision ID: dg-abc123
Reason:     Low risk + low permission tool within workspace

--- 3. Simulation / diff preview ---
Status:  Success
Summary: Would append 1 line to docs/log.md

--- 4. Approval path ---
Approval:    missing — simulation only
Next step:  rerun with --approve to execute
  arpagona actor run "append meeting notes to docs/log.md" --approve

--- 5. Execution + readback ---
Execution:  not run

--- 6. Observation / audit ---
Observation ID: obs-xyz
Journal Entry:  je-456
```

### JSON output (--json)

```json
{
  "command": "actor-run",
  "task": "append meeting notes to docs/log.md",
  "intent": {
    "tool": "append_file",
    "rationale": "Actor run: append meeting notes to docs/log.md",
    "risk_level": "Low"
  },
  "decision": {
    "id": "dg-abc123",
    "status": "Approved",
    "reason": "Low risk + low permission tool within workspace"
  },
  "simulation_result": {
    "status": "Success",
    "output_summary": "Would append 1 line to docs/log.md"
  },
  "approval_state": "simulation_only_waiting_for_explicit_approval",
  "execution_result": null,
  "readback_result": null,
  "cognitive_observation": null,
  "journal_entry_id": "je-456"
}
```

## 7. Files to Modify

| File | Change |
|------|--------|
| `crates/cli/src/main.rs` | Add `Actor` variant to `Command` enum, `ActorCommand`, `ActorSubcommand`, `ActorRunArgs`, `parse_intent()`, `actor_run()` function, dispatch wiring, warning constant |
| `crates/cli/src/main.rs` (test module) | Add tests: `parse_intent` unit tests, `actor_run` parse tests, smoke path for approved append, unapproved simulation, unrecognized task error, each tool variant |

No new files needed. All logic lives in `crates/cli/src/main.rs` as a new module section.

## 8. What Is NOT Included (Explicitly Deferred)

| Feature | Rationale |
|---------|-----------|
| `arpagona actor status` | Not required for first vertical loop. Defer unless it falls out trivially from journal/readback state. |
| LLM-based intent parsing | Deterministic only. LLM interpretation would introduce non-determinism and governance gaps. |
| Shell, network, browser, secrets, delete, scheduler tools | Explicitly excluded by GONA. |
| Status queues, API endpoints | Not needed for the first product increment. |
| Web dashboard | Will be deferred to D-series after D1-D3. |
| `write_file` tool (beyond append) | Not in the allowed tool set for first increment. |
| `copy_file`, `move_file`, `mkdir`, `patch_file` | Not in the allowed tool set for first increment. |

## 9. Test Plan

All tests are CLI parse tests and in-process unit tests (no external dependencies, no network):

1. **Intent parsing tests** (unit tests):
   - `parse_intent_append_text_to_path` — "append hello to docs/test.txt"
   - `parse_intent_read_path` — "read docs/notes.md"
   - `parse_intent_read_no_args_path` — "show docs/notes.md"
   - `parse_intent_list_files_root` — "list files"
   - `parse_intent_list_files_in_path` — "list files in src/"
   - `parse_intent_search_for_pattern_in_path` — "search for FIXME in lib/"
   - `parse_intent_search_pattern_in_path` — "find TODO in src/"
   - `parse_intent_unrecognized` — "do something crazy"
   - `parse_intent_case_insensitive` — "APPEND hello TO docs/test.txt"
   - `parse_intent_append_content_multi_word` — "append my multi-word note to docs/log.md"

2. **CLI parse tests** (parse_from tests):
   - `actor_run_parses_simple_task` — `cargo run -- actor run "append hello to test.txt"`
   - `actor_run_parses_with_approve` — `cargo run -- actor run "read docs/x" --approve`
   - `actor_run_parses_with_json` — `cargo run -- actor run "list files" --json`
   - `actor_run_parses_with_workspace` — `cargo run -- actor run "search for x" --workspace /tmp/scratch`

3. **Smoke path test** (calls `actor_run()` in-process):
   - `actor_run_simulate_only_does_not_execute` — verify simulation succeeds but no file created
   - `actor_run_approved_append_creates_file_and_readback` — verify --approve creates file and shows readback
   - `actor_run_unrecognized_task_returns_error` — verify error path

4. **Parser safety tests** (new — added per GONA correction):
   - `parse_intent_append_content_containing_to` — "append 'welcome to the team' to docs/notes.md" — content must include the first "to"
   - `parse_intent_rejects_parent_traversal_path` — "append x to ../etc/passwd" — must be caught by ToolRuntime
   - `parse_intent_rejects_absolute_path` — "read /etc/passwd" — must be caught by ToolRuntime
   - `parse_intent_shell_like_task_rejected` — "run rm -rf /" — returns UnrecognizedTask, not misinterpreted as a tool
   - `parse_intent_unrecognized_ambiguity` — "append to docs" — missing content; should return UnrecognizedTask or an appropriate error because "append" is followed by "to" with no content gap
   - `parse_intent_ambiguous_content_and_path` — "read file.md to review it" — " to " could split the string, but "read" returns the full remainder as path (no "to" parsing for read)

## 10. Estimated Scope

- **New code**: ~250-350 lines (intent parsing: ~100 lines, actor_run function: ~200 lines, dispatch wiring: ~30 lines)
- **Tests**: ~200-300 lines
- **Files changed**: 1 file (`crates/cli/src/main.rs`)
- **No new crates, no new dependencies** (uses `std::str` operations for parsing; DecisionGate, ToolRuntime, CognitiveObservation, LlmJournal all already imported)

## 11. Risk Assessment

| Risk | Mitigation |
|------|------------|
| NL parsing too brittle for useful tasks | Start with 4 clear patterns for 4 tools. Expand only after feedback. Priority: correct rejection > false match. |
| Workspace path escaping | Already handled by existing ToolRuntime safety boundary (rejects ../, .env, absolute paths outside workspace). |
| Regression on existing tests | Full `cargo test --workspace` run before push. |
| Merge conflicts with other in-flight branches | Branch from current main. If others merge first, rebase. |
| Duplicate of existing `tool demo` path | The `tool demo actor-lab` path stays as a demo. `actor run` is the productized evolution. After `actor run` is stable, `tool demo actor-lab` can be kept for backward compat or deprecated with a pointer to `actor run`. |

## 12. Verification Commands

```bash
cargo fmt -- --check
cargo check
cargo test --workspace
# Manual smoke (relative paths resolve against workspace):
cargo run -- actor run "append test note to docs/actor-test.md"
cargo run -- actor run "append test note to docs/actor-test.md" --approve --json
cargo run -- actor run "read docs/actor-test.md"
cargo run -- actor run "list files"
cargo run -- actor run "search for note in src/"
```
