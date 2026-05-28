#!/usr/bin/env bash
#
# ARPAGONA Agent Core — SME Documentary Assistant Demo — LLM Variant (E1)
#
# A realistic SME business document analysis scenario demonstrating the
# full governed cognitive pipeline WITH LLM-assisted synthesis.
#
#   Objective → Tool Runtime Read → Cognitive Analysis (with --llm)
#   → Governance (DecisionGate → Decision → Audit)
#   → LLM Journal Readback → Operator Readback
#
# Three modes:
#   bash demo-llm.sh                    (default: --provider mock, deterministic)
#   bash demo-llm.sh mock               (explicit mock provider)
#   bash demo-llm.sh ollama             (real local model via Ollama)
#   bash demo-llm.sh both               (mock first, then ollama comparison)
#
# Prerequisites:
#   - cargo in PATH, workspace compiled (cargo build)
#   - "both" or "ollama" mode only: ollama must be running with qwen3.5:9b
#
# No API server required. No external side effects. Read-only governance.
#

set -euo pipefail

# ── Mode selection ────────────────────────────────────────────────────
MODE="${1:-mock}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

CLI=(cargo run -q --bin arpagona)
SAMPLES_REL="demos/sme-documentary/samples"

# ── Formatting helpers ────────────────────────────────────────────────
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

# ── Mode banner ───────────────────────────────────────────────────────
case "$MODE" in
  mock)
    PROVIDER="mock"
    PROVIDER_LABEL="Mock provider (deterministic)"
    ;;
  ollama)
    PROVIDER="ollama"
    PROVIDER_LABEL="Ollama local model (qwen3.5:9b)"
    ;;
  both)
    PROVIDER="mock"
    PROVIDER_LABEL="Mock provider first, then Ollama comparison"
    ;;
  *)
    echo "  Unknown mode: $MODE (use: mock, ollama, both)"
    exit 1
    ;;
esac

# ═════════════════════════════════════════════════════════════════════
printf '\n'
header "ARPAGONA Agent Core — SME Documentary Assistant"
printf '\n'
printf '  Mode:        %s\n' "$PROVIDER_LABEL"
printf '  Scenario:    Artisans du Sud — Refonte e-commerce\n'
printf '  Repository:  %s\n' "$REPO_ROOT"
printf '\n'
printf '  This demo runs the complete governed cognitive pipeline WITH\n'
printf '  LLM-assisted synthesis integrated into every cognitive phase.\n'
printf '\n'
printf '  Documents available:\n'
printf '    • client-brief.md\n'
printf '    • project-requirements.md\n'
printf '    • commercial-proposition.md\n'
printf '\n'


# ═════════════════════════════════════════════════════════════════════
# PHASE 1 — Tool Runtime: read-only document discovery
# ═════════════════════════════════════════════════════════════════════
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
    print(f'    • {e.get(\"name\", \"?\")}')
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
    print(f'    • {fname}: \"{snippet}...\"')
" tool demo search-text "budget" "$SAMPLES_REL" --json


# ═════════════════════════════════════════════════════════════════════
# PHASE 2 — Cognitive Analysis with LLM synthesis
# ═════════════════════════════════════════════════════════════════════
printf '\n\n'
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 2  —  Cognitive Analysis (LLM-assisted)              ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'
printf '\n'
printf '  Running cognitive work loop with --llm --provider %s\n' "$PROVIDER"
printf '  The LLM enriches working memory, plan, and proposals with\n'
printf '  structured model synthesis.\n'
printf '\n'

run_and_parse "2.1 — LLM-assisted cognitive analysis" "
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
synth = d.get('llm_synthesis', '')
if synth:
    print(f'  LLM Synthesis ({len(synth)} chars):')
    for line in synth.split(chr(10)):
        print(f'    {line}')
print(f'')
prov = d.get('llm_provider', '?')
if isinstance(prov, dict):
    pid = prov.get('provider_id', '?')
    mod = prov.get('model', '?')
    print(f'  Provider: {pid} / {mod}')
else:
    print(f'  Provider: {prov}')
" cognitive run \
  --objective "Évaluer la faisabilité du projet e-commerce Artisans du Sud (budget, périmètre, risques)" \
  --domain business \
  --llm \
  --provider "$PROVIDER" \
  --json


# ═════════════════════════════════════════════════════════════════════
# PHASE 3 — Governed Analysis Pipeline with LLM synthesis
# ═════════════════════════════════════════════════════════════════════
printf '\n\n'
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 3  —  Governed Analysis Pipeline (LLM-assisted)     ║\n'
printf '║  Assessment → FailureInsightCandidates → Decision Gate     ║\n'
printf '║  → Decision → AuditEvent + LLM Synthesis                   ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'
printf '\n'
printf '  Running with --assess --observe --govern AND --llm\n'
printf '  This exercises the complete offline governance chain with\n'
printf '  LLM-enriched synthesis at every stage.\n'
printf '\n'

run_and_parse "3.1 — Full governed pipeline (LLM)" "
import json
with open('$JSON') as f:
    d = json.load(f)

dc = d.get('decision_count', 0)
ac = d.get('audit_event_count', 0)
asd = d.get('assessed', False)
gvd = d.get('governed', False)
obs_count = len(d.get('cognitive_observations', []))
warn = d.get('governance_warning', '')
synth = d.get('llm_synthesis', '')

print(f'  decision_count:      {dc}')
print(f'  audit_event_count:   {ac}')
print(f'  assessed:            {asd}')
print(f'  governed:            {gvd}')
print(f'  cognitive_observations: {obs_count}')
if synth:
    print(f'  LLM synthesis:       {len(synth)} chars')
print(f'')
if dc > 0:
    print(f'  Governance chain produced decisions and audit events')
else:
    print(f'  No decisions produced (expected in offline readback mode)')

for r in d.get('governance_results', []):
    pa = r.get('proposed_action', {})
    dec = r.get('decision', {})
    ae = r.get('audit_event', {})
    print(f'')
    print(f'  ProposedAction:   {pa.get(\"action_type\", \"?\")} (risk: {pa.get(\"risk_level\", \"?\")})')
    print(f'    Rationale:      {str(pa.get(\"rationale\", \"\"))[:100]}')
    print(f'  Decision:         {dec.get(\"status\", \"?\")} (id: {dec.get(\"id\", \"?\")})')
    print(f'  AuditEvent:       {ae.get(\"event_type\", \"?\")} (actor: {ae.get(\"actor\", \"?\")})')

if synth:
    print(f'')
    print(f'  === LLM Synthesis ===')
    for line in synth.split(chr(10)):
        print(f'  {line}')
" cognitive run \
  --objective "Analyser le brief client Artisans du Sud et proposer un plan d'action pour la migration e-commerce" \
  --domain business \
  --assess \
  --observe \
  --govern \
  --llm \
  --provider "$PROVIDER" \
  --json


# ═════════════════════════════════════════════════════════════════════
# PHASE 4 — Operator Readback + LLM Journal
# ═════════════════════════════════════════════════════════════════════
printf '\n\n'
printf '╔══════════════════════════════════════════════════════════════╗\n'
printf '║  PHASE 4  —  Operator Readback + LLM Journal                ║\n'
printf '╚══════════════════════════════════════════════════════════════╝\n'
printf '\n'
printf '  Inspecting system status, LLM journal, and governance\n'
printf '  readback surfaces after the LLM-assisted run.\n'
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

run_and_parse "4.2 — LLM interaction journal" "
import json
with open('$JSON') as f:
    d = json.load(f)
entries = d.get('entries', [])
total = d.get('total_entries', len(entries))
print(f'  LLM journal entries: {total}')
for i, e in enumerate(entries[-5:]):
    n = e.get('id', '?')
    provider = e.get('provider', '?')
    model = str(e.get('model', 'unnamed'))
    ps = e.get('prompt_summary', '')[:60]
    rs = e.get('response_summary', '')[:60]
    print(f'    [{i+1}] #{n} Provider: {provider}, Model: {model}')
    print(f'        Prompt: {ps}')
    print(f'        Response: {rs}')
    print(f'')
" llm journal --limit 10 --json


# ═════════════════════════════════════════════════════════════════════
# PHASE 5 (optional) — Ollama comparison run
# ═════════════════════════════════════════════════════════════════════
if [ "$MODE" = "both" ]; then
  printf '\n\n'
  printf '╔══════════════════════════════════════════════════════════════╗\n'
  printf '║  PHASE 5  —  Comparison: Ollama synthesis (qwen3.5:9b)     ║\n'
  printf '╚══════════════════════════════════════════════════════════════╝\n'
  printf '\n'
  printf '  Running the same cognitive analysis with the real local\n'
  printf '  model for comparison against the mock provider.\n'
  printf '\n'

  run_and_parse "5.1 — Ollama-assisted cognitive analysis" "
import json
with open('$JSON') as f:
    d = json.load(f)
obj = d.get('objective', {})
title = obj.get('title', obj.get('description', ''))
print(f'  Objective: {title[:80]}')
print(f'')
steps = d.get('plan', {}).get('steps', [])
print(f'  Plan steps ({len(steps)}):')
for s in steps:
    print(f'    {s.get(\"order\", \"?\")}. {s.get(\"description\", \"\")[:100]}')
print(f'')
synth = d.get('llm_synthesis', '')
if synth:
    print(f'  LLM Synthesis ({len(synth)} chars, local model):')
    for line in synth.split(chr(10)):
        print(f'    {line}')
print(f'')
prov = d.get('llm_provider', '?')
if isinstance(prov, dict):
    pid = prov.get('provider_id', '?')
    mod = prov.get('model', '?')
    print(f'  Provider: {pid} / {mod}')
else:
    print(f'  Provider: {prov}')
" cognitive run \
    --objective "Évaluer la faisabilité du projet e-commerce Artisans du Sud (budget, périmètre, risques)" \
    --domain business \
    --llm \
    --provider ollama \
    --json

  run_and_parse "5.2 — Ollama governed pipeline" "
import json
with open('$JSON') as f:
    d = json.load(f)
dc = d.get('decision_count', 0)
ac = d.get('audit_event_count', 0)
synth = d.get('llm_synthesis', '')
print(f'  decision_count:      {dc}')
print(f'  audit_event_count:   {ac}')
print(f'')
if synth:
    print(f'  LLM Synthesis (local model):')
    for line in synth.split(chr(10)):
        print(f'    {line}')
" cognitive run \
    --objective "Analyser le brief client Artisans du Sud et proposer un plan d'action pour la migration e-commerce" \
    --domain business \
    --assess \
    --observe \
    --govern \
    --llm \
    --provider ollama \
    --json
fi


# ═════════════════════════════════════════════════════════════════════
# Summary
# ═════════════════════════════════════════════════════════════════════
printf '\n\n'
header "Demo Summary — E1 SME Documentary Assistant (LLM variant)"
printf '\n'

case "$MODE" in
  mock)
    echo '  Mode:              Mock provider (deterministic)'
    ;;
  ollama)
    echo '  Mode:              Ollama local model (qwen3.5:9b)'
    ;;
  both)
    echo '  Mode:              Both (mock + ollama comparison)'
    ;;
esac

cat <<'SUMMARY'

  The complete governed cognitive pipeline WITH LLM synthesis:

  PHASE 1  Tool Runtime (read-only)
    Discovered 3 sample documents, read contents, searched keywords

  PHASE 2  Cognitive Analysis (LLM-assisted)
    Working memory enriched with model synthesis, structured [STATE],
    [KEY GAP/RISK], and [RECOMMENDED NEXT STEP] sections

  PHASE 3  Governed Analysis Pipeline (LLM-assisted)
    Assessment -> FailureInsightCandidates -> Decision Gate
    -> Decision + AuditEvent production with LLM-enriched context

  PHASE 4  Operator Readback
    System status, LLM journal with provider/model/summary traces
    All output in structured JSON for programmatic consumption

  Safe boundaries preserved:
    Read-only + workspace-scoped tools, non-authorizing output,
    governance chain, no external effects, LLM never approves actions
    or writes memory directly.

SUMMARY
printf '\n'
