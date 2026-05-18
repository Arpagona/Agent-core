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

Le Compute Reservoir devra gérer :

- inventory compute resources;
- model profiles;
- local/cloud routing;
- privacy constraints;
- cost and latency estimation;
- task capability matching;
- fallback planning;
- performance memory.

Il devra notamment permettre au runtime de préférer une ressource locale lorsque les données sont sensibles, de limiter les coûts cloud, de choisir un modèle plus fort lorsque la tâche le justifie, et de mémoriser les performances observées des ressources.

## Future Types

Types futurs envisagés :

- `ComputeNode`
- `ModelProfile`
- `ComputeRequest`
- `ComputeAllocation`
- `ComputeBudget`
- `ComputePolicy`
- `ComputeTelemetry`

Ces types ne sont pas encore implémentés. Ce document cadre la future brique sans créer de crate ni d'API.

## V0 Routing Rules

Règles V0 envisagées :

- sensitive data -> local preferred;
- low budget -> local preferred;
- complex reasoning -> strong model allowed;
- high-risk task -> strong model plus human-governed path;
- unavailable resource -> fallback;
- long task -> local preprocessing before cloud synthesis.

Ces règles servent à guider l'allocation compute. Elles ne donnent aucun droit d'exécution.

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

## Governance Boundary

Le Compute Reservoir peut recommander une ressource.

Il ne peut pas approuver une action.
Il ne peut pas exécuter une action.
Il ne peut pas ignorer une politique.
Il ne peut pas exposer un secret au LLM.

Tout effet externe futur devra rester contrôlé par :

```text
ProposedAction -> ToolRegistry lookup -> DecisionGate -> Human approval if needed -> Controlled execution -> Audit -> Graph update
```
