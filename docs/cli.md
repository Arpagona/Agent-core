# CLI ARPAGONA Alpha

La CLI alpha est fournie par le crate `crates/cli`.

- Package Cargo : `arpagona-cli`
- Binaire installé : `arpagona`
- API cible par défaut : `http://127.0.0.1:3000`

## Lancer l'API

Via la CLI alpha :

```bash
cargo run -p arpagona-cli -- serve
```

Équivalent direct recommandé pendant l'alpha :

```bash
cargo run -p arpagona-api-server
```

## Interface terminal interactive

```bash
cargo run -p arpagona-cli -- chat --provider mock
```

Après installation :

```bash
arpagona chat --provider mock
```

Le mode `chat` vérifie `/health`, affiche une bannière terminal, puis accepte des demandes utilisateur et des commandes internes.

Provider par défaut du mode chat : `mock`.

Avec OpenAI :

```bash
export OPENAI_API_KEY="..."
arpagona chat --provider openai
```

Commandes internes :

```text
/help                 Afficher l'aide
/quit ou /exit        Quitter
/status               Afficher le cockpit read-only local
/tasks                Lister les tâches
/actions              Lister les actions proposées
/evaluate action-1    Évaluer une action via Decision Gate
/audit                Lister les événements d'audit
/provider mock        Basculer sur le provider mock
/provider openai      Basculer sur le provider OpenAI
```

Tout autre texte est envoyé à `/agent/propose`. Le résultat est une `ProposedAction` avec `pending_decision`. Rien n'est exécuté directement.

Limites alpha du mode terminal :

- Interface ligne par ligne uniquement : pas de TUI plein écran, pas de `ratatui`, pas de `crossterm`.
- Pas de shell, pas de scheduler et pas d'exécution d'outils.
- Le provider propose uniquement ; le Decision Gate reste explicite via `/evaluate`.
- Le provider `mock` est recommandé pour tester sans réseau et sans clé OpenAI.

Documentation dédiée : [`terminal-interface.md`](terminal-interface.md).

## Commandes

### Health

```bash
cargo run -p arpagona-cli -- health
```

Appelle :

```text
GET /health
```

Affiche :

```text
ARPAGONA API: ok
```

### Status local de supervision

```bash
cargo run -p arpagona-cli -- status
```

Appelle les chemins de lecture existants :

```text
GET /health
GET /tasks
GET /proposed-actions
GET /decisions
GET /audit
```

Affiche un cockpit local read-only : santé API, compteurs de tâches, actions proposées, décisions, événements d'audit, décisions en attente, demandes d'approbation humaine, événements d'audit récents et dernier timestamp d'audit connu quand ces données sont disponibles.

Exemple :

```text
ARPAGONA status
api_health: ok
task_count: 1
proposed_action_count: 2
decision_count: 1
audit_event_count: 1
pending_decision_count: 1
needs_human_approval_count: 1
recent_audit_event_count: 1
last_audit_event_at: 2026-01-01T00:00:00+00:00
Readback only: this summary is not approval, authorization, orchestration, or execution state.
```

La commande ne crée aucun endpoint et ne modifie aucun état. Si l'API est indisponible, elle affiche une santé `unavailable` et des compteurs indisponibles sans transformer le readback en autorisation.

### Créer une tâche

```bash
cargo run -p arpagona-cli -- task create "Préparer une réponse client"
```

Appelle :

```text
POST /tasks
```

Payload :

```json
{
  "workspace_id": "workspace-alpha",
  "title": "Préparer une réponse client",
  "description": null
}
```

Affiche notamment :

```text
Created task: task-1
Title: Préparer une réponse client
```

### Proposer une action

```bash
cargo run -p arpagona-cli -- action propose --type simulate_email --risk medium
```

Options :

- `--type simulate_email` par défaut.
- `--risk medium` par défaut.
- `--rationale "Préparer un brouillon sans l’envoyer"` par défaut.
- `--permission simulate_email` par défaut, répétable.
- `--task-id task-1` par défaut.
- `--target client@example.com` par défaut.

Appelle :

```text
POST /proposed-actions
```

Payload par défaut :

```json
{
  "workspace_id": "workspace-alpha",
  "task_id": "task-1",
  "proposed_by": "agent-alpha",
  "action_type": "simulate_email",
  "target": "client@example.com",
  "risk_level": "medium",
  "required_permissions": ["simulate_email"],
  "rationale": "Préparer un brouillon sans l’envoyer",
  "payload": {
    "to": "client@example.com",
    "subject": "Simulation alpha ARPAGONA",
    "body": "Préparer un brouillon sans l’envoyer"
  }
}
```

Affiche notamment :

```text
Created proposed action: action-1
Status: pending_decision
```

### Proposer une action via Agent Proposer V0

```bash
cargo run -p arpagona-cli -- agent propose "Prépare un brouillon de réponse client"
```

Options :

- `--provider openai` par défaut ; utiliser `--provider mock` pour un test sans réseau.
- `--task-id task-1` par défaut.
- `--workspace-id workspace-alpha` par défaut.

Appelle :

```text
POST /agent/propose
```

Affiche :

```text
Proposed action: action-1
Type: simulate_email
Risk: low
Status: pending_decision
```

Cette commande ne déclenche pas le Decision Gate et ne crée aucune `Decision`.

Routage read-only alpha du provider OpenAI : les demandes explicites de lecture d’état runtime sont routées vers des types précis et informationnels (`read_tasks`, `read_proposed_actions`/`read_pending_actions`, `read_decisions`, `read_audit`, `read_status`) plutôt que vers `read_memory` ou `system_check` générique. `read_memory` reste réservé à la mémoire longue durée/cognitive ; `system_check` reste réservé aux vérifications système générales. Les questions de conseil/analyse restent des `DirectReply`.

### Évaluer une action

```bash
cargo run -p arpagona-cli -- action evaluate action-1
```

Options :

- `--permission simulate_email` par défaut, répétable.

Appelle :

```text
POST /decision-gate/evaluate
```

Payload :

```json
{
  "proposed_action_id": "action-1",
  "granted_permissions": ["simulate_email"]
}
```

Affiche :

```text
Decision: needs_human_approval
Audit: audit-decision-action-1
```

### Lister l'audit

```bash
cargo run -p arpagona-cli -- audit list
```

Appelle :

```text
GET /audit
```

Affiche chaque événement de façon lisible :

```text
- id: audit-decision-action-1
  event_type: decision_created
  proposed_action_id: action-1
  decision_id: decision-action-1
```

### Résumé d'audit par décision

```bash
cargo run -p arpagona-cli -- audit decision-summary decision-action-1
cargo run -p arpagona-cli -- audit decision-summary decision-action-1 --json
```

Appelle :

```text
GET /audit
```

La CLI filtre localement les événements correspondant à `decision_id`, les ordonne chronologiquement et affiche un résumé de supervision en lecture seule : portée causale, nombre d'événements, premier/dernier événement, statut de décision, risque, politiques et indicateurs d'approbation humaine/exécution.

### Résumé d'audit par tâche

```bash
cargo run -p arpagona-cli -- audit task-summary task-1
cargo run -p arpagona-cli -- audit task-summary task-1 --json
```

Appelle :

```text
GET /audit
```

La CLI filtre localement les événements correspondant à `task_id`, les ordonne chronologiquement et affiche un résumé de supervision en lecture seule pour la tâche : workspace, première action/décision observée, nombre d'événements, bornes temporelles et indicateurs de proposition, décision, demande humaine et exécution.

### Résumé d'audit par workspace

```bash
cargo run -p arpagona-cli -- audit workspace-summary workspace-alpha
cargo run -p arpagona-cli -- audit workspace-summary workspace-alpha --json
```

Appelle :

```text
GET /audit
```

La CLI filtre localement les événements correspondant à `workspace_id`, les ordonne chronologiquement et affiche un résumé de supervision en lecture seule pour le workspace : première tâche/action/décision observée, nombre d'événements, bornes temporelles et indicateurs de proposition, décision, demande humaine et exécution.

Ces résumés sont explicitement du readback : ils ne valent pas approbation, autorisation, orchestration ou état d'exécution.

## Installation

```bash
cargo install --path crates/cli
arpagona health
arpagona chat --provider mock
```