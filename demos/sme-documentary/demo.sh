#!/usr/bin/env bash
#
# ARPAGONA Agent Core — SME Documentary Assistant Demo (E1)
#
# A realistic SME business document analysis scenario demonstrating
# the full governed cognitive pipeline:
#
#   Objective → Tool Runtime Read → Cognitive Analysis
#   → Governance (DecisionGate → Decision → Audit)
#   → Readback → LLM Interaction Journal
#
# No API server required. No external side effects. Read-only governance.
# All file access is through the bounded read-only Tool Runtime.
#
# Prerequisites: cargo in PATH, workspace compiled (cargo build).
#
# Usage:  from repo root:  bash demos/sme-documentary/demo.sh
#

set -euo pipefail

# ── Locate the demo directory ───────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$REPO_ROOT"

CLI=(cargo run -q --bin arpagona)
SAMPLES_REL="demos/sme-documentary/samples"

# ── Formatting helpers ──────────────────────────────────────────────
header() {
  local title="$1"
  local len="${#title}"
  local bar
  bar=$(printf '═%.0s' $(seq 1 $((len + 6))))
  printf '\n  ╔%s╗\n' "$bar"
  printf '  ║   %s   ║\n' "$title"
  printf '  ╚%s╝\n' "$bar"
}

section() {
  printf '\n  ─── %s ───\n' "$1"
}

JSON=$(mktemp)
cleanup() { rm -f "$JSON"; }
trap cleanup EXIT

run_and_parse() {
  local label="$1"
  local py_code="$2"
  shift 2
  section "$label"
  printf '\n'
  "${CLI[@]}" "$@" 2>/dev/null > "$JSON" || true
  python3 -c "$py_code" 2>&1
}

# ──────────────────────────────────────────────────────────────────────────
#  MAIN DEMO
# ──────────────────────────────────────────────────────────────────────────

printf '\n  Starting in: %s\n' "$REPO_ROOT"
printf '  Demo dir:    %s\n\n' "$SCRIPT_DIR"

header "ARPAGONA Agent Core — SME Documentary Assistant"

printf '\n'
printf '  Scenario: Artisans du Sud — Refonte e-commerce\n'
printf '  A cooperative of 12 craftspeople needs a complete e-commerce\n'
printf '  platform migration with stock management across 3 workshops.\n'
printf '  Budget: 15 000-20 000 €, Deadline: 3 months.\n'
printf '\n'
printf '  Three documents are available:\n'
printf '    • client-brief.md           — SME client brief (French)\n'
printf '    • project-requirements.md    — Functional specifications\n'
printf '    • commercial-proposition.md  — ARPAGONA commercial proposal\n'
printf '\n'
printf '  The demo runs the governed cognitive pipeline entirely offline.\n'
printf '  No API server. No LLM calls by default. Read-only governance.\n'
printf '\n'

# ════════════════════════════════════════════════════════════════════
# PHASE 1 — Tool Runtime: read-only document discovery
# ════════════════════════════════════════════════════════════════════
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 1  —  Tool Runtime: Read-Only Document Discovery    ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'

run_and_parse "1.1 — List available sample documents" "
import json
with open('$JSON') as f:
    d = json.load(f)
entries = d.get('observation', {}).get('payload', {}).get('entries', [])
print(f'  Found {len(entries)} sample documents:')
for e in entries:
    print(f'    \u2022 {e.get(\"name\", \"?\")}')
print(f'  Status: {d.get(\"status\", \"?\")}')
" tool demo list-files "$SAMPLES_REL" --json

run_and_parse "1.2 — Read the client brief" "
import json
with open('$JSON') as f:
    d = json.load(f)
pay = d.get('observation', {}).get('payload', {})
print(f'  File: {pay.get(\"path\", \"?\")}')
print(f'  Lines: {pay.get(\"lines\", 0)}, Chars: {pay.get(\"characters\", 0)}')
print(f'  Status: {d.get(\"status\", \"?\")}')
preview = pay.get('content_preview', '')
first_line = preview.split(chr(10))[0] if preview else '(empty)'
print(f'  First line: {first_line[:80]}')
" tool demo read-file "$SAMPLES_REL/client-brief.md" --json

run_and_parse "1.3 — Read project requirements" "
import json
with open('$JSON') as f:
    d = json.load(f)
pay = d.get('observation', {}).get('payload', {})
preview = pay.get('content_preview', '')
lines = preview.split(chr(10))
print(f'  Lines: {pay.get(\"lines\", 0)}, Chars: {pay.get(\"characters\", 0)}')
print(f'  Headers found:')
for l in lines:
    l = l.strip()
    if l.startswith('## ') or l.startswith('# '):
        print(f'      {l}')
" tool demo read-file "$SAMPLES_REL/project-requirements.md" --json

run_and_parse "1.4 — Search for budget-related keywords" "
import json
with open('$JSON') as f:
    d = json.load(f)
matches = d.get('observation', {}).get('payload', {}).get('matches', [])
print(f'  Found {len(matches)} budget-related matches across documents')
for m in matches[:6]:
    fname = m.get('file', '?')
    snippet = m.get('snippet', '')[:80]
    print(f'    \u2022 {fname}: \"{snippet}...\"')
" tool demo search-text "budget" "$SAMPLES_REL" --json


# ════════════════════════════════════════════════════════════════════
# PHASE 2 — Cognitive Analysis (proposal-only)
# ════════════════════════════════════════════════════════════════════
printf '\n\n'
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 2  —  Cognitive Analysis (Proposal-Only Mode)       ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'
printf '\n'
printf '  Running the cognitive work loop with an SME business objective.\n'
printf '  The objective is derived from what the tool runtime just read.\n'
printf '\n'

run_and_parse "2.1 — Cognitive analysis (business domain)" "
import json
with open('$JSON') as f:
    d = json.load(f)
obj = d.get('objective', {})
title = obj.get('title', obj.get('description', ''))
wm = d.get('working_memory', {})
print(f'  Objective: {title[:80]}')
print(f'  Sensitivity: {wm.get(\"sensitivity_estimate\", \"?\")}')
print(f'  Complexity: {wm.get(\"complexity_estimate\", \"?\")}')
print(f'')
steps = d.get('plan', {}).get('steps', [])
print(f'  Plan steps ({len(steps)}):')
for s in steps:
    print(f'    {s.get(\"order\", \"?\")}. {s.get(\"description\", \"\")[:100]}')
print(f'')
pa = d.get('proposed_next_action', {})
print(f'  Proposed next action:')
print(f'    Kind: {pa.get(\"kind\", \"?\")}')
print(f'    Description: {pa.get(\"description\", \"\")[:120]}')
print(f'    Non-authorizing: {pa.get(\"non_authorizing\", \"?\")}')
print(f'')
ics = d.get('improvement_candidates', [])
print(f'  Improvement candidates ({len(ics)}):')
for ic in ics:
    print(f'    \u2022 {ic.get(\"description\", \"\")[:100]}')
" cognitive run \
  --objective "Évaluer la faisabilité du projet e-commerce Artisans du Sud (budget, périmètre, risques)" \
  --domain business \
  --json

# ════════════════════════════════════════════════════════════════════
# PHASE 3 — Governed Analysis Pipeline
# ════════════════════════════════════════════════════════════════════
printf '\n\n'
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 3  —  Governed Analysis Pipeline                     ║\n'
printf '║  Assessment → FailureInsightCandidates → Decision Gate      ║\n'
printf '║  → Decision → AuditEvent                                    ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'
printf '\n'
printf '  Running with --assess --observe --govern to exercise the\n'
printf '  complete offline governance chain.\n'
printf '\n'

run_and_parse "3.1 — Full governed pipeline" "
import json
with open('$JSON') as f:
    d = json.load(f)

dc = d.get('decision_count', 0)
ac = d.get('audit_event_count', 0)
asd = d.get('assessed', False)
gvd = d.get('governed', False)
obs_count = len(d.get('cognitive_observations', []))
warn = d.get('governance_warning', '')

print(f'  decision_count:      {dc}')
print(f'  audit_event_count:   {ac}')
print(f'  assessed:            {asd}')
print(f'  governed:            {gvd}')
print(f'  cognitive_observations: {obs_count}')
print(f'')

if dc > 0:
    print(f'  Governance chain produced decisions and audit events')
else:
    print(f'  No decisions produced (expected in offline readback mode)')
print(f'')

for r in d.get('governance_results', []):
    pa = r.get('proposed_action', {})
    dec = r.get('decision', {})
    ae = r.get('audit_event', {})
    print(f'  ProposedAction:   {pa.get(\"action_type\", \"?\")} (risk: {pa.get(\"risk_level\", \"?\")})')
    print(f'  Decision:         {dec.get(\"status\", \"?\")} (id: {dec.get(\"id\", \"?\")})')
    print(f'  AuditEvent:       {ae.get(\"event_type\", \"?\")} (actor: {ae.get(\"actor\", \"?\")})')
    print(f'')
" cognitive run \
  --objective "Analyser le brief client Artisans du Sud et proposer un plan d'action pour la migration e-commerce" \
  --domain business \
  --assess \
  --observe \
  --govern \
  --json


# ════════════════════════════════════════════════════════════════════
# PHASE 4 — Operator Readback Surfaces
# ════════════════════════════════════════════════════════════════════
printf '\n'
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 4  —  Operator Readback Surfaces                     ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'
printf '\n'
printf '  Inspecting status, audit events, and LLM journal.\n'
printf '\n'

run_and_parse "4.1 — System status readback" "
import json
with open('$JSON') as f:
    d = json.load(f)
print(f'  API health:          {str(d.get(\"api_health\", \"?\"))[:60]}')
print(f'  Task count:          {d.get(\"task_count\", \"?\")}')
print(f'  Decision count:      {d.get(\"decision_count\", \"?\")}')
print(f'  Audit event count:   {d.get(\"audit_event_count\", \"?\")}')
print(f'  Needs human approval: {d.get(\"needs_human_approval_count\", \"?\")}')
print(f'  Warning:             {str(d.get(\"warning\", \"\"))[:80]}')
" status --json

run_and_parse "4.2 — LLM interaction journal (initial)" "
import json
with open('$JSON') as f:
    d = json.load(f)
entries = d.get('entries', [])
if entries:
    print(f'  Found {d.get(\"total_entries\", len(entries))} LLM journal entries')
    for e in entries[:3]:
        eid = e.get('id', '?')
        ptype = e.get('interaction_type', '?')
        pprovider = e.get('provider', '?')
        print(f'    \u2022 [{eid}] {ptype} — provider: {pprovider}')
else:
    print(f'  No LLM journal entries yet — run with --llm to populate')
" llm journal --json

run_and_parse "4.3 — (Optional) LLM-assisted run (mock provider)" "
import json
with open('$JSON') as f:
    d = json.load(f)
synth = d.get('llm_synthesis', '')
if synth:
    print(f'  Synthesis: {synth[:200]}...')
    print(f'')
    print(f'  LLM journal now populated. View with:')
    print(f'    arpagona llm journal')
    print(f'    arpagona llm journal --json')
else:
    print(f'  No LLM synthesis produced (--llm may require a provider)')
" cognitive run \
  --objective "Identifier les risques principaux du projet Artisans du Sud" \
  --llm \
  --provider mock \
  --json

run_and_parse "4.4 — LLM journal readback (now populated)" "
import json
with open('$JSON') as f:
    d = json.load(f)
entries = d.get('entries', [])
total = d.get('total_entries', len(entries))
print(f'  LLM journal entries: {total}')
for i, e in enumerate(entries[-5:]):
    provider = e.get('provider', '?')
    model = str(e.get('model', 'unnamed'))
    psum = e.get('prompt_summary', '')[:60]
    rsum = e.get('response_summary', '')[:60]
    print(f'    [{i+1}] Provider: {provider}, Model: {model}')
    print(f'        Prompt: {psum}')
    print(f'        Response: {rsum}')
" llm journal --json


# ════════════════════════════════════════════════════════════════════
# Summary
# ════════════════════════════════════════════════════════════════════
printf '\n\n'
header "Demo Summary — E1 SME Documentary Assistant"
printf '\n'
cat <<'SUMMARY'
  The complete governed cognitive pipeline was demonstrated:

  PHASE 1  Tool Runtime (read-only)
    Discovered 3 sample documents, read contents, searched keywords

  PHASE 2  Cognitive Analysis (proposal-only)
    Classified domain, formed working memory, generated plan steps,
    proposed non-authorizing next action

  PHASE 3  Governed Analysis Pipeline
    Assessment -> FailureInsightCandidates -> Decision Gate
    -> Decision + AuditEvent production

  PHASE 4  Operator Readback
    System status, LLM journal persistence and readback
    All output in structured JSON for programmatic consumption

  Safe boundaries preserved:
    Read-only + workspace-scoped tools, non-authorizing output,
    governance chain, no external effects

  Next step: run with --llm --provider ollama for real model synthesis.

SUMMARY
printf '\n'
