#!/usr/bin/env bash
#
# Demo — ARPAGONA Agent Core Full Governed Cognitive Loop
#
# A single repeatable script that runs the complete governed loop:
#
#   Objective → WorkingMemory → Plan → Observations → Assessment
#   → FailureInsightCandidates → DecisionGate → Decision → Audit
#   → CycleTrace (cost/quality metadata) → readback
#
# No API server required. No external side effects. Read-only governance.
#
# Prerequisites: cargo in PATH, workspace compiled (cargo build).
#
set -euo pipefail

CLI=(cargo run -q --bin arpagona)

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

pass() {
  printf '  ✔ %s\n' "$1"
}

fail() {
  printf '  ✘ %s\n' "$1" >&2
  exit 1
}

show_json_field() {
  python3 -c "
import json, sys
data = json.load(sys.stdin)
val = data
for key in '$1'.split('.'):
    if isinstance(val, dict):
        val = val.get(key, '<<MISSING>>')
    elif isinstance(val, list):
        val = [item.get(key, '<<MISSING>>') for item in val]
    else:
        val = '<<INVALID>>'
if isinstance(val, list):
    for v in val:
        print(v)
else:
    print(val)
" 2>/dev/null || echo "  (could not parse: $1)"
}

validate_and_format() {
  # Validate JSON structure and extract key governance fields
  python3 -c "
import json, sys

data = json.load(sys.stdin)

# Check required top-level keys
required = ['objective', 'working_memory', 'plan', 'required_observations',
            'proposed_next_action', 'improvement_candidates']
for key in required:
    if key not in data:
        sys.exit(f'MISSING_TOPLEVEL: {key}')

decision_count = data.get('decision_count', 0)
audit_event_count = data.get('audit_event_count', 0)
governed = data.get('governed', False)
warning = data.get('governance_warning', '')
assessed = data.get('assessed', False)

# Validation
errors = []
if not isinstance(decision_count, int) or decision_count < 1:
    errors.append(f'decision_count must be >= 1, got {decision_count}')
if not isinstance(audit_event_count, int) or audit_event_count < 1:
    errors.append(f'audit_event_count must be >= 1, got {audit_event_count}')
if not warning:
    errors.append('governance_warning is empty')
if not governed:
    errors.append('governed is False')
if not assessed:
    errors.append('assessed is False')

gov_results = data.get('governance_results', [])
if not isinstance(gov_results, list) or len(gov_results) == 0:
    errors.append('governance_results is empty')

if errors:
    for err in errors:
        print(f'VALIDATION_ERROR: {err}')
    sys.exit(1)

print(json.dumps({
    'decision_count': decision_count,
    'audit_event_count': audit_event_count,
    'assessed': assessed,
    'governed': governed,
    'governance_proposal_count': len(gov_results),
    'governance_warning': warning[:120] + '...' if len(warning) > 120 else warning,
    'proposed_actions': [{
        'id': r['proposed_action_id'],
        'action_type': r.get('proposed_action', {}).get('action_type', 'unknown'),
        'decision_status': r.get('decision', {}).get('status', 'unknown'),
        'risk_level': r.get('proposed_action', {}).get('risk_level', 'unknown'),
    } for r in gov_results],
}, indent=2))
" 2>&1 || echo "{\"error\": \"Output validation failed — the CLI may have returned unexpected content\"}"
}

# ════════════════════════════════════════════════════════════════════
#  MAIN DEMO
# ════════════════════════════════════════════════════════════════════

header "ARPAGONA Agent Core — Full Governed Cognitive Loop"

printf '\n  This demo runs the complete offline governance chain:\n'
printf '  Objective → WorkingMemory → Plan → Assessment → DecisionGate → Decision → Audit\n'
printf '  No API server. No LLM calls. No external effects. Read-only.\n'

# ── Step 1: Governance loop (business domain) ─────────────────────

section "1. Governance chain — business domain"

printf '\n'
"${CLI[@]}" cognitive run \
  --objective "Analyser les tendances du marché de l'IA en France pour 2026" \
  --domain business \
  --assess \
  --observe \
  --govern \
  --json 2>/dev/null \
  | validate_and_format

printf '\n'

# ── Step 2: Governance loop (coding domain) ────────────────────────

section "2. Governance chain — coding domain"

printf '\n'
"${CLI[@]}" cognitive run \
  --objective "Refactoriser le module de parsing CSV pour supporter l'UTF-16" \
  --domain coding \
  --assess \
  --observe \
  --govern \
  --json 2>/dev/null \
  | validate_and_format

printf '\n'

# ── Step 3: Governance loop (research domain) ──────────────────────

section "3. Governance chain — research domain"

printf '\n'
"${CLI[@]}" cognitive run \
  --objective "Évaluer l'impact des modèles de langage sur l'analyse de documents juridiques" \
  --domain research \
  --assess \
  --observe \
  --govern \
  --json 2>/dev/null \
  | validate_and_format

printf '\n'

# ── Step 4: Orchestrator run — simulated proposal generator ─────────

section "4. Neutral Orchestrator — deterministic (simulated)"

printf '\n'
printf '  Simulated proposal generator (default): deterministic ReadDocument at Low risk.\n\n'
"${CLI[@]}" orchestrator run \
  --objective "Analyser les tendances du marché de l'IA en France pour 2026" \
  --proposal-generator simulated \
  --workspace-id workspace-alpha \
  --agent-id agent-alpha \
  --perm ReadDocument \
  --json 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
print(f'  Cycle status:  {data.get(\"cycle_status\", \"?\")}')
print(f'  Gate applied:  {data.get(\"gate_was_applied\", \"?\")}')
print(f'  Non-auth:      {data.get(\"non_authorizing\", \"?\")}')
print(f'  Decision ID:   {data.get(\"decision_id\", \"?\")}')
print(f'  Audit events:  {len(data.get(\"audit_event_ids\", []))}')
print(f'  Summary:       {data.get(\"summary\", \"\")}')
"
printf '\n'

# ── Step 5: Orchestrator run — LLM proposal generator ──────────────

section "5. Neutral Orchestrator — LLM-backed (mock provider)"

printf '\n'
printf '  LLM proposal generator: wraps MockProvider for real proposal-only cycle integration.\n\n'
"${CLI[@]}" orchestrator run \
  --objective "Analyser les tendances du marché de l'IA en France pour 2026" \
  --proposal-generator llm \
  --workspace-id workspace-alpha \
  --agent-id agent-alpha \
  --perm ReadDocument \
  --json 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
print(f'  Cycle status:  {data.get(\"cycle_status\", \"?\")}')
print(f'  Gate applied:  {data.get(\"gate_was_applied\", \"?\")}')
print(f'  Non-auth:      {data.get(\"non_authorizing\", \"?\")}')
print(f'  Decision ID:   {data.get(\"decision_id\", \"?\")}')
print(f'  Audit events:  {len(data.get(\"audit_event_ids\", []))}')
print(f'  Summary:       {data.get(\"summary\", \"\")}')
"
printf '\n'

# ── Step 6: Orchestrator run with CycleTrace — cost/quality metadata ─

section "6. Orchestrator CycleTrace — trace with cost/quality metadata"

printf '\n'
printf '  CycleTrace gives operator full visibility into context assembly,\n'
printf '  compute routing and failure insight candidates.\n\n'
"${CLI[@]}" orchestrator run \
  --objective "Analyser les tendances du marché de l'IA en France pour 2026" \
  --proposal-generator simulated \
  --workspace-id workspace-alpha \
  --agent-id agent-alpha \
  --perm ReadDocument \
  --trace \
  --save-trace target/demo-cycletrace.json \
  --json 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
trace = data.get('cycle_trace', data)
print(f'  Objective:      {trace.get(\"objective_summary\", \"?\")}')
print(f'  Context items:  {trace.get(\"total_context_items\", 0)}')
print(f'  Sources used:   {trace.get(\"context_sources_used\", 0)}')
print(f'  Gate applied:   {trace.get(\"gate_was_applied\", \"?\")}')
print(f'  Failure cand.:  {len(trace.get(\"failure_insight_candidates\", []))}')
print(f'  Trace path:     target/demo-cycletrace.json')
"
printf '\n'

# ── Step 7: Orchestrator status from saved CycleTrace ───────────────

section "7. Orchestrator readback — status from saved trace"

printf '\n'
printf '  Demonstrates cross-invocation readback: re-read a saved CycleTrace.\n\n'
"${CLI[@]}" orchestrator status --trace-path target/demo-cycletrace.json --json 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
trace = data.get('cycle_trace', data)
print(f'  Trace loaded:   {trace.get(\"objective_summary\", \"?\")}')
print(f'  Context items:  {trace.get(\"total_context_items\", 0)}')
print(f'  Sources used:   {trace.get(\"context_sources_used\", 0)}')
print(f'  Failure cand.:  {len(trace.get(\"failure_insight_candidates\", []))}')
for cand in trace.get('failure_insight_candidates', []):
    print(f'    - {cand.get(\"kind\", \"?\")}: {cand.get(\"summary\", \"\")[:120]}')
"
printf '\n'

# ── Summary ─────────────────────────────────────────────────────────

header "Résumé — Demo terminée"

cat <<'SUMMARY'

  ✔ Sept étapes cognitives complètes ont été exécutées :
    - Cycles 1-3 : Cognitive Work Loop avec gouvernance (business, coding, research)
    - Cycle 4   : Neutral Orchestrator avec générateur de proposition simulé
    - Cycle 5   : Neutral Orchestrator avec générateur de proposition LLM (mock)
    - Cycle 6   : CycleTrace avec métadonnées de contexte et de routage
    - Cycle 7   : Readback cross-invocation à partir du fichier trace sauvegardé
    - Tous les cycles ont produit :
      • Objective + WorkingMemory + Plan
      • Assessment (FailureInsightCandidates)
      • Observation bridge (tool runtime)
      • Governance (DecisionGate → Decision → Audit)
      • CycleTrace avec métadonnées de contexte
    - Toutes les sorties sont evidence-only, non-authorizing.
    - Aucun appel LLM réel, aucune persistence, aucun effet externe.

  La chaîne de gouvernance hors-ligne fonctionne sans serveur API :
    ProposedAction → DecisionGate → Decision → AuditEvent
    → gouvernance_results JSON avec décisions et événements d'audit.

  Le CycleTrace donne à l'opérateur la visibilité complète sur :
    - le contexte assemblé (sources, items, unavailable)
    - le routage compute (coût, latence, type de ressource)
    - les candidats Failure-to-Insight détectés

  Le Neutral Orchestrator supporte deux backends de proposition :
    - `simulated` (défaut) : proposition déterministe ReadDocument/Low
    - `llm` : proposition via fournisseur LLM en mode proposition uniquement

  Prochaine étape recommandée :
    Intégrer le Holographic Memory (résonance) dans le CycleTrace
    pour enrichir le contexte assembleur avec des traces épisodiques passées.

SUMMARY

printf '\n'
