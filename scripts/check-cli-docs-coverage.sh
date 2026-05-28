#!/usr/bin/env bash
# CLI Docs Coverage Check
# Lightweight validation that docs/cli.md covers all top-level commands
# from `arpagona --help`.
#
# Part of DV-2026-05-26-002 fix: prevent docs drift.
#
# Usage:
#   ./scripts/check-cli-docs-coverage.sh
#
# Exit code: 0 if all commands are covered, 1 if gaps found.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCS_FILE="$PROJECT_DIR/docs/cli.md"

if [ ! -f "$DOCS_FILE" ]; then
    echo "ERROR: $DOCS_FILE not found"
    exit 1
fi

# Extract top-level command names from --help output
# Lines look like: "  serve      Run the local API server through cargo"
# Parse the first word after leading whitespace (the command name),
# skip "help" (trivial, always documented by clap).
COMMANDS=$(
    cargo run -q --bin arpagona -- --help 2>/dev/null \
        | sed -n '/^Commands:/,/^$/p' \
        | tail -n +2 \
        | grep -v '^[[:space:]]*$' \
        | sed 's/^[[:space:]]*//' \
        | awk '{print $1}' \
        | grep -v '^help$'
)

# Map of command names -> patterns that identify their docs section.
# This handles descriptive French headings where the heading title
# doesn't match the command name literally.
declare -A CMD_PATTERNS
CMD_PATTERNS["serve"]="Lancer.l.API|### serve"
CMD_PATTERNS["executor"]="### Executor|### executor"
CMD_PATTERNS["mcp-server"]="### MCP Server|### mcp-server|Serveur MCP"
CMD_PATTERNS["chat"]="Interface.terminal.interactive|### chat"
CMD_PATTERNS["health"]="### Health|### health"
CMD_PATTERNS["status"]="### Status|### status"
CMD_PATTERNS["auth"]="### Auth|### auth|Authentification|OpenAI auth"
CMD_PATTERNS["task"]="Créer.une.tâche|### task|### Task|Gestion.des.tâches"
CMD_PATTERNS["action"]="Proposer.une.action|### action|### Action"
CMD_PATTERNS["agent"]="Agent.Proposer|### agent|### Agent"
CMD_PATTERNS["audit"]="### Audit|### audit|Lister.l.audit|Résumé.d.audit"
CMD_PATTERNS["insight"]="### Insight|### insight"
CMD_PATTERNS["memory"]="### Graph Memory|### memory|Mémoire|Démos Memory"
CMD_PATTERNS["tool"]="### Tool Runtime|### tool|Tool Runtime"
CMD_PATTERNS["cognitive"]="### Cognitive|### cognitive|Cognitive Work Loop"
CMD_PATTERNS["mcp-governance-audit"]="### MCP Governance Audit|MCP Governance Audit|mcp-governance-audit"
CMD_PATTERNS["llm"]="### LLM|Journal d.interaction LLM|llm journal"

# For commands not in the map above, fall back to a simple grep for "### <cmd>"
MISSING=()
for cmd in $COMMANDS; do
    pattern="${CMD_PATTERNS[$cmd]:-}"
    if [ -n "$pattern" ]; then
        # Use extended regex to match any of the patterns
        if ! grep -qiE "$pattern" "$DOCS_FILE" 2>/dev/null; then
            MISSING+=("$cmd")
        fi
    else
        # Fallback: just look for any heading mentioning the command
        if ! grep -qi "^###.*${cmd}" "$DOCS_FILE" 2>/dev/null && \
           ! grep -qi "^#.*${cmd}[[:space:]\\\"]" "$DOCS_FILE" 2>/dev/null; then
            MISSING+=("$cmd")
        fi
    fi
done

if [ ${#MISSING[@]} -eq 0 ]; then
    echo "✅ All CLI commands are covered in docs/cli.md"
    exit 0
else
    echo "❌ Missing docs for CLI commands: ${MISSING[*]}"
    echo ""
    for cmd in "${MISSING[@]}"; do
        echo "  - $cmd"
    done
    exit 1
fi
