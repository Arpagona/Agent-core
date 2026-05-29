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
cargo run -p arpagona-cli -- status --json
```

`--json` émet le même readback sous forme structurée pour scripts locaux, rapports et futures surfaces Mission Control, sans ajouter d'endpoint ni modifier l'état.

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

### Auth — Statut et configuration OpenAI

```bash
cargo run -p arpagona-cli -- auth status
cargo run -p arpagona-cli -- auth openai
```

Sous-commandes :

- `status` — Vérifie si les variables d'environnement OpenAI sont configurées (`OPENAI_API_KEY`).
- `openai` — Affiche les instructions sécurisées pour configurer l'authentification OpenAI.

Exemple de sortie :

```text
ARPAGONA OpenAI Auth Status
openai_api_key: not configured
```

L'authentification est limitée à la clé API en alpha. OAuth complet est post-alpha.

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

### Cognitive Work Loop V0 (P4 — Working Memory / P5 — Compute Reservoir)

```bash
cargo run -p arpagona-cli -- cognitive run --objective "Analyse les journaux d'erreur" --domain coding
```

Options :

- `--objective <TEXT>` (obligatoire) — Le texte de l'objectif à traiter.
- `--domain <DOMAIN>` — Classification optionnelle du domaine (`coding`, `research`, `teaching`, `business`).
- `--context <TEXT>` — Contexte supplémentaire au format `clé:valeur`, une par ligne.
- `--json` — Sortie structurée JSON au lieu du texte lisible.
|- `--assess` — Pont d'évaluation : convertit les `ImprovementCandidates` en `FailureInsightCandidates`.
|- `--allocate` — Pont d'allocation Compute Reservoir : associe la mémoire de travail à une sélection de ressource.
|- `--resonate` — Pont de résonance HolographicMemory : génère des indices de motifs non autorisants à partir de l'état cognitif (domaine, sensibilité, complexité, prochaine action, allocation).
|- `--observe` — Pont d'observation outil : exécute la lecture d'outils (read_file, list_files, search_text) pour les observations requises.
|- `--llm` — Synthèse LLM : appelle un fournisseur LLM pour enrichir la sortie du cycle cognitif.
|- `--provider <NAME>` — Fournisseur LLM (mock, openai, ollama). Défaut: ollama.

Le Work Loop produit une mémoire de travail (WorkingMemory) complète avec objectif, contexte, hypothèses, contraintes, contexte manquant, estimation de sensibilité/complexité, candidats d'amélioration, plan et prochaine action proposée. Les flags `--assess`, `--allocate`, `--resonate`, `--observe` et `--llm` peuvent être combinés pour une exécution en pipeline unique.

### Neutral Orchestrator — Cycle déterministe

```bash
cargo run -p arpagona-cli -- orchestrator run --objective "Inspecter l'état du projet"
cargo run -p arpagona-cli -- orchestrator run --objective "Inspecter l'état du projet" --json
cargo run -p arpagona-cli -- orchestrator run --objective "Inspecter l'état du projet" --json --trace
```

Options :

- `--objective <TEXT>` (obligatoire) — Objectif à traiter dans un cycle orchestré.
- `--json` — Sortie structurée JSON.
- `--trace` — Ajoute le `CycleTrace` complet avec métadonnées d'assemblage de contexte ; utile avec `--json`.
- `--perm <PERMISSION>` — Permissions accordées au Decision Gate, répétable. Défaut : `ReadDocument`.
- `--workspace-id <ID>` — Workspace du cycle. Défaut : `workspace-alpha`.
- `--agent-id <ID>` — Agent émetteur du cycle. Défaut : `agent-alpha`.
- `--proposal-generator <BACKEND>` — Backend de génération de proposition : `simulated` (déterministe, par défaut) ou `llm` (via fournisseur LLM en mode proposition uniquement). Défaut : `simulated`.

La commande exécute le squelette local du Neutral Orchestrator : objectif → assemblage de contexte consultatif → routage compute → proposition → Decision Gate → issue orchestrée. Le résultat reste explicitement non autorisant : il ne planifie pas de scheduler, n'exécute pas d'outil externe, ne crée pas d'approbation durable et ne remplace pas le Decision Gate.

Options supplémentaires pour `orchestrator run` :

- `--save-trace <PATH>` — Sauvegarde le `CycleTrace` complet au format JSON dans un fichier. Utilisez cette option pour capturer le breakdown compute-aware (route calculée, items par source de contexte) pour consultation ultérieure via `orchestrator status`.

#### Orchestrator Status — Lecture du breakdown compute-aware

```bash
cargo run -p arpagona-cli -- orchestrator status
cargo run -p arpagona-cli -- orchestrator status --json
cargo run -p arpagona-cli -- orchestrator status --trace-path target/last-orchestrator-trace.json
```

Options :

- `--json` — Sortie structurée JSON du CycleTrace complet.
- `--trace-path <PATH>` — Chemin vers un fichier `CycleTrace` JSON sauvegardé. Défaut : `target/last-orchestrator-trace.json`.

La commande lit un `CycleTrace` précédemment sauvegardé (via `orchestrator run --save-trace`) et affiche le breakdown complet du contexte : nombre d'items par source, route compute sélectionnée et sa justification, statut décisionnel et résumé du cycle.

Exemple de boucle complète :

```bash
# 1. Exécuter le cycle et sauvegarder la trace
cargo run -p arpagona-cli -- orchestrator run \
  --objective "Analyser le projet" \
  --trace --save-trace target/last-orchestrator-trace.json

# 2. Consulter la trace sauvegardée (même dans une invocation séparée)
cargo run -p arpagona-cli -- orchestrator status --json
```

La trace et son readback restent explicitement non autorisants (`non_authorizing: true`).

### Graph Memory — Statut et démos

```bash
cargo run -p arpagona-cli -- memory status
cargo run -p arpagona-cli -- memory status --json
```

Affiche l'état alpha de Graph Memory et la lecture des conventions.

#### Démos Memory

```bash
cargo run -p arpagona-cli -- memory demo failure-insight
cargo run -p arpagona-cli -- memory demo failure-insight --json
cargo run -p arpagona-cli -- memory demo failure-insight --description "insight personnalisé" --json
```

Options `failure-insight` :

- `--json` — Sortie structurée JSON.
- `--description <TEXT>` — Description personnalisée d'un échec pour remplacer la valeur par défaut.
- `--inspect-id <ID>` — Inspecte un FailureInsight spécifique après la démo.
- `--snapshot-path <PATH>` — Chemin pour écrire un snapshot JSON de preuve de lecture inter-invocation.

La démo exécute la boucle alpha complète : signal → `ProposedAction` → Decision Gate → audit → persistance locale → readback avec preuve de trace.

#### Démos Snapshot (persistance inter-invocation)

```bash
cargo run -p arpagona-cli -- memory demo snapshot-list
cargo run -p arpagona-cli -- memory demo snapshot-list --json
cargo run -p arpagona-cli -- memory demo snapshot-read <snapshot-path>
```

#### Propositions d'écriture mémoire

```bash
cargo run -p arpagona-cli -- memory proposals
cargo run -p arpagona-cli -- memory proposal <proposed-action-id>
```

### Tool Runtime Alpha (lecture seule)

```bash
cargo run -p arpagona-cli -- tool list --json
cargo run -p arpagona-cli -- tool inspect read_file --json
```

Le Tool Runtime expose des outils de perception cognitive read-only :

- `read_file` — Lire un fichier dans le workspace.
- `list_files` — Lister les fichiers d'un répertoire du workspace.
- `search_text` — Chercher un motif textuel dans les fichiers du workspace.

Démos d'exécution d'outils :

```bash
cargo run -p arpagona-cli -- tool demo read-file PROJECT_STATUS.md --json
cargo run -p arpagona-cli -- tool demo list-files . --json
cargo run -p arpagona-cli -- tool demo search-text "Decision Gate" . --json
```

Toute tentative de lire en dehors du workspace (chemins absolus, `..`, `.env`, `.git`) est bloquée avec une erreur structurée et un marqueur de sécurité.

### Insight — Schéma Failure-to-Insight

```bash
cargo run -p arpagona-cli -- insight schema
cargo run -p arpagona-cli -- insight schema --json
```

Affiche la taxonomie et les conventions de lecture du vocabulaire Failure-to-Insight : types de signaux, cibles de correction, catégories de candidats, sans autorisation ni mutation.

### Executor — État du registre d'exécuteurs

```bash
cargo run -p arpagona-cli -- executor list
cargo run -p arpagona-cli -- executor list --json
cargo run -p arpagona-cli -- executor list --offline
cargo run -p arpagona-cli -- executor list --offline --state-file state.json
```

Sous-commandes :

- `list` — Liste tous les exécuteurs enregistrés avec leur état actuel.
- `inspect <EXECUTOR_ID>` — Affiche les détails d'un exécuteur spécifique.

Options communes :

- `--json` — Sortie structurée JSON.
- `--offline` — Interroge l'état local depuis le crate core sans connexion au serveur API. Affiche seulement l'état statique du registre par défaut (NoopExecutor désactivé). Peut être combiné avec `--state-file` pour charger des transitions d'état persistées.
- `--state-file <PATH>` — Chemin vers un fichier JSON d'état persistant des exécuteurs, appliqué par-dessus le registre par défaut en mode offline. Format : `{"executor_id": "disabled"|"ready"|"blocked"}`.

La commande est une surface de supervision alpha read-only. Elle n'exécute, n'approuve, ni ne modifie aucun exécuteur.

### MCP Server — Serveur MCP natif (transport stdio)

```bash
cargo run -p arpagona-cli -- mcp-server
cargo run -p arpagona-cli -- mcp-server --workspace /path/to/workspace --name mon-agent
```

Options :

- `--workspace <PATH>` — Chemin du workspace à servir (défaut : répertoire courant).
- `--name <NAME>` — Nom du serveur annoncé aux clients MCP (défaut : `arpagona-mcp`).
- `--version <VERSION>` — Version du serveur annoncée aux clients MCP (défaut : `0.1.0`).

Démarre le serveur MCP natif en mode transport stdio (Phase 1). Le serveur répond aux requêtes `tools/list` et `tools/call` via les outils read-only du Tool Runtime. La gouvernance via DecisionGate pour `tools/call` nécessite l'intégration Phase 2.

Cette commande est une extension alpha expérimentale du Tool Runtime. Elle n'ajoute pas d'exécution non supervisée, d'accès shell, de modification de fichiers, d'autonomie ou d'intégration réseau externe.

### MCP Governance Audit — Lecture des décisions d'audit MCP

```bash
cargo run -p arpagona-cli -- mcp-governance-audit
cargo run -p arpagona-cli -- mcp-governance-audit --json
cargo run -p arpagona-cli -- mcp-governance-audit --audit-path target/mcp-audit.jsonl --limit 50
```

Options :

- `--audit-path <PATH>` — Chemin vers le fichier journal d'audit de gouvernance MCP (défaut : `target/mcp-audit.jsonl`).
- `--limit <N>` — Nombre maximum d'entrées récentes à afficher (défaut : 20).
- `--json` — Sortie structurée JSON.

Lit et affiche les décisions d'audit de gouvernance MCP récentes depuis un fichier JSONL persistant. Chaque entrée documente une décision `tools/call` passée par le Decision Gate du serveur MCP : outil appelé, arguments, risque, résultat de la décision (allowed/blocked), horodatage et identifiant d'audit.

Cette commande est une surface de supervision alpha read-only. Les entrées d'audit sont produites par le serveur MCP lors de l'évaluation de `tools/call` par le Decision Gate ; la commande ne fait que les lire.

### LLM — Journal d'interaction LLM (C3)

```bash
cargo run -p arpagona-cli -- llm journal
cargo run -p arpagona-cli -- llm journal --json
cargo run -p arpagona-cli -- llm journal --limit 20
```

Sous-commandes :

- `journal` — Affiche les entrées récentes du journal d'interaction LLM.

Options `journal` :

- `--limit <N>` — Nombre maximum d'entrées récentes à afficher (défaut : 10).
- `--json` — Sortie structurée JSON.

Affiche les traces récentes d'interaction LLM : résumé du prompt, résumé de la réponse, fournisseur/modèle utilisé, actions proposées ou intentions d'appel d'outil émises par le LLM, résultat du Decision Gate, niveau de risque et horodatage.

Cette commande est une surface de supervision alpha read-only. Elle n'exécute aucun appel LLM, n'approuve aucune action et ne modifie aucun état.

### Orchestrator — Cycle orchestré (Phase 3)

```bash
cargo run -p arpagona-cli -- orchestrator run --objective "Analyser les tendances du marché"
cargo run -p arpagona-cli -- orchestrator run --objective "Analyse projet" --json --trace
cargo run -p arpagona-cli -- orchestrator run --objective "Code review" --proposal-generator llm
cargo run -p arpagona-cli -- orchestrator status
cargo run -p arpagona-cli -- orchestrator status --json
```

Sous-commandes `orchestrator` :

- `run` — Exécute un cycle orchestré complet : contexte → allocation compute → proposition → Decision Gate → audit.
- `status` — Affiche le dernier CycleTrace sauvegardé (lecture cross-invocation).

Options `run` :

- `--objective <TEXT>` — L'objectif à traiter par le cycle (obligatoire).
- `--json` (ou `-j`) — Sortie structurée JSON.
- `--trace` — Affiche le CycleTrace complet avec les métadonnées d'assembly contexte (nombre d'items par source, échantillons, sources indisponibles).
- `--save-trace <PATH>` — Chemin de sauvegarde explicite (par défaut : auto-sauvegardé dans `target/last-orchestrator-trace.json`).
- `--proposal-generator <simulated|llm>` — Backend de génération de proposition (défaut : `simulated`).
- `--perm <PERMISSION>` — Permissions accordées pour l'évaluation Decision Gate (répétable, défaut : `ReadDocument`).
- `--workspace-id <ID>` — Identifiant du workspace (défaut : `workspace-alpha`).
- `--agent-id <ID>` — Identifiant de l'agent (défaut : `agent-alpha`).

Options `status` :

- `--json` — Sortie structurée JSON du CycleTrace complet.
- `--trace-path <PATH>` — Chemin vers un fichier CycleTrace JSON sauvegardé (défaut : `target/last-orchestrator-trace.json`).

Le trace est automatiquement sauvegardé après chaque `orchestrator run` dans `target/last-orchestrator-trace.json`, permettant une lecture cross-invocation via `orchestrator status` sans option supplémentaire.

Chaîne du cycle :

```text
ObjectiveInput → ContextBundle → ComputeRouteResult → ProposalRequest → ProposedAction → Decision Gate → AuditEvent → OrchestratorOutcome
```

Tous les champs sont consultatifs (advisory) et non autorisants. La sortie du Decision Gate et les événements d'audit portent l'état réel de gouvernance.

Si aucun trace n'a encore été sauvegardé, `orchestrator status` affiche un message d'aide au lieu d'une erreur.

## Installation

```bash
cargo install --path crates/cli
arpagona health
arpagona chat --provider mock
```