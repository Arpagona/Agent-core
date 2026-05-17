# Graph Memory

Graph Memory est la première couche de persistance structurée d'ARPAGONA Agent Core.

Elle sert à conserver les éléments de mémoire et de traçabilité qui devront être explorables par le runtime agentique : faits, sources, épisodes, observations, décisions, actions proposées et événements d'audit.

## Rôle

Graph Memory n'est pas un orchestrateur et n'exécute aucune action. Son rôle est limité à :

- initialiser un schéma SurrealDB minimal ;
- enregistrer et relire des types du crate `core` ;
- permettre des requêtes simples par entité ou workspace ;
- fournir une base testable pour les briques suivantes.

Le flux non négociable du projet reste inchangé :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Graph Memory ne remplace pas le Decision Gate. Elle persiste seulement les objets nécessaires à la mémoire et à l'audit.

## Séparation avec `crates/core`

`crates/core` reste un crate de types purs. Il ne dépend pas de SurrealDB, d'Axum, d'un LLM, d'un shell ou d'un système d'exécution.

La persistance est donc placée dans un crate séparé :

```text
crates/graph-memory
```

Cette séparation évite de contaminer le modèle domaine avec des détails runtime. Les types restent sérialisables et réutilisables, tandis que Graph Memory porte l'adapter SurrealDB.

## Entités prévues dans le schéma initial

La migration `crates/graph-memory/migrations/0001_graph_memory.surql` crée une première base pour :

- `fact` : faits attachés à une entité (`entity_type`, `entity_id`) ;
- `source` : origine documentaire, utilisateur, import, système ou API ;
- `episode` : séquence contextualisée dans un workspace ;
- `observation` : observation issue d'un épisode ;
- `audit_event` : événement d'audit consultable par workspace ;
- `decision` : décision produite plus tard par le Decision Gate ;
- `proposed_action` : action proposée par un agent, sans exécution directe.

La première API Rust expose notamment :

- `GraphMemoryStore` ;
- `GraphMemoryError` ;
- `SurrealGraphMemoryStore` ;
- `init_schema()` ;
- `upsert_fact()` / `get_fact()` / `list_facts_for_entity()` / `revoke_fact()` ;
- `upsert_source()` / `get_source()` ;
- `record_audit_event()` / `list_audit_events_for_workspace()`.

## Choix de stockage V0

Les structs de `crates/core` sont stockées dans un champ `data` JSON SurrealDB. En V0, ce champ `data` est la source canonique : les objets relus par l'API sont désérialisés depuis ce JSON.

Certains champs sont volontairement dupliqués au niveau du document SurrealDB (`entity_type`, `entity_id`, `workspace_id`, `created_at`, etc.) pour permettre les premières requêtes et poser les futurs index sans complexifier le modèle domaine.

En V0, `created_at` est stocké comme string RFC3339 dans ces champs dupliqués. Cela évite les problèmes de conversion implicite entre `chrono::DateTime<Utc>` et le type `datetime` SurrealDB. Cette représentation pourra évoluer vers un vrai `datetime` SurrealDB quand le mapping de persistance sera stabilisé.

Ce compromis garde la V0 simple :

- pas de mapping ORM complexe ;
- pas de mutation du crate core ;
- round-trip JSON direct sur les types domaine ;
- possibilité d'ajouter des index et relations plus fines ensuite.

## Tests

Les tests du crate utilisent SurrealDB en mémoire et couvrent :

- création du store en mémoire ;
- initialisation du schéma ;
- insertion et lecture d'un `Fact` ;
- révocation d'un `Fact` ;
- insertion et lecture d'une `Source` ;
- enregistrement et liste d'un `AuditEvent` ;
- sérialisation/désérialisation des types `core`.

Commande depuis la racine :

```bash
cargo test
```

## Limites V0

Cette première version est volontairement expérimentale et limitée :

- pas de relations graphe avancées ;
- pas de requêtes transversales complexes ;
- pas de migration runner dédié ;
- pas de stratégie de versionnement des schémas au-delà du fichier initial ;
- pas de Decision Gate ;
- pas d'API Axum ;
- pas de Mission Control ;
- pas de LLM ;
- pas d'exécution d'outils.

## À faire ensuite

Priorités recommandées :

1. Stabiliser les conventions d'identifiants SurrealDB et les index utiles.
2. Ajouter des méthodes de persistance pour `Episode`, `Observation`, `ProposedAction` et `Decision` si les briques suivantes en ont besoin.
3. Définir les relations graphe métier (`supports`, `derived_from`, `contradicts`, etc.).
4. Préparer la Brique 3 — Decision Gate, sans donner aux agents la capacité d'exécuter directement des actions.
