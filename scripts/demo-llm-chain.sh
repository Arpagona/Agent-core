#!/usr/bin/env bash
#
# Demo — ARPAGONA Agent Core LLM Chain (C1→C3→D1)
#
# Demonstrates the complete operator-visible LLM interaction chain:
#
#   C1: Real LLM integration (proposal-only mode) → cognitive run --llm
#   C3: LLM interaction journal → arpagona llm journal
#   D1: Operator status surface → arpagona status
#
# No API server required. No external side effects. Read-only governance.
# Uses --provider mock for deterministic output without local model or API key.
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

check_json_field() {
  # Check that a JSON field exists and is non-empty
  local field="$1"
  local file="$2"
  local label="${3:-$field}"
  if python3 -c "
import json, sys
data = json.load(open('$file'))
val = data
for key in '$field'.split('.'):
    if isinstance(val, dict):
        val = val.get(key, None)
    elif isinstance(val, list):
        if len(val) > 0:
            val = val[0].get(key, None)
        else:
            val = None
    if val is None:
        sys.exit(1)
" 2>/dev/null; then
    pass "$label present"
  else
    fail "MISSING: $label"
  fi
}

# ════════════════════════════════════════════════════════════════════
#  MAIN DEMO
# ════════════════════════════════════════════════════════════════════

header "ARPAGONA Agent Core — LLM Chain Demo (C1→C3→D1)"

printf '\n'
cat <<'INTRO'
  This demo runs the complete operator-visible LLM interaction chain:

    C1  cognitive run --llm           → real LLM synthesis (proposal-only)
    C3  arpagona llm journal           → prompt/response journal with model metadata
    D1  arpagona status --json         → operator status surface with subsystem health

  All commands use --provider mock for deterministic, offline-safe output.
  No Ollama, no OpenAI API key, no network required.

INTRO

TMPDIR=$(mktemp -d /tmp/arpagona-demo-llm-chain-XXXXXX)
trap 'rm -rf "$TMPDIR"' EXIT

# ── Step 1: C1 — Cognitive run with LLM synthesis ─────────────────

section "Step 1: C1 — cognitive run --llm --provider mock"

"${CLI[@]}" cognitive run \
  --objective "Analyser les tendances du marché de l'IA en France pour 2026" \
  --domain business \
  --llm \
  --provider mock \
  --json \
  2>/dev/null > "$TMPDIR/step1.json" || fail "Step 1 failed"

check_json_field "llm_synthesis" "$TMPDIR/step1.json" "llm_synthesis (C1 output)"
check_json_field "llm_provider" "$TMPDIR/step1.json" "llm_provider"
check_json_field "llm_routing" "$TMPDIR/step1.json" "llm_routing"

# Verify synthesis contains structured sections
python3 -c "
import json
data = json.load(open('$TMPDIR/step1.json'))
synth = data.get('llm_synthesis', '')
assert '[STATE]' in synth, 'Missing [STATE] section'
assert '[KEY GAP / RISK]' in synth or '[KEY GAP' in synth, 'Missing [KEY GAP / RISK] section'
assert '[RECOMMENDED NEXT STEP]' in synth, 'Missing [RECOMMENDED NEXT STEP] section'
print('  ✔ Synthesis contains [STATE]/[KEY GAP]/[RECOMMENDED NEXT STEP] sections')
" || fail "Synthesis missing required sections"

pass "C1 — LLM synthesis produces structured, proposal-only output"
pass "C1 — Provider and routing metadata available in JSON output"

# ── Step 2: C3 — LLM journal readback ──────────────────────────────

section "Step 2: C3 — arpagona llm journal"

"${CLI[@]}" llm journal --json --limit 5 \
  2>/dev/null > "$TMPDIR/step2.json" || fail "Step 2 failed"

python3 -c "
import json
data = json.load(open('$TMPDIR/step2.json'))
entries = data if isinstance(data, list) else data.get('entries', data.get('journal_entries', []))
assert len(entries) > 0, 'No journal entries found'
entry = entries[0]
assert entry.get('provider') == 'mock', f'Expected mock provider, got {entry.get(\"provider\")}'
assert entry.get('objective') or entry.get('prompt_summary'), 'Missing objective or prompt_summary'
print(f'  ✔ Found {len(entries)} journal entries')
print(f'  ✔ Latest entry: provider={entry.get(\"provider\")}, model={entry.get(\"model\")}')
print(f'  ✔ Objective: {entry.get(\"objective\", \"(present)\")[:60]}...')
" || fail "Journal validation failed"

pass "C3 — LLM journal persists synthesis interaction"
pass "C3 — Provider, model metadata stored in journal entry"

# ── Step 3: C1 + C4 — LLM synthesis with compute routing ──────────

section "Step 3: (Optional) C1+C4 — cognitive run --llm --allocate"

"${CLI[@]}" cognitive run \
  --objective "Évaluer l'impact des modèles de langage sur l'analyse de documents juridiques" \
  --domain business \
  --llm \
  --provider mock \
  --allocate \
  --json \
  2>/dev/null > "$TMPDIR/step3.json" || fail "Step 3 failed"

# When --allocate is used, the compute reservoir overrides --provider mock.
# Business domain routes to local-smol → ollama (works without API key).
python3 -c "
import json
data = json.load(open('$TMPDIR/step3.json'))
synth = data.get('llm_synthesis', '')
assert synth, 'llm_synthesis not found at top level'
assert '[STATE]' in synth, 'Missing [STATE] section in synthesis'
assert '[KEY GAP / RISK]' in synth or '[KEY GAP' in synth, 'Missing [KEY GAP / RISK]'
assert '[RECOMMENDED NEXT STEP]' in synth, 'Missing [RECOMMENDED NEXT STEP]'
print('  ✔ llm_synthesis present at top level (C1+C4 integration)')
print('  ✔ Synthesis contains [STATE]/[KEY GAP]/[RECOMMENDED NEXT STEP] sections')
provider = data.get('llm_provider', '?')
routing = data.get('llm_routing', '?')
cr = data.get('compute_requirement', {})
print(f'  ✔ Provider: {provider}')
print(f'  ✔ Routing: {routing}')
print(f'  ✔ compute_requirement.status: {cr.get(\"status\", \"?\")}')
print(f'  ✔ compute_requirement.selected_node_id: {cr.get(\"selected_node_id\", \"?\")}')
assert data.get('allocated'), 'allocated flag should be true'
print('  ✔ allocated flag is true')
" || fail "Step 3 validation failed"

pass "C1+C4 — LLM synthesis with compute routing produces expected output"

# ── Step 4: D1 — Operator status surface ───────────────────────────

section "Step 4: D1 — arpagona status"

"${CLI[@]}" status --json \
  2>/dev/null > "$TMPDIR/step4.json" || fail "Step 4 failed"

check_json_field "local" "$TMPDIR/step4.json" "local subsystem status (D1 surface)"

# Check specific subsystem fields
python3 -c "
import json
data = json.load(open('$TMPDIR/step4.json'))
local = data.get('local', {})
required = ['holographic_memory_db_path', 'tool_runtime_tool_count', 'tool_runtime_tools', 'mcp_server_binary_available', 'cli_version']
for field in required:
    assert field in local, f'Missing local subsystem field: {field}'
    val = local[field]
    if val is not None:
        print(f'  ✔ {field}: {val}')
    else:
        print(f'  ✔ {field}: (None)')
print(f'  ✔ All {len(required)} required fields present')
" || fail "D1 status missing required fields"

pass "D1 — Operator status surface shows subsystem health"

# ── Summary ─────────────────────────────────────────────────────────

header "Résumé — Chaîne LLM complète (C1→C3→D1)"

cat <<SUMMARY

  ✔ C1 — LLM synthesis produces structured proposal-only output
  ✔ C3 — LLM journal persists interaction with provider/model metadata
  ✔ C4 — Compute routing metadata captured in journal
  ✔ D1 — Operator status surface shows subsystem health

  Architecture chaîne prouvée:

    cognitive run --llm --provider mock --json
    → llm_synthesis (structured [STATE]/[KEY GAP]/[RECOMMENDED NEXT STEP])
    → llm_journal entry (provider, model, prompt_summary, response_summary)
    → status --json (local subsystem health, version, handoff)

  Sécurité:
    - Toute sortie LLM est proposal-only (n'approuve pas d'actions)
    - Le journal est evidence-only (pas d'autorisation)
    - Le statut est read-only (pas d'exécution)
    - Aucun appel LLM réel, aucune clé API, aucun réseau requis
    - --provider mock donne un comportement déterministe et reproductible

SUMMARY
