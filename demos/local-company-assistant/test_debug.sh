#!/usr/bin/env bash
# Debug helper — reads each sample file via the Tool Runtime CLI.
# Uses relative paths (Tool Runtime blocks absolute paths).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROJECT_ROOT/target/debug/arpagona"
# Relative to PROJECT_ROOT — Tool Runtime blocks absolute paths
SAMPLES_REL="demos/local-company-assistant/samples"

for pair in "feedback-customers.md:Fiche Client 1" "operations-snapshot.md:Horaires" "staff-suggestions.md:Participants"; do
  FILE="${pair%%:*}"
  MATCH="${pair##*:}"
  TMPF=$(mktemp)
  "$BINARY" tool demo read-file "$SAMPLES_REL/$FILE" 2>/dev/null > "$TMPF" || true
  S=$(wc -c < "$TMPF")
  if grep -q "$MATCH" "$TMPF"; then
    echo "PASS: found '$MATCH' in $FILE (size $S)"
  else
    echo "FAIL: did not find '$MATCH' in $FILE (size $S)"
  fi
  rm -f "$TMPF"
done