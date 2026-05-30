#!/usr/bin/env bash
# scripts/smoke-human-cli.sh
#
# Human smoke test for ARPAGONA CLI.
# Uses the pre-built binary at target/debug/arpagona.
# All commands have explicit timeouts.
#
# Usage:
#   bash scripts/smoke-human-cli.sh [--build] [--all]
#
#   --build   Rebuild the binary before running smoke tests.
#   --all     Run all smoke tests (default: run standard set).
#
# Exit code: number of failed tests (0 = all pass).
set -euo pipefail

BINARY="target/debug/arpagona"
TIMEOUT=20  # seconds per command
FAILED=0
PASSED=0
SKIPPED=0
TOTAL=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Parse flags
BUILD=false
ALL=false
for arg in "$@"; do
  case "$arg" in
    --build) BUILD=true ;;
    --all)   ALL=true ;;
  esac
done

echo ""
echo "=== ARPAGONA Human Smoke Test ==="
echo ""

# Optional build
if [ "$BUILD" = true ]; then
  echo "Building binary (this may take a while)..."
  cargo build -p arpagona-cli 2>&1
  echo "Build complete."
  echo ""
fi

# Verify binary exists
if [ ! -f "$BINARY" ]; then
  echo -e "${RED}[FAIL]${NC} Binary not found at $BINARY"
  echo "       Run with --build first, or build manually: cargo build -p arpagona-cli"
  exit 1
fi
echo -e "${GREEN}[OK]${NC} Binary found: $BINARY"
echo ""

run_test() {
  local name="$1"
  local cmd="$2"
  TOTAL=$((TOTAL + 1))
  echo -n "  $name ... "

  # Run with timeout, capture stdout+stderr
  local output
  local exit_code=0
  output=$(timeout "$TIMEOUT" bash -c "$cmd" 2>&1) || exit_code=$?

  if [ $exit_code -eq 124 ]; then
    echo -e "${RED}TIMEOUT${NC} (${TIMEOUT}s)"
    echo "    Command exceeded timeout: $cmd"
    FAILED=$((FAILED + 1))
    return 1
  elif [ $exit_code -ne 0 ]; then
    echo -e "${RED}FAIL${NC} (exit code $exit_code)"
    echo "    Output (first 10 lines):"
    echo "$output" | head -10 | sed 's/^/    /'
    FAILED=$((FAILED + 1))
    return 1
  else
    echo -e "${GREEN}PASS${NC}"
    PASSED=$((PASSED + 1))
    return 0
  fi
}

echo "--- Basic Commands ---"
run_test "help"          "$BINARY --help > /dev/null"
run_test "version"       "$BINARY --version > /dev/null 2>&1 || $BINARY version > /dev/null"
run_test "status"        "$BINARY status --help > /dev/null 2>&1; $BINARY status > /dev/null 2>&1"

echo ""
echo "--- Run Command ---"
run_test "run (basic)"   "$BINARY run 'Test objective' > /dev/null"

echo ""
echo "--- Orchestrator ---"
run_test "orchestrator help" "$BINARY orchestrator --help > /dev/null"

# Only run --json --trace with --all since it may be slow
if [ "$ALL" = true ]; then
  echo ""
  echo "--- Extended Commands ---"
  run_test "orchestrator run --json --trace" "$BINARY orchestrator run --objective 'Smoke test objective' --json --trace > /dev/null"
  run_test "compute routing"                 "$BINARY compute routing --purpose 'Smoke test' > /dev/null"
  run_test "memory status"                   "$BINARY memory status > /dev/null"
  run_test "audit list"                      "$BINARY audit list > /dev/null 2>&1"
fi

# Summary
echo ""
echo "--- Results ---"
echo -e "  ${GREEN}PASSED${NC}:  $PASSED"
if [ "$FAILED" -gt 0 ]; then
  echo -e "  ${RED}FAILED${NC}:  $FAILED"
else
  echo "  FAILED:  0"
fi
if [ "$SKIPPED" -gt 0 ]; then
  echo -e "  ${YELLOW}SKIPPED${NC}: $SKIPPED"
fi
echo "  TOTAL:   $TOTAL"
echo ""

exit "$FAILED"
