# Compute Reservoir

## Role

Le Compute Reservoir choisit comment penser ou traiter une tâche.

Il répond à la question : quelle ressource cognitive ou computationnelle doit être utilisée pour ce traitement, pourquoi, avec quel coût attendu, quelle latence, quel niveau de confidentialité et quel fallback ?

Il ne décide pas si une action peut être exécutée.
Il ne remplace pas le Decision Gate.
Il ne remplace pas Graph Memory.
Il ne remplace pas le Tool Registry.

Le Decision Gate décide si une action proposée peut être approuvée, bloquée, reroutée ou soumise à validation humaine.

Graph Memory conserve les faits, relations, sources, épisodes et décisions importantes.

Le Tool Registry déclare les outils disponibles, leurs schémas, permissions et risques.

## Responsibilities

Le Compute Reservoir gère en alpha minimal :

- inventory compute resources;
- local/cloud routing;
- privacy constraints;
- cost and latency estimation;
- task capability matching;
- fallback planning;
- performance telemetry shape without persistence.

Les model profiles détaillés, la mémoire de performance persistante et l'intégration runtime viendront plus tard.

Il permet déjà de préférer une ressource locale lorsque les données sont sensibles, de limiter les coûts cloud, de choisir une ressource plus forte lorsque la tâche le justifie et que la policy l'autorise, et de préparer une télémétrie future sans l'enregistrer.

## Alpha Minimal Crate

Le crate `crates/compute-reservoir` existe.

Statut : alpha minimal.

Il fournit des types sérialisables et une fonction pure :

```rust
allocate_compute(request, nodes, policy) -> ComputeAllocation
```

Cette fonction est déterministe et ne fait aucun I/O, aucun appel modèle, aucun réseau, aucune persistence, aucune création de `ProposedAction` et aucune évaluation Decision Gate.

## Current Types

Types actuels :

- `ComputeNodeId`
- `ComputeNode`
- `ComputeResourceKind`
- `ComputeNodeStatus`
- `ComputeCapability`
- `DataSensitivity`
- `ComputeRequest`
- `ComputeAllocation`
- `ComputeFallback`
- `ComputeBudget`
- `ComputePolicy`
- `ComputeTelemetry`

Ces types sont volontairement simples. Ils représentent une allocation de traitement, pas une permission d'agir.

## V0 Routing Rules

Règles V0 implémentées :

- sensitive data -> local preferred;
- low budget -> local preferred;
- complex reasoning -> strong model allowed;
- unavailable or disabled resource -> never selected;
- unavailable resource -> fallback;
- no acceptable resource -> `NoSuitableResource`.

Ces règles servent à guider l'allocation compute. Elles ne donnent aucun droit d'exécution.

Le routage high-risk et le preprocessing long task restent des pistes futures. Toute action future reste soumise au Decision Gate.

## Difference from Reservoir Echo

Reservoir Echo:

- short-term cognitive continuity;
- volatile traces;
- activation and decay;
- limited influence on upcoming cognitive cycles;
- no persistent memory responsibility;
- no model routing.

Compute Reservoir:

- resource selection;
- model/worker routing;
- cost/privacy/capability management;
- fallback planning;
- performance telemetry;
- compute budget awareness.

Le Reservoir Echo aide le système à garder une continuité cognitive courte.
Le Compute Reservoir aide le système à choisir la bonne ressource pour traiter une tâche.

Ils sont complémentaires, mais architecturalement distincts.

## Non-Goals for Initial Implementation

La première implémentation du Compute Reservoir ne devra pas inclure :

- exécution d'outils ;
- accès shell ;
- accès secrets ;
- scheduler autonome ;
- browser automation ;
- MCP integration ;
- self-modification ;
- routage permettant de contourner le Decision Gate.

La V0 n'ajoute pas d'endpoint API, ne modifie pas la CLI et ne crée pas de scheduler.

## Governance Boundary

Le Compute Reservoir peut recommander une ressource.

Il ne peut pas approuver une action.
Il ne peut pas exécuter une action.
Il ne peut pas ignorer une politique.
Il ne peut pas exposer un secret au LLM.

Il ne crée pas de `Decision`.
Il ne crée pas de `ProposedAction`.

Tout effet externe futur devra rester contrôlé par :

```text
ProposedAction -> ToolRegistry lookup -> DecisionGate -> Human approval if needed -> Controlled execution -> Audit -> Graph update
```
