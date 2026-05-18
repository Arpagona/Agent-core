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
/tasks                Lister les tâches
/actions              Lister les actions proposées
/evaluate action-1    Évaluer une action via Decision Gate
/audit                Lister les événements d'audit
/provider mock        Basculer sur le provider mock
/provider openai      Basculer sur le provider OpenAI
```

Tout autre texte est envoyé à `/agent/propose`. Le résultat est une `ProposedAction` avec `pending_decision`. Rien n'est exécuté directement.

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

## Installation

```bash
cargo install --path crates/cli
arpagona health
arpagona chat --provider mock
```