#!/usr/bin/env bash
#
# Demo — Boucle cognitive complète P5 + P6
# ARPAGONA Agent Core
#
# Prérequis : cargo dans le PATH, workspace compilé.
# Aucun serveur API nécessaire — tout est local.
#
set -euo pipefail

CLI=(cargo run -q --bin arpagona)

step() {
  printf '\n═══════════════════════════════════════════════\n'
  printf '  >>> %s\n' "$1"
  printf '═══════════════════════════════════════════════\n'
}

json_step() {
  printf '\n─── %s ───\n' "$1"
}

separator() {
  printf '\n───────────────────────────────────────────────\n'
}

step "1. Boucle cognitive — objectif business confidentiel"

"${CLI[@]}" cognitive run \
  --objective "Développer une stratégie de prospection pour ARPAGONA sur le marché français de l'IA" \
  --domain business \
  --assess \
  --allocate \
  --json

separator

step "2. Boucle cognitive — objectif recherche complexe"

"${CLI[@]}" cognitive run \
  --objective "Analyser les performances des modèles LLM sur des benchmarks de raisonnement multi-étapes" \
  --domain research \
  --assess \
  --allocate \
  --json

separator

step "3. Boucle cognitive — objectif sensible avec données privées"

"${CLI[@]}" cognitive run \
  --objective "Préparer une analyse confidentielle des salaires pour le département RH" \
  --domain administration \
  --assess \
  --allocate \
  --json

separator

step "4. Boucle cognitive — objectif codage simple (lecture seule)"

"${CLI[@]}" cognitive run \
  --objective "Implémenter une fonction Rust de parsing CSV avec gestion d'erreurs" \
  --domain coding \
  --assess \
  --allocate \
  --json

separator

step "5. Tests d'intégration chaîne complète"

cargo test -p arpagona-compute-reservoir p6_integration 2>&1

separator

echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║   DÉMO TERMINÉE                                      ║"
echo "║                                                       ║"
echo "║   Aucun LLM, aucune API, aucune persistence,          ║"
echo "║   aucune autorisation n'ont été utilisées.            ║"
echo "║   La sortie est une readback non-autorisante.         ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""
