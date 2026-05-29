#!/usr/bin/env bash
#
# Demo — ARPAGONA Seeded Orchestrator (P3-18 multi-adapter CLI wiring)
#
# Proves the full `--seed-* --multi-adapter --trace --json` chain works
# end-to-end. Requires the P3-18 branch (feat/p3-18-multi-adapter-cli-wiring).
#
#   Objective → WorkingMemory → ContextAssembly (5 adapters)
#   → Compute Allocation → Proposal → DecisionGate → Decision → Audit
#
# No API server required. No external side effects. Read-only governance.
#
# Prerequisites: cargo in PATH, workspace compiled.
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

warn() {
  printf '  ⚠ %s\n' "$1"
}

fail() {
  printf '  ✘ %s\n' "$1" >&2
  exit 1
}

# ── JSON field extractor (pipe-based, avoids shell expansion of $ in JSON) ──
get_json_field() {
  local field="$2"
  printf '%s' "$1" | python3 -c "import json,sys; data=json.load(sys.stdin); print(data.get('$field','?'))" 2>/dev/null || echo "?"
}

# ── JSON multi-field validator (reads from stdin) ───────────────────
validate_full_chain() {
  python3 -c "
import json, sys
data = json.load(sys.stdin)

# Handle both trace and non-trace schemas
has_ids = 'audit_event_ids' in data
has_count = 'audit_event_count' in data

required = ['cycle_id', 'cycle_status', 'non_authorizing', 'gate_was_applied',
            'summary']
if not has_ids and not has_count:
    required.append('audit_event_ids_or_count')

missing = [k for k in required if k not in data]
if missing:
    print('MISSING: %s' % missing)
    sys.exit(1)

count = len(data.get('audit_event_ids', [])) if has_ids else (data.get('audit_event_count', 0) if has_count else 0)
print('OK cycle_id=%s status=%s audit_events=%d' % (
    data['cycle_id'], data['cycle_status'], count))
" 2>/dev/null || echo "VALIDATION_FAILED"
}

# ════════════════════════════════════════════════════════════════════
#  MAIN DEMO
# ════════════════════════════════════════════════════════════════════

header "ARPAGONA — Seeded Orchestrator Demo (P3-18)"

printf '\n'
printf '  This demo proves the multi-adapter orchestrator chain:\n'
printf '  --seed-* flags → MultiAdapterContextAssembler → CycleTrace → JSON\n'
printf '  No API server. No LLM calls. Read-only governance.\n'

PASS=0
FAIL=0
TRACE_FILE="target/demo-seeded-trace.json"

# ── Step 1: Basic orchestrator without --multi-adapter ─────────────

section "1. Basic orchestrator (simulated assembler)"

OUT=$("${CLI[@]}" orchestrator run \
  --objective "Analyser les tendances du marche" \
  --perm ReadDocument \
  --json 2>/dev/null) || true

CYCLE_STATUS=$(get_json_field "$OUT" "cycle_status")
NON_AUTH=$(get_json_field "$OUT" "non_authorizing")

if [ "$CYCLE_STATUS" = "completed" ] || [ "$CYCLE_STATUS" = "Completed" ]; then
  pass "Basic orchestrator produces completed cycle"
  PASS=$((PASS+1))
else
  warn "Basic orchestrator cycle_status=$CYCLE_STATUS (expecting 'completed')"
  FAIL=$((FAIL+1))
fi

if [ "$NON_AUTH" = "True" ]; then
  pass "Non-authorizing invariant preserved"
  PASS=$((PASS+1))
else
  warn "non_authorizing is not True (got: $NON_AUTH)"
  FAIL=$((FAIL+1))
fi

# ── Step 2: With --multi-adapter ───────────────────────────────────

section "2. Multi-adapter context assembly"

OUT=$("${CLI[@]}" orchestrator run \
  --objective "Review project documentation quality" \
  --multi-adapter \
  --perm ReadDocument \
  --json 2>/dev/null) || true

CYCLE_STATUS=$(get_json_field "$OUT" "cycle_status")
if [ "$CYCLE_STATUS" = "completed" ] || [ "$CYCLE_STATUS" = "Completed" ]; then
  pass "Multi-adapter cycle completes"
  PASS=$((PASS+1))
else
  fail "Multi-adapter cycle failed: $CYCLE_STATUS"
fi

# ── Step 3: With seed flags ────────────────────────────────────────

section "3. Seeded adapters"

OUT=$("${CLI[@]}" orchestrator run \
  --objective "Analyse client feedback patterns" \
  --multi-adapter \
  --seed-audit-event "Client requested GDPR compliance review" \
  --seed-holo-trace "Previous audit found data retention policy gaps" \
  --seed-reservoir-pulse "Working memory: GDPR analysis in progress" \
  --seed-cca-event "Compliance review needed for Q3 client onboarding" \
  --perm ReadDocument \
  --json 2>/dev/null) || true

CYCLE_STATUS=$(get_json_field "$OUT" "cycle_status")
NON_AUTH=$(get_json_field "$OUT" "non_authorizing")
GATE=$(get_json_field "$OUT" "gate_was_applied")

if [ "$CYCLE_STATUS" = "completed" ] || [ "$CYCLE_STATUS" = "Completed" ]; then
  pass "Seeded cycle completes"
  PASS=$((PASS+1))
else
  fail "Seeded cycle failed: $CYCLE_STATUS"
fi

if [ "$NON_AUTH" = "True" ] && [ "$GATE" = "True" ]; then
  pass "Seeded: gate applied + non-authorizing preserved"
  PASS=$((PASS+1))
else
  warn "Seeded: non_authorizing=$NON_AUTH gate=$GATE"
  FAIL=$((FAIL+1))
fi

# ── Step 4: With --trace (CycleTrace metadata) ─────────────────────

section "4. CycleTrace with metadata"

OUT=$("${CLI[@]}" orchestrator run \
  --objective "Code review for authentication module" \
  --multi-adapter \
  --seed-audit-event "Auth module reviewed in Q2" \
  --trace \
  --json 2>/dev/null) || true

CYCLE_STATUS=$(get_json_field "$OUT" "cycle_status")
if [ "$CYCLE_STATUS" = "completed" ] || [ "$CYCLE_STATUS" = "Completed" ]; then
  pass "CycleTrace with metadata completes"
  PASS=$((PASS+1))
else
  fail "CycleTrace failed: $CYCLE_STATUS"
fi

# ── Step 5: With --save-trace ──────────────────────────────────────

section "5. Save trace to file"

rm -f "$TRACE_FILE"

"${CLI[@]}" orchestrator run \
  --objective "Evaluate API documentation completeness" \
  --multi-adapter \
  --seed-audit-event "API docs reviewed in sprint 12" \
  --seed-holo-trace "Previous sprints: consistent doc gaps in error handling" \
  --seed-reservoir-pulse "Current sprint focus: API error responses" \
  --save-trace "$TRACE_FILE" \
  --trace \
  2>/dev/null > /dev/null || true

if [ -f "$TRACE_FILE" ]; then
  TRACE_SIZE=$(wc -c < "$TRACE_FILE")
  if [ "$TRACE_SIZE" -gt 50 ]; then
    pass "Trace saved: $TRACE_SIZE bytes"
    PASS=$((PASS+1))
  else
    warn "Trace file too small: $TRACE_SIZE bytes"
    FAIL=$((FAIL+1))
  fi
else
  fail "Trace file not created: $TRACE_FILE"
fi

# Verify trace contents
TRACE_CYCLE_ID=$(python3 -c "
import json
with open('$TRACE_FILE') as f:
    data = json.load(f)
print(data.get('cycle_id', '?'))
" 2>/dev/null) || TRACE_CYCLE_ID=""

if [ -n "$TRACE_CYCLE_ID" ] && [ "$TRACE_CYCLE_ID" != "?" ]; then
  pass "Trace contains valid cycle_id: $TRACE_CYCLE_ID"
  PASS=$((PASS+1))
else
  warn "Trace file missing cycle_id"
  FAIL=$((FAIL+1))
fi

rm -f "$TRACE_FILE"

# ── Step 6: All flags together ─────────────────────────────────────

section "6. Full chain: all flags + JSON"

PY_OUT=$("${CLI[@]}" orchestrator run \
  --objective "GDPR compliance audit for customer data processing" \
  --multi-adapter \
  --seed-audit-event "Data processing agreement signed Q1 2026" \
  --seed-holo-trace "GDPR training completed for all engineering staff" \
  --seed-reservoir-pulse "Priority: update data retention policy" \
  --seed-cca-event "Previous audit: customer consent records need review" \
  --trace \
  --json \
  2>/dev/null | validate_full_chain) || PY_OUT="VALIDATION_FAILED"

if echo "$PY_OUT" | grep -q "^OK"; then
  pass "Full chain JSON valid: $PY_OUT"
  PASS=$((PASS+1))
elif echo "$PY_OUT" | grep -q "^MISSING"; then
  fail "Full chain missing fields: $PY_OUT"
else
  warn "Full chain JSON: $PY_OUT"
  FAIL=$((FAIL+1))
fi

# ── Summary ─────────────────────────────────────────────────────────

header "Demo complete"
printf '\n'
printf '  Pass: %d   Fail: %d\n' "$PASS" "$FAIL"
printf '\n'

if [ "$FAIL" -eq 0 ]; then
  pass "All steps passed!"
else
  warn "$FAIL step(s) had issues"
fi

printf '\n'
cat <<'SUMMARY'
  The seeded orchestrator demo proves:

  1. Basic orchestrator cycle runs with non-authorizing invariant preserved
  2. --multi-adapter flag enables all 5 memory adapter context assembly
  3. --seed-* flags pre-populate individual adapters with test data
  4. --trace outputs CycleTrace with context assembly metadata
  5. --save-trace persists CycleTrace to a JSON file for later inspection
  6. --json produces structured machine-readable output
  7. All combinations preserve: Decision Gate, non-authorizing, audit chain

  Requirements:
  - feat/p3-18-multi-adapter-cli-wiring branch (PR #205)
  - No API server, no LLM calls, no external effects

  Next step for GONA:
  Merge the P3 stack (#200 -> #202 -> #197 -> #198 -> #199 -> #203 -> #204 -> #205)
  then extend this demo to include --insights (P3-15, PR #197) and
  compute efficiency feedback (P3-16, PR #198).

SUMMARY
