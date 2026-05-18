# README Alpha — ARPAGONA Agent Core

Cette alpha expose une vertical slice locale : création de tâche, proposition d'action, évaluation par Decision Gate, événement d'audit consultable en CLI.

## Périmètre livré

- Serveur API Axum minimal : `arpagona-api-server`.
- CLI installable : binaire `arpagona`, package Cargo `arpagona-cli`.
- Stockage alpha en mémoire dans le serveur API.
- Aucun appel LLM, aucun envoi email réel, aucune exécution d'outil.

## Démo complète

Terminal 1 :

```bash
cargo run -p arpagona-cli -- serve
# équivalent alpha direct : cargo run -p arpagona-api-server
```

Terminal 2 :

```bash
cargo run -p arpagona-cli -- health
cargo run -p arpagona-cli -- task create "Préparer une réponse client"
cargo run -p arpagona-cli -- action propose --type simulate_email --risk medium
cargo run -p arpagona-cli -- action evaluate action-1
cargo run -p arpagona-cli -- audit list
```

Sorties attendues principales :

```text
ARPAGONA API: ok

Created task: task-1
Title: Préparer une réponse client

Created proposed action: action-1
Status: pending_decision

Decision: needs_human_approval
Audit: audit-decision-action-1
```

## Installation du binaire CLI

Pendant le développement, utiliser :

```bash
cargo run -p arpagona-cli -- health
```

Quand le binaire `arpagona` est installé :

```bash
cargo install --path crates/cli
arpagona health
```

## Configuration

Par défaut, la CLI appelle :

```text
http://127.0.0.1:3000
```

Override possible :

```bash
ARPAGONA_API_URL=http://127.0.0.1:3000 arpagona health
# ou
arpagona --api-url http://127.0.0.1:3000 health
```

## Commandes disponibles

```bash
arpagona serve
arpagona health
arpagona task create "Titre" [--description "..."] [--workspace-id workspace-alpha]
arpagona action propose [--type simulate_email] [--risk medium] [--task-id task-1] [--target client@example.com] [--rationale "Préparer un brouillon sans l’envoyer"] [--permission simulate_email]
arpagona action evaluate action-1 [--permission simulate_email]
arpagona audit list
```

## Limites alpha

- Données perdues au redémarrage du serveur : stockage in-memory.
- `serve` délègue à `cargo run -p arpagona-api-server` pour l’alpha.
- Pas d'exécution réelle d'email : `simulate_email` ne fait que proposer une action.
- Pas d'authentification/API key dans cette vertical slice locale.
