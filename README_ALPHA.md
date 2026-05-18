# README Alpha — ARPAGONA Agent Core

Cette alpha expose une vertical slice locale : création de tâche, proposition d'action, évaluation par Decision Gate, événement d'audit consultable en CLI.

Flux démontré :

```text
Task -> ProposedAction -> DecisionGate -> Decision -> AuditEvent -> Consultation CLI
```

## Périmètre livré

- Serveur API Axum minimal : `arpagona-api-server`.
- CLI installable : binaire `arpagona`, package Cargo `arpagona-cli`.
- Graph Memory V0 séparée du serveur API alpha.
- Decision Gate alpha.
- Stockage alpha en mémoire dans le serveur API.
- Aucun appel LLM, aucun scheduler, aucun envoi email réel, aucune exécution d'outil.

## Vérification globale

Avant une release alpha, lancer à la racine du workspace :

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

La checklist complète est dans [`docs/alpha-release-checklist.md`](docs/alpha-release-checklist.md).

## Démo complète recommandée

Terminal 1 :

```bash
cargo run -p arpagona-api-server
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

`audit list` doit afficher `audit-decision-action-1`.

## Script de démo

Un script de démo est fourni :

```bash
scripts/demo-alpha.sh
```

Il suppose que le serveur API tourne déjà sur `http://127.0.0.1:3000`, exécute les commandes CLI dans l’ordre, et échoue clairement si l’API ne répond pas.

URL alternative :

```bash
ARPAGONA_API_URL=http://127.0.0.1:3000 scripts/demo-alpha.sh
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

## Note sur `serve` pendant l’alpha

`arpagona serve` est un raccourci alpha qui délègue à :

```bash
cargo run -p arpagona-api-server
```

Il dépend donc de `cargo` et d’une exécution depuis le workspace source. Pour la démo reproductible, préférer lancer explicitement :

```bash
cargo run -p arpagona-api-server
```

## Limites alpha

- Données perdues au redémarrage du serveur : stockage API in-memory.
- IDs déterministes locaux (`task-1`, `action-1`, etc.).
- Pas d'exécution réelle d'email : `simulate_email` ne fait que proposer une action.
- Pas d'exécution réelle d'outil.
- Pas d'authentification/API key dans cette vertical slice locale.
- Pas de persistance serveur obligatoire.
- Pas de LLM, scheduler, Mission Control complet, UI ou validation humaine interactive.
- Pas de store HTTP pour les policies : ces sujets sont post-alpha.
