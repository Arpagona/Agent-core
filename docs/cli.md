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

Équivalent direct :

```bash
cargo run -p arpagona-api-server
```

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
```
