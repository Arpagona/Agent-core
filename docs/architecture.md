# Architecture cible

ARPAGONA Agent Core est organisé autour d'un runtime gouverné par graphe. Les agents ne sont pas des exécutants directs : ils produisent des intentions structurées qui sont évaluées, tracées et éventuellement exécutées par des composants contrôlés.

## Flux central

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Ce flux est non négociable : toute action sensible passe par une décision explicite et traçable.

## Architectural Re-Centering

Le dépôt contient déjà plusieurs briques alpha ou expérimentales. Elles doivent maintenant être recadrées autour des responsabilités suivantes.

`crates/core` doit rester le vocabulaire domaine du système : types, identifiants, événements, risques, permissions, actions proposées, décisions, sources, faits, épisodes, audit, graph primitives et contrats purs. Il ne doit pas devenir un fourre-tout pour la logique runtime, les adapters, les providers, l'API, la CLI ou la gouvernance métier avancée.

Le Decision Gate doit être extrait dans un crate dédié lorsque l'extraction peut être faite proprement sans casser les tests. Sa responsabilité est de prendre une `ProposedAction`, des politiques, des permissions et du contexte applicable, puis de produire une `Decision` traçable. Il ne doit pas appeler de LLM, exécuter d'outil, accéder au shell ou faire d'I/O direct.

Le Compute Reservoir est une brique distincte du Reservoir Echo :

- Reservoir Echo : continuité cognitive court terme, traces volatiles, activation/décroissance, influence limitée sur les prochains cycles ;
- Compute Reservoir : sélection de ressources, routage modèle/worker, arbitrage coût/latence/confidentialité/capability/fallback, mémoire de performance.

Le Compute Reservoir choisit comment penser ou traiter une tâche. Il ne décide pas si une action peut être exécutée. Cette responsabilité reste celle du Decision Gate.

Le Tool Registry doit précéder toute exécution réelle. Il décrit déclarativement les outils, leurs schémas, permissions, risques et états activé/désactivé. Un agent ne doit jamais obtenir d'accès direct à un outil.

L'API, la CLI et le Runtime ne doivent pas prendre de responsabilité de gouvernance métier. Ils exposent ou orchestrent les couches, mais ne remplacent ni Decision Gate, ni Tool Registry, ni Audit, ni Graph Memory.

Toute future exécution doit être contrôlée par :

```text
ProposedAction -> ToolRegistry lookup -> DecisionGate -> Human approval if needed -> Controlled execution -> Audit -> Graph update
```

Aucune exécution d'outil, autonomie scheduler, browser automation, MCP integration, shell access, email sending ou secrets access by LLM ne doit être ajoutée avant stabilisation de Decision Gate + Tool Registry + Audit.

## Runtime Core

Le Runtime Core définit les concepts fondamentaux : agents, workspaces, tâches, objectifs, actions proposées, décisions, politiques, permissions, risques, sources, épisodes, faits et événements d'audit.

Dans cette première brique, `crates/core` reste un crate de types purs. Il ne dépend pas d'Axum, de SurrealDB, d'un provider LLM ou d'un système d'exécution.

## Mission Control

Mission Control sera l'interface web de supervision. Elle permettra de consulter les tâches, inspecter les actions proposées, approuver les actions sensibles, explorer la mémoire graphe et suivre les événements d'audit.

Mission Control est deferred tant que les couches de gouvernance ne sont pas stabilisées.

## Graph Memory

Graph Memory sera la couche de mémoire structurée. Elle stockera les faits, sources, épisodes, observations, relations et décisions importantes. Le contrat domaine vit dans `crates/core` sous forme d'un port Rust pur (`GraphMemoryStore`) et d'une implémentation en mémoire (`InMemoryGraphMemoryStore`). Le crate `crates/graph-memory` est l'adapter persistant SurrealDB : il expose `SurrealGraphMemoryStore` et un port async expérimental nommé `AsyncGraphMemoryStore`, distinct du contrat domaine. Aucune couche Graph Memory ne donne de pouvoir d'exécution aux agents.

Graph Memory doit aussi devenir la base de traçabilité des décisions importantes, en lien avec Audit, sans devenir un orchestrateur ou un moteur d'exécution.

## Decision Gate

Le Decision Gate évaluera chaque `ProposedAction` selon :

- le type d'action ;
- le niveau de risque ;
- les permissions requises ;
- les politiques actives ;
- le contexte du workspace ;
- le besoin éventuel d'approbation humaine.

Il produira une `Decision` : autorisation, blocage ou demande de validation humaine.

État actuel : alpha dans `crates/core`.

État cible : crate dédié `crates/decision-gate`, sans I/O direct et sans exécution.

## Compute Reservoir

Le Compute Reservoir est une future brique centrale. Il choisira la ressource cognitive ou computationnelle adaptée à une tâche : modèle local, modèle cloud, worker local, GPU, CPU, tâche différée ou fallback.

Il arbitrera selon : capability, confidentialité, coût, latence, budget, disponibilité, performance observée et stratégie de fallback.

Il ne remplace pas le Decision Gate, ne remplace pas Graph Memory et ne donne pas de droit d'exécution.

Document de cadrage : `docs/compute-reservoir.md`.

## Reservoir Echo

Reservoir Echo appartient aux primitives cognitives court terme. Il sert à maintenir une continuité volatile entre cycles : traces, activation, décroissance et influence limitée sur les prochains tours.

Il ne doit pas être confondu avec une mémoire persistante, un routeur de modèles, une couche de budget ou le Compute Reservoir.

## Tool Registry

Le Tool Registry décrit les capacités disponibles sans donner un accès libre aux agents. Son état actuel est alpha minimal dans `crates/tool-registry` : catalogue déclaratif en mémoire, schémas, permissions, risques, statuts, lookup et désactivation de déclarations uniquement. Un agent peut demander une action outillée, mais l'exécution effective reste contrôlée par le Decision Gate et les couches runtime.

Le Tool Registry doit exister et être stabilisé avant toute exécution réelle d'outil.

## Audit Store

L'Audit Store conservera les événements importants : action proposée, décision prise, approbation humaine, exécution, échec, révocation, changement de politique, etc.

Audit doit être stabilisé avant toute exécution réelle afin que chaque effet externe soit relié à une proposition, une décision et un contexte vérifiable.

## Orchestrator

L'Orchestrator coordonnera les agents, les tâches, les objectifs et les propositions d'action. Il restera neutre et adaptable : assistant d'entreprise, agent documentaire, agent de recherche, agent métier, agent de code ou assistant personnel/pro.

Il ne doit jamais contourner le flux : `ProposedAction -> DecisionGate -> Audit`.

## LLM Providers

Les providers LLM vivent hors de `crates/core`, dans `crates/llm`. La V0 expérimentale transforme une demande utilisateur en `ProposedActionDraft`, puis en `ProposedAction` avec le statut `pending_decision`.

Contraintes permanentes : le LLM ne doit jamais exécuter, ne doit pas utiliser d'outils OpenAI, ne doit pas faire de web search et ne doit jamais contourner le Decision Gate. `OPENAI_API_KEY` est lu uniquement par le provider OpenAI et ne doit jamais être loggé.

## API Server

L'API Server expose des objets et flux alpha. Il ne doit pas contenir la gouvernance métier profonde. Il doit appeler les crates responsables et ne jamais devenir un bypass d'exécution.

## CLI

La CLI est une interface de contrôle et de test alpha. Elle ne doit pas obtenir de privilèges directs supérieurs aux couches métier. Toute commande sensible future devra passer par Tool Registry, Decision Gate, approbation humaine si nécessaire et Audit.

## Scheduler

Le Scheduler déclenchera des tâches planifiées ou périodiques, mais ses actions devront suivre le même circuit de décision que les actions proposées par un agent.

Le scheduler est deferred tant que le chemin de gouvernance n'est pas stable.

## Workers d'ingestion

Les workers d'ingestion intégreront documents, données et sources externes dans la mémoire graphe. Ils devront produire des sources, observations et faits traçables.

Ils ne doivent pas contourner Graph Memory, Audit ou les politiques de confidentialité.
