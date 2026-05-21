# Graph Memory

Graph Memory est la couche de mémoire structurée d'ARPAGONA Agent Core.

Elle conserve les éléments qui devront être explorables par le runtime agentique : sources, faits, épisodes, observations, relations et événements d'audit. Elle ne décide pas et n'exécute rien.

Le flux non négociable reste :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Graph Memory persiste le contexte et les traces. Elle ne remplace ni l'orchestrateur, ni le Decision Gate.

## Source de vérité domaine

Le contrat domaine canonique vit dans `crates/core` :

- `GraphMemoryStore` : trait synchrone, pur Rust, sans base externe ;
- `InMemoryGraphMemoryStore` : implémentation en mémoire pour tests et développement du domaine ;
- `GraphRef`, `GraphRelation` et `RelationType` : représentation minimale des liens graphe ;
- types domaine stockés : `Source`, `Fact`, `Episode`, `Observation`, `AuditEvent`.

`crates/core` doit rester indépendant de SurrealDB, Axum, Tokio runtime, LLM, shell et exécution d'outils.

## Store mémoire pur

`InMemoryGraphMemoryStore` permet de tester Graph Memory sans infrastructure :

- création et lecture de sources ;
- ajout et récupération de faits, dont faits actifs par entité ;
- ajout et récupération d'épisodes ;
- ajout et récupération d'observations liées à un épisode ;
- stockage d'événements d'audit ;
- stockage et lecture de relations `DerivedFrom`, `Supports`, etc.

Ce store est la référence V0 pour le comportement domaine. Il n'a aucune dépendance DB.

## Adapter SurrealDB

Le crate `crates/graph-memory` est l'adapter persistant SurrealDB.

Il expose :

- `SurrealGraphMemoryStore` : adapter SurrealDB ;
- `AsyncGraphMemoryStore` : port async expérimental de l'adapter, volontairement renommé pour ne pas concurrencer le trait domaine `arpagona_core::GraphMemoryStore` ;
- `GraphMemoryError` ;
- `GRAPH_MEMORY_SCHEMA`, basé sur `crates/graph-memory/migrations/0001_graph_memory.surql`.

Le port async existe parce que le client SurrealDB est async, tandis que le contrat domaine V0 reste synchrone pour garder `crates/core` simple et pur. La source de vérité conceptuelle reste le trait `GraphMemoryStore` de `crates/core`.

## Entités du schéma SurrealDB V0

La migration initiale crée une base pour :

- `fact` : faits attachés à une entité (`entity_type`, `entity_id`) ;
- `source` : origine documentaire, utilisateur, import, système ou API ;
- `episode` : séquence contextualisée dans un workspace ;
- `observation` : observation issue d'un épisode ;
- `graph_relation` : relation minimale entre deux références graphe ;
- `audit_event` : événement d'audit consultable par workspace ;
- `decision` : décision produite plus tard par le Decision Gate ;
- `proposed_action` : action proposée par un agent, sans exécution directe.

Les structs de `crates/core` sont stockées dans un champ `data` JSON SurrealDB. Certains champs sont dupliqués (`entity_type`, `entity_id`, `workspace_id`, `episode_id`, `from_node_type`, `from_node_id`, `created_at`, etc.) pour permettre les premières requêtes et index.

## Limites restantes

L'adapter SurrealDB est maintenant aligné sur les grandes entités du contrat core (`Source`, `Fact`, `Episode`, `Observation`, `AuditEvent`, `GraphRelation`), mais il reste expérimental :

- il n'implémente pas directement le trait synchrone `arpagona_core::GraphMemoryStore`, afin d'éviter de cacher un runtime async dans le core ;
- il ne persiste pas encore `ProposedAction` et `Decision` via une API dédiée ;
- les relations graphe restent simples et stockées comme documents, pas encore comme traversées SurrealDB avancées ;
- il n'y a pas encore de migration runner dédié ;
- il n'y a pas encore de stratégie de versionnement de schéma ;
- il n'y a pas de Decision Gate ;
- il n'y a pas d'API Axum, de Mission Control, de LLM ou d'exécution d'outils.

## Governed memory-write proposal vocabulary

The first alpha memory-write integration step is proposal vocabulary, not persistence. New memory-changing intents should be represented as specific `ProposedAction` action types before any state is changed:

```text
create_memory_fact
link_memory_fact
invalidate_memory_fact
create_failure_insight_memory
```

These variants refine the legacy coarse `write_memory` action type. They still require `Permission::WriteMemory`, and they still pass through:

```text
ProposedAction -> DecisionGate -> Decision -> Audit
```

`MemoryWriteIntent` carries the minimum metadata required for governed future writes: typed target, provenance/source, confidence, proposing actor, reason for remembering, proposal timestamp, optional decision/audit linkage and an invalidation/supersession note. Creating this intent is non-mutating. It does not insert a fact, create a relation, persist a FailureInsight, approve a write or authorize future recall.

The intended first-alpha behavior is conservative:

- missing `WriteMemory` permission blocks the proposal with explanatory audit;
- medium or higher memory-write risk requires human confirmation unless future explicit policy says otherwise;
- Graph Memory persistence may only be added later after the proposal, permission, decision and audit path remains covered by tests.

## Read-only CLI status

The alpha CLI exposes a bounded Graph Memory status readback:

```bash
arpagona memory status
arpagona memory status --json
```

This command reports whether Graph Memory support is compiled into the CLI, the expected alpha backend (`surrealdb`), whether a backend name was configured through `ARPAGONA_GRAPH_MEMORY_BACKEND`, whether the SurrealDB adapter and schema are available, alpha limitations and intentionally missing capabilities.

It is strictly read-only. It does not initialize a database, run migrations, create facts, persist observations, approve actions, authorize memory writes, inject context into LLM prompts or execute tools.

## Tests

Les tests du core restent indépendants de SurrealDB.

Les tests de `crates/graph-memory` utilisent SurrealDB en mémoire et couvrent :

- initialisation du schéma ;
- insertion, lecture, liste et révocation de `Fact` ;
- insertion et lecture de `Source` ;
- insertion et lecture de `Episode` ;
- insertion, lecture et liste de `Observation` par épisode ;
- insertion et lecture de `GraphRelation` ;
- enregistrement et liste de `AuditEvent` par workspace ;
- sérialisation/désérialisation des types core.

Commande depuis la racine :

```bash
cargo test
```

## Prochaine étape : Decision Gate

La prochaine brique doit utiliser Graph Memory comme contexte et audit, sans donner de pouvoir d'exécution aux agents.

Le Decision Gate devra évaluer les `ProposedAction`, produire des `Decision`, appliquer les politiques et enregistrer l'audit. L'exécution éventuelle restera une couche contrôlée séparée.
