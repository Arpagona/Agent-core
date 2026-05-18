# API Server alpha

## Rôle

`arpagona-api-server` expose une vertical slice HTTP locale pour l'alpha installable d'ARPAGONA Agent Core.

Le serveur permet uniquement de :

- créer des `Task` ;
- créer des `ProposedAction` ;
- évaluer une `ProposedAction` via le `DecisionGate` pur Rust ;
- stocker en mémoire les `Decision` et `AuditEvent` produits ;
- consulter l'état courant.

Il ne fait pas d'exécution réelle. Une action proposée reste une intention structurée jusqu'à décision humaine ou système ultérieure. Cette étape ne contient aucun LLM, aucun shell, aucun scheduler, aucun outil exécutable et aucune dépendance obligatoire à SurrealDB.

## Lancement

```bash
cargo run -p arpagona-api-server
```

Le serveur écoute par défaut sur :

```text
127.0.0.1:3000
```

## Endpoints disponibles

```text
GET  /health
POST /tasks
GET  /tasks
POST /proposed-actions
GET  /proposed-actions
POST /decision-gate/evaluate
GET  /decisions
GET  /audit
```

## Exemples curl

### Health

```bash
curl http://127.0.0.1:3000/health
```

Réponse attendue :

```json
{
  "status": "ok",
  "service": "arpagona-api-server"
}
```

### Créer une task

```bash
curl -X POST http://127.0.0.1:3000/tasks \
  -H "Content-Type: application/json" \
  -d '{"workspace_id":"workspace-alpha","title":"Préparer une réponse client","description":"Rédiger une proposition de réponse sans l envoyer."}'
```

La task créée reçoit un ID lisible du type `task-1`.

### Créer une proposed action

```bash
curl -X POST http://127.0.0.1:3000/proposed-actions \
  -H "Content-Type: application/json" \
  -d '{"workspace_id":"workspace-alpha","task_id":"task-1","proposed_by":"agent-alpha","action_type":"simulate_email","target":"client@example.com","risk_level":"medium","required_permissions":["simulate_email"],"rationale":"Préparer un brouillon email sans l envoyer"}'
```

L'action créée reçoit un ID lisible du type `action-1` et le statut `pending_decision`.

Important : cet endpoint ne simule pas, n'envoie pas et n'exécute rien. Il stocke seulement la `ProposedAction`.

### Évaluer via Decision Gate

```bash
curl -X POST http://127.0.0.1:3000/decision-gate/evaluate \
  -H "Content-Type: application/json" \
  -d '{"proposed_action_id":"action-1","granted_permissions":["simulate_email"]}'
```

Réponse :

```json
{
  "decision": {
    "id": "decision-action-1",
    "proposed_action_id": "action-1",
    "status": "needs_human_approval",
    "reason": "...",
    "risk_level": "medium",
    "policies_applied": [],
    "decided_by": "system",
    "created_at": "..."
  },
  "audit_event": {
    "id": "audit-decision-action-1",
    "event_type": "decision_created",
    "actor": "system",
    "workspace_id": "workspace-alpha",
    "task_id": "task-1",
    "proposed_action_id": "action-1",
    "decision_id": "decision-action-1",
    "payload": {},
    "created_at": "..."
  }
}
```

Le statut de la `ProposedAction` est mis à jour selon la décision (`approved`, `blocked` ou `needs_human_approval`).

### Lire l'audit

```bash
curl http://127.0.0.1:3000/audit
```

### Lire les décisions

```bash
curl http://127.0.0.1:3000/decisions
```

### Lire les tasks et proposed actions

```bash
curl http://127.0.0.1:3000/tasks
curl http://127.0.0.1:3000/proposed-actions
```

## Limites alpha

- Stockage uniquement in-memory : les données disparaissent au redémarrage.
- Pas de création de workspace dédiée dans cette étape ; `workspace_id` est porté par les payloads.
- Pas de policy store HTTP ; l'évaluation appelle le Decision Gate avec une liste de policies vide et les permissions accordées par le payload.
- Pas de validation humaine interactive ; `needs_human_approval` est seulement enregistré et consultable.
- Pas de LLM, pas de shell, pas de scheduler, pas de Mission Control, pas d'exécution d'outil.
- Pas de SurrealDB obligatoire.
