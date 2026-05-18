#!/usr/bin/env bash

# Démo alpha ARPAGONA Agent Core.
# Prérequis : le serveur API tourne déjà sur ${ARPAGONA_API_URL:-http://127.0.0.1:3000}.
# Exemple serveur : cargo run -p arpagona-api-server

set -euo pipefail

API_URL="${ARPAGONA_API_URL:-http://127.0.0.1:3000}"
CLI=(cargo run -q -p arpagona-cli -- --api-url "$API_URL")

step() {
  printf '\n==> %s\n' "$1"
}

fail() {
  printf 'ERREUR: %s\n' "$1" >&2
  exit 1
}

extract_id() {
  local label="$1"
  sed -n "s/^${label}: //p" | head -n 1
}

step "Vérification API"
if ! health_output=$("${CLI[@]}" health 2>&1); then
  printf '%s\n' "$health_output" >&2
  fail "API indisponible sur $API_URL. Lance d'abord: cargo run -p arpagona-api-server"
fi
printf '%s\n' "$health_output"

step "Création tâche"
task_output=$("${CLI[@]}" task create "Préparer une réponse client")
printf '%s\n' "$task_output"
task_id=$(printf '%s\n' "$task_output" | extract_id "Created task")
[ -n "$task_id" ] || fail "impossible d'extraire l'id de tâche depuis la sortie CLI"

step "Proposition d'action simulée"
action_output=$("${CLI[@]}" action propose --type simulate_email --risk medium --task-id "$task_id")
printf '%s\n' "$action_output"
action_id=$(printf '%s\n' "$action_output" | extract_id "Created proposed action")
[ -n "$action_id" ] || fail "impossible d'extraire l'id d'action depuis la sortie CLI"

step "Évaluation Decision Gate"
"${CLI[@]}" action evaluate "$action_id"

step "Consultation audit"
"${CLI[@]}" audit list

printf '\nDémo alpha terminée. Aucune action réelle n’a été exécutée.\n'
