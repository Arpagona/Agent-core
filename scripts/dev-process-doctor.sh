#!/usr/bin/env bash
# scripts/dev-process-doctor.sh
#
# Reports stale/active ARPAGONA development processes.
# Read-only by default — shows what is running.
# Use --kill to terminate stale processes (with confirmation prompts).
#
# Detects:
#   - arpagona-api-server instances
#   - arpagona chat sessions (any provider)
#   - Cargo.lock contention
#   - Port ownership for known ARPAGONA ports
#
# Usage:
#   bash scripts/dev-process-doctor.sh         # read-only report
#   bash scripts/dev-process-doctor.sh --kill   # prompt to kill stale processes
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

KILL_MODE=false
for arg in "$@"; do
  [ "$arg" = "--kill" ] && KILL_MODE=true
done

echo ""
echo "=== ARPAGONA Dev Process Doctor ==="
echo ""

# --- Section 1: Running arpagona processes ---
echo -e "${CYAN}[1] Active arpagona processes${NC}"
PROCS=$(ps aux | grep -E 'target/debug/arpagona' | grep -v grep || true)
if [ -z "$PROCS" ]; then
  echo -e "  ${GREEN}None${NC} — no arpagona processes detected."
else
  echo "  PID   PPID  STARTED     ELAPSED   CMD"
  echo "  ----  ----  ---------  ---------  -----------------------------"
  while IFS= read -r line; do
    pid=$(echo "$line" | awk '{print $2}')
    ppid=$(echo "$line" | awk '{print $3}')
    start=$(echo "$line" | awk '{print $9,$10,$11}')
    # calculate elapsed time
    if [ -d "/proc/$pid" ]; then
      elapsed=$(ps -o etime= -p "$pid" 2>/dev/null | xargs || echo "?")
    else
      elapsed="gone"
    fi
    cmd=$(echo "$line" | awk '{for(i=11;i<=NF;i++) printf "%s ", $i; print ""}' | sed 's/ *$//')
    echo "  $pid  $ppid  $start  $elapsed  $cmd"
  done <<< "$PROCS"
fi
echo ""

# --- Section 2: Port ownership ---
echo -e "${CYAN}[2] Port occupancy${NC}"
PORTS=(3000 3001)
for port in "${PORTS[@]}"; do
  p=$(ss -tlnp "sport = :$port" 2>/dev/null || ss -tlnp "src :$port" 2>/dev/null)
  if [ -n "$p" ]; then
    owner=$(ss -tlnp "sport = :$port" 2>/dev/null | grep -oP 'pid=\K[0-9]+' || echo "unknown")
    echo -e "  Port ${BOLD}$port${NC}: ${YELLOW}IN USE${NC} (PID: $owner)"
  else
    echo -e "  Port ${BOLD}$port${NC}: ${GREEN}free${NC}"
  fi
done
echo ""

# --- Section 3: Cargo lock contention ---
echo -e "${CYAN}[3] Cargo lock contention${NC}"
CARGO_LOCK="/home/thibaud/arpagona-agent-core/Cargo.lock"
if [ -f "$CARGO_LOCK" ]; then
  # Check if a cargo build is currently running (holds .cargo-lock or .lock file)
  CARGO_CMDS=$(ps aux | grep -E 'cargo (build|check|test|clippy)' | grep -v grep || true)
  if [ -n "$CARGO_CMDS" ]; then
    echo -e "  ${YELLOW}Cargo build/check in progress${NC}:"
    while IFS= read -r line; do
      pid=$(echo "$line" | awk '{print $2}')
      start=$(echo "$line" | awk '{print $9}')
      elapsed=$(ps -o etime= -p "$pid" 2>/dev/null | xargs || echo "?")
      cmd=$(echo "$line" | awk '{for(i=11;i<=NF;i++) printf "%s ", $i; print ""}' | sed 's/ *$//')
      echo "    PID $pid ($elapsed): $cmd"
    done <<< "$CARGO_CMDS"
  else
    echo -e "  ${GREEN}No cargo contention detected.${NC}"
  fi
else
  echo -e "  ${YELLOW}No Cargo.lock found — has the project been built?${NC}"
fi
echo ""

# --- Summary / Kill prompt ---
STALE_COUNT=$(ps aux | grep -E 'target/debug/arpagona' | grep -v grep | wc -l)
if [ "$STALE_COUNT" -gt 0 ]; then
  echo -e "${YELLOW}Summary: $STALE_COUNT stale process(es) detected.${NC}"

  if [ "$KILL_MODE" = true ]; then
    echo ""
    echo -e "${RED}[!] Kill mode${NC}"
    echo -n "Kill all arpagona processes (y/N)? "
    read -r confirm
    if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
      pids=$(ps aux | grep -E 'target/debug/arpagona' | grep -v grep | awk '{print $2}')
      for pid in $pids; do
        echo "  Killing PID $pid ..."
        kill "$pid" 2>/dev/null || echo "    (already gone)"
      done
      # Wait briefly, then SIGKILL survivors
      sleep 1
      survivors=$(ps aux | grep -E 'target/debug/arpagona' | grep -v grep | awk '{print $2}')
      for pid in $survivors; do
        echo "  Force-killing PID $pid ..."
        kill -9 "$pid" 2>/dev/null || true
      done
      echo -e "${GREEN}Done.${NC}"
    else
      echo "  Skipped."
    fi
  else
    echo ""
    echo "  Run with --kill to terminate stale processes interactively."
  fi
else
  echo -e "${GREEN}Summary: No stale arpagona processes.${NC}"
fi

echo ""
