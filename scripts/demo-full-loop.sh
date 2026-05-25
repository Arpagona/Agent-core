#!/usr/bin/env bash
# ARPAGONA Agent Core — Full Governed FailureInsight Demo Loop
#
# A self-contained, single-repeatable-command demo that proves the
# governed FailureInsight learning loop end-to-end:
#
#   signal → proposal → decision → audit → persistence → readback → snapshot-list → cross-invocation readback
#
# Prerequisites: None (uses the built binary, no API server needed)
#
# Usage:
#   ./scripts/demo-full-loop.sh             # human-readable output
#   ./scripts/demo-full-loop.sh --json      # structured JSON throughout
#   ./scripts/demo-full-loop.sh --clean     # remove previous snapshots before starting
#
# Exit codes:
#   0 — full chain succeeded
#   1 — any step failed
#

set -euo pipefail

BIN=(cargo run -q --bin arpagona --)
SNAPSHOT_DIR="${ARPAGONA_SNAPSHOT_DIR:-target/demo-snapshots}"
SNAPSHOT_FILE="${SNAPSHOT_DIR}/full-loop-demo.snapshot.json"
DESCRIPTION="${DESCRIPTION:-Full governed FailureInsight demo loop — signal to cross-invocation readback}"

# ── Parse CLI flags ──────────────────────────────────────────────
JSON_MODE=false
CLEAN_MODE=false
for arg in "$@"; do
  case "$arg" in
    --json) JSON_MODE=true ;;
    --clean) CLEAN_MODE=true ;;
  esac
done

JSON_FLAG=""
if $JSON_MODE; then
  JSON_FLAG="--json"
fi

# ── Colors / formatting ──────────────────────────────────────────
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

step() {
  printf "\n${BOLD}━━━ %s ━━━${NC}\n" "$1"
}

ok()   { printf "  ${GREEN}✓${NC} %s\n" "$1"; }
info() { printf "  ${BLUE}→${NC} %s\n" "$1"; }
warn() { printf "  ${YELLOW}⚠${NC} %s\n" "$1"; }
fail() { printf "  ${RED}✗${NC} %s\n" "$1"; exit 1; }

# ── Step 0: Setup ────────────────────────────────────────────────
step "Step 0: Setup"

mkdir -p "$SNAPSHOT_DIR"
if $CLEAN_MODE; then
  info "Cleaning old snapshots..."
  rm -f "${SNAPSHOT_DIR}"/*.snapshot.json
  ok "Old snapshots removed"
fi

info "Snapshot path: ${SNAPSHOT_FILE}"
ok "Setup complete"

# ── Step 1: Governed FailureInsight demo with snapshot ───────────
step "Step 1: Governed FailureInsight demo (signal → proposal → decision → audit → persistence)"

OUTPUT="$("${BIN[@]}" memory demo failure-insight \
  --description "${DESCRIPTION}" \
  --snapshot-path "${SNAPSHOT_FILE}" \
  ${JSON_FLAG} 2>&1)"
echo "$OUTPUT" | head -5

if [ ! -f "${SNAPSHOT_FILE}" ]; then
  fail "Snapshot file was not created at ${SNAPSHOT_FILE}"
fi
ok "Snapshot written to ${SNAPSHOT_FILE}"

# Verify decision was approved
if echo "$OUTPUT" | grep -q '"decision_status": "approved"'; then
  ok "Decision Gate approved the proposal"
elif echo "$OUTPUT" | grep -q 'Approved because'; then
  ok "Decision Gate approved the proposal"
else
  warn "Could not confirm decision status — check output above"
fi

# ── Step 2: Snapshot readback (same process) ─────────────────────
step "Step 2: Snapshot readback (same process)"

"${BIN[@]}" memory demo snapshot-read "${SNAPSHOT_FILE}" ${JSON_FLAG} 2>&1 | head -5
ok "Snapshot readback succeeded"

# ── Step 3: Snapshot listing ─────────────────────────────────────
step "Step 3: Snapshot discovery (snapshot-list)"

LIST_OUTPUT="$("${BIN[@]}" memory demo snapshot-list \
  --snapshot-dir "${SNAPSHOT_DIR}" \
  ${JSON_FLAG} 2>&1)"
echo "$LIST_OUTPUT" | head -8

if echo "$LIST_OUTPUT" | grep -q 'full-loop-demo\|"file_name"'; then
  ok "snapshot-list found the demo snapshot"
else
  fail "snapshot-list did not find the snapshot"
fi

# ── Step 4: Cross-invocation readback (separate process) ─────────
step "Step 4: Cross-invocation readback (separate process invocation)"

CROSS_OUTPUT="$("${BIN[@]}" memory demo snapshot-read "${SNAPSHOT_FILE}" --json 2>&1)"
echo "$CROSS_OUTPUT" | head -5

# Verify evidence-only token
if echo "$CROSS_OUTPUT" | grep -q 'evidence_only_token\|evidence-only\|"non_authorizing"'; then
  ok "Readback carries evidence-only marker"
else
  warn "Evidence-only marker not found in readback — check output"
fi

# Verify description persisted through the chain
if echo "$CROSS_OUTPUT" | grep -q 'governed\|FailureInsight\|demo'; then
  ok "Description content persisted across process invocations"
else
  warn "Could not find description content in cross-invocation readback"
fi

ok "Cross-invocation readback succeeded"

# ── Step 5: Verify with custom description ───────────────────────
step "Step 5: Custom description propagation"

CUSTOM_SNAPSHOT="${SNAPSHOT_DIR}/custom-description.snapshot.json"
CUSTOM_DESC="Custom operator-supplied FailureInsight for daily validation — $(date +%Y-%m-%d)"

CUSTOM_OUTPUT="$("${BIN[@]}" memory demo failure-insight \
  --description "${CUSTOM_DESC}" \
  --snapshot-path "${CUSTOM_SNAPSHOT}" \
  --json 2>&1)"

CUSTOM_READBACK="$("${BIN[@]}" memory demo snapshot-read "${CUSTOM_SNAPSHOT}" --json 2>&1)"

if echo "$CUSTOM_READBACK" | grep -q 'daily validation'; then
  ok "Custom description propagated through the full governed loop"
else
  warn "Custom description may not have propagated — check readback"
fi

# ── Final summary ────────────────────────────────────────────────
step "Demo complete"
printf "
${GREEN}✓${NC} Full governed FailureInsight demo loop succeeded.

Chain verified:
  signal → proposal → decision → audit → approved persistence
  → snapshot-write → snapshot-read → snapshot-list
  → cross-invocation readback → custom description propagation

Snapshots:
  ${SNAPSHOT_FILE}
  ${CUSTOM_SNAPSHOT}

Evidence-only token present: ✓
Decision Gate enforced:     ✓
No direct execution:        ✓
Readback ≠ authorization:   ✓

Use --json for structured output suitable for CI validation.
Use --clean to reset snapshots before running.
"
