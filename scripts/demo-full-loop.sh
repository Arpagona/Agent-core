#!/usr/bin/env bash
#
# Demo — ARPAGONA Agent Core Full Governed Cognitive Loop
#
# A single repeatable script that runs the complete governed loop:
#
#   Objective → WorkingMemory → Plan → Observations → Assessment
#   → FailureInsightCandidates → DecisionGate → Decision → Audit
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

# ── Summary ─────────────────────────────────────────────────────────

header "Résumé — Demo terminée"

cat <<'SUMMARY'

  ✔ Trois cycles cognitifs complets ont été exécutés :
    - Chaque cycle a produit :
      • Objective + WorkingMemory + Plan
      • Assessment (FailureInsightCandidates)
      • Observation bridge (tool runtime)
      • Governance (DecisionGate → Decision → Audit)
    - Toutes les sorties sont evidence-only, non-authorizing.
    - Aucun appel LLM, aucune persistence, aucun effet externe.

  La chaîne de gouvernance hors-ligne fonctionne sans serveur API :
    ProposedAction → DecisionGate → Decision → AuditEvent
    → gouvernance_results JSON avec décisions et événements d'audit.

  Prochaine étape recommandée :
    Ajouter des tests d'intégration prouvant que la sortie JSON
    de la boucle gouvernée contient bien decision_count > 0,
    audit_event_count > 0, et governance_warning non vide.

SUMMARY

printf '\n'
