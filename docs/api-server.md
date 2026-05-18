# API Server alpha

## Rôle

`arpagona-api-server` expose une vertical slice HTTP locale pour l'alpha installable d'ARPAGONA Agent Core.

Le serveur permet uniquement de :

- créer des `Task` ;
- créer des `ProposedAction` ;
- demander à un provider agentique expérimental de proposer une `ProposedAction` pending ;
- évaluer une `ProposedAction` via le `DecisionGate` pur Rust ;
- stocker en mémoire les `Decision` et `AuditEvent` produits ;
- consulter l'état courant.

Il ne fait pas d'exécution réelle. Une action proposée reste une intention structurée jusqu'à décision humaine ou système ultérieure. Le provider LLM expérimental ne peut que générer une proposition JSON, stockée avec le statut `pending_decision`; il n'appelle aucun outil, aucun web search et ne contourne jamais le Decision Gate.

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
POST /agent/propose
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

### Proposer une action via Agent Proposer V0

```bash
curl -X POST http://127.0.0.1:3000/agent/propose \
  -H "Content-Type: application/json" \
  -d '{"workspace_id":"workspace-alpha","task_id":"task-1","prompt":"Prépare un brouillon de réponse client pour expliquer que nous allons envoyer un devis.","provider":"openai"}'
```

Le provider `openai` lit `OPENAI_API_KEY`, accepte `OPENAI_MODEL` et produit uniquement une proposition structurée. Pour les tests locaux sans réseau, `provider":"mock"` retourne une proposition déterministe.

Réponse :

```json
{
  "proposed_action": {
    "id": "action-1",
    "workspace_id": "workspace-alpha",
    "task_id": "task-1",
    "proposed_by": "agent-proposer-v0",
    "action_type": "simulate_email",
    "target": "client-response-draft",
    "payload": {},
    "risk_level": "low",
    "required_permissions": ["simulate_email"],
    "rationale": "...",
    "context_refs": [],
    "status": "pending_decision",
    "created_at": "..."
  }
}
```

Important : `/agent/propose` ne déclenche pas `/decision-gate/evaluate`. Il ne fait que stocker la `ProposedAction` pending.

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
- LLM expérimental limité à la proposition d'action ; pas de shell, pas de scheduler, pas de Mission Control, pas d'exécution d'outil.
- Pas de SurrealDB obligatoire.
