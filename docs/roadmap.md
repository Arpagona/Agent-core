# Roadmap

La roadmap reflète à la fois l'avancement réel et l'ordre architectural cible. Certaines briques ont été prototypées hors ordre pour explorer rapidement le système. Elles doivent rester alpha/expérimentales tant que les couches de gouvernance ne sont pas stabilisées, mais cela ne doit pas bloquer les incréments read-only utiles.

Référence canonique : lire `PROJECT_OBJECTIVES.md` pour la vision, `PROJECT_STATUS.md` pour l'état opérationnel courant, `docs/operating-doctrine.md` pour la doctrine de travail et `docs/development-acceleration.md` pour la direction actuelle d'accélération.

## Recentrage architectural et accélération contrôlée

Priorité immédiate : accélération contrôlée vers une alpha fonctionnelle.

Le projet doit avancer vers une ergonomie Hermes-like tout en conservant l'architecture ARPAGONA : Rust-first, local-first, graph-native, compute-aware, auditable et gouvernée.

Le frein principal reste le même : aucune exécution réelle d'outil, autonomie scheduler, navigateur, MCP, shell, envoi email ou accès secrets par LLM ne doit être ajouté avant stabilisation du chemin gouverné.

Mais les surfaces read-only de supervision sont désormais prioritaires. En particulier, la CLI est le premier Mission Control local.

Ordre de développement actuel :

1. conserver les Core Domain Types propres ;
2. maintenir le Decision Gate séparé ;
3. maintenir le Compute Reservoir minimal ;
4. maintenir le Tool Registry déclaratif ;
5. stabiliser Graph Memory + SurrealDB autant que nécessaire pour le readback ;
6. stabiliser Audit autant que nécessaire pour le readback ;
7. développer la CLI de supervision read-only ;
8. reprendre ensuite Runtime / API / Orchestrator par petits incréments gouvernés ;
9. différer Mission Control Web tant que la CLI locale n'a pas prouvé les bons patterns.

Consignes de recadrage :

- controlled fast iteration ;
- CLI supervision first ;
- Rust-first implementation ;
- LOCO/Ollama delegation for local first-pass analysis ;
- protect governance boundaries ;
- avoid endless test-only stabilization when a useful read-only supervision increment is available.

Le Tool Registry doit exister avant toute exécution réelle d'outil. L'API, la CLI et le Runtime doivent rester alpha tant que Decision Gate, Compute Reservoir, Tool Registry, Graph Memory et Audit ne sont pas stabilisés. Alpha ne signifie pas gelé : les chemins read-only, observables et réversibles peuvent progresser.

## Brique 1 — Fondation core

Objectif : poser une base saine et compilable.

Inclus :

- structure monorepo ;
- documentation fondatrice ;
- crate Rust `core` ;
- types fondamentaux sérialisables ;
- tests unitaires simples.

Exclus : API stable, UI, exécution d'outils, scheduler actif, autonomie, secrets opérationnels.

État : fondation stable, mais `crates/core` doit rester un vocabulaire domaine et ne pas devenir un fourre-tout.

## Brique 2 — Decision Gate séparé

État actuel : implémentation alpha minimale extraite dans `crates/decision-gate`.

Objectif suivant : conserver la frontière propre entre `crates/core` et `crates/decision-gate`, tout en rendant les décisions inspectables par Audit et CLI.

État actuel détaillé :

- module pur Rust `decision_gate` sans API, LLM, I/O, shell ni exécution d'outils ;
- fonction `evaluate_proposed_action(action, policies, granted_permissions) -> Decision` ;
- règles alpha : permissions manquantes bloquées, risques `Informational` / `Low` approuvés sauf politique d'escalade, `Medium` en validation humaine, `High` / `Critical` en validation humaine ou blocage selon policy, `Custom` non explicitement autorisé en validation humaine ;
- helper `audit_event_for_decision(action, decision) -> AuditEvent` pour matérialiser le flux `ProposedAction -> DecisionGate -> Decision -> AuditEvent` ;
- documentation dédiée : `docs/decision-gate.md`.

Contraintes :

- ne pas casser l'API alpha ;
- ne pas casser la CLI alpha ;
- ne pas introduire d'exécution ;
- conserver des tests verts ;
- si l'extraction n'est pas triviale, la faire dans une mission dédiée.

## Brique 3 — Compute Reservoir minimal

État : alpha minimal dans `crates/compute-reservoir`.

Objectif : fournir une brique distincte chargée de choisir comment penser ou traiter une tâche.

Implémentation actuelle : types sérialisables et fonction pure `allocate_compute(request, nodes, policy) -> ComputeAllocation`. La brique couvre l'inventaire de ressources, les capacités, la sensibilité des données, le budget, la latence/coût attendus, la justification, le fallback et une forme minimale de télémétrie future sans persistence.

Le Compute Reservoir doit gérer à terme :

- inventaire des ressources cognitives et computationnelles ;
- profils de modèles ;
- routage local/cloud/workers/GPU/CPU ;
- contraintes de confidentialité ;
- estimation coût/latence ;
- matching capability/tâche ;
- fallback ;
- mémoire de performance.

Il ne fait aucun appel modèle, aucune exécution, aucun réseau, aucune persistence et ne décide pas si une action peut être exécutée. Cette responsabilité appartient au Decision Gate.

Document de cadrage : `docs/compute-reservoir.md`.

Direction actuelle : relier progressivement cette brique à la délégation locale/cloud observée dans la doctrine LOCO/Ollama, sans encore ajouter d'exécution ni de provider runtime supplémentaire.

## Brique 4 — Tool Registry

État : alpha minimal dans `crates/tool-registry`.

Objectif : décrire déclarativement les outils disponibles sans donner d'accès libre aux agents.

Doit inclure :

- description déclarative des outils ;
- schémas d'entrée/sortie ;
- permissions requises ;
- niveau de risque ;
- statut activé/désactivé ;
- simulation en V0 avant exécution réelle.

Implémentation actuelle : catalogue déclaratif en mémoire, types sérialisables pour déclarations d'outils, capacités, schémas, permissions, risques, statuts, lookup, désactivation et liste des outils activés.

Document de cadrage : `docs/tool-registry.md`.

Contrainte non négociable : aucune exécution réelle d'outil avant Tool Registry + Decision Gate + Audit stabilisés.

## Brique 5 — Graph Memory + SurrealDB stabilisé

État : abstraction domaine pure Rust dans `crates/core` et adapter SurrealDB expérimental dans `crates/graph-memory`.

- Port synchrone canonique `GraphMemoryStore` sans dépendance DB.
- Implémentation `InMemoryGraphMemoryStore` pour tests et développement du domaine.
- Stockage minimal de `Source`, `Fact`, `Episode`, `Observation`, `AuditEvent` et relations `GraphRelation` / `RelationType`.
- Requêtes de base, dont récupération des faits actifs par entité.
- Adapter `SurrealGraphMemoryStore` séparé du domaine core.
- Port async d'adapter nommé `AsyncGraphMemoryStore`, distinct du contrat domaine.
- Migration `0001_graph_memory.surql` et tests d'adapter avec SurrealDB en mémoire.
- Readback audit alpha pour workspace, task, proposed action, decision et summaries décisionnels.

Travail restant :

- stabiliser les conventions SurrealDB utiles au readback ;
- stabiliser les relations graphe ;
- garantir que les décisions importantes sont traçables dans le graphe ;
- éviter que Graph Memory devienne une couche d'exécution ou d'autorisation.

## Brique 6 — Audit System stabilisé

État : alpha avec readback décisionnel utilisable.

Objectif : garantir une trace causale claire pour :

- action proposée ;
- contexte utilisé ;
- décision prise ;
- approbation humaine ;
- résultat ;
- erreur ;
- invalidation ou changement de politique.

L'Audit doit être stabilisé avant toute exécution réelle.

Direction actuelle : rendre l'audit inspectable via CLI avant d'élargir l'API ou Mission Control Web.

## Brique 7 — CLI supervision locale

État : alpha, première surface locale de supervision.

Objectif : faire de la CLI le premier Mission Control local.

Commandes existantes ou en cours :

- `arpagona audit decision-summary <decision-id>` ;
- `arpagona audit decision-summary <decision-id> --json`.

Prochaines commandes souhaitées :

- `arpagona audit task-summary <task-id>` ;
- `arpagona audit workspace-summary <workspace-id>` ;
- commandes de status/readback permettant de comprendre tâches, actions proposées, décisions, risques, politiques et événements d'audit.

Contraintes :

- read-only par défaut ;
- pas d'approbation implicite ;
- pas d'exécution ;
- pas de contournement du Decision Gate ;
- pas de privilège supérieur aux couches métier ;
- ne pas transformer la CLI en gouvernance cachée.

## Brique 8 — Neutral Orchestrator

État : pas encore implémenté comme brique stable.

Objectif : coordonner objectifs, tâches, rappel mémoire, allocation Compute Reservoir, propositions d'action, Decision Gate et Audit.

Contrainte : l'orchestrateur ne doit jamais devenir un agent autonome non gouverné.

Direction actuelle : différer l'orchestrateur stable jusqu'à ce que la CLI permette d'inspecter clairement les décisions et traces produites par les couches existantes.

## Brique 9 — API Server Axum

État : alpha minimale dans `apps/api-server`.

- Serveur Axum lançable avec `cargo run -p arpagona-api-server`.
- Stockage in-memory des `Task`, `ProposedAction`, `Decision` et `AuditEvent`.
- Endpoints REST initiaux : `health`, `tasks`, `proposed-actions`, `agent/propose`, `decision-gate/evaluate`, `decisions`, `audit`.
- Consultation du flux `Task -> ProposedAction -> DecisionGate -> Decision -> AuditEvent` sans shell, scheduler, outil exécutable ni SurrealDB obligatoire.
- Provider LLM expérimental limité à la proposition de `ProposedAction`.
- Documentation dédiée : `docs/api-server.md`.

Contrainte : l'API ne doit pas prendre de responsabilité de gouvernance métier. Elle expose les couches, elle ne les remplace pas.

Direction actuelle : ne pas élargir l'API avant que le besoin soit démontré par la CLI ou par une limitation claire du filtrage client.

## Brique 10 — Mission Control Web

État : deferred.

Objectif futur : Next.js + TypeScript pour supervision, validation humaine, visibilité de l'audit et exploration graphe.

Ne pas développer maintenant. La CLI doit d'abord démontrer les bons patterns de supervision locale.

## Brique 11 — Scheduler / controlled autonomous loops

État : deferred.

Objectif futur : déclencher des tâches planifiées ou périodiques.

Contrainte : toute boucle autonome devra passer par Graph Memory, Compute Reservoir, Tool Registry, Decision Gate, Audit et approbation humaine si sensible.

## Brique 12 — LLM Provider abstraction stabilisée

État : V0 expérimentale dans `crates/llm` et endpoint `POST /agent/propose`.

- `LlmProvider` abstrait.
- `MockProvider` pour tests et démos sans réseau.
- `OpenAiProvider` utilisant l'API Responses via `OPENAI_API_KEY`.
- `ProposedActionDraft` transformé en `ProposedAction` avec `PendingDecision`.
- Aucune exécution, aucun tool OpenAI, aucun appel automatique au Decision Gate.
- Documentation dédiée : `docs/llm-provider.md`.

Contrainte : le provider LLM propose, mais ne gouverne pas et n'exécute pas.

## Brique 13 — End-to-end demo

État : deferred.

Objectif futur : démontrer le flux complet contrôlé : objectif -> tâche -> rappel mémoire -> allocation compute -> proposition -> décision -> audit -> observation.

Ne pas faire avant stabilisation des couches de gouvernance et avant que les surfaces CLI de supervision puissent expliquer le flux.

## Brique 14 — Security hardening

État : deferred.

Objectif futur : durcir authentification, autorisations, secrets, isolation runtime, logs, rate limiting, accès réseau, stockage et déploiement.

La sécurité ne doit pas être ajoutée comme patch tardif pour justifier une autonomie précoce : elle doit consolider une architecture déjà gouvernée.

## Briques expérimentales existantes hors ordre

### Cognitive Runtime / Reservoir Echo

État : primitives domaine ajoutées dans `crates/core/src/cognitive.rs`.

Inclus :

- `CognitiveLayer` : Input, WorkingMemory, ReservoirEcho, GraphMemory, AgentProposal, DecisionGate, HumanBoundary, Audit, Reflection, etc. ;
- `AgentLoopPhase` : ordre alpha-safe d'une boucle agentique ;
- `CognitivePulse` : signal court terme ;
- `ReservoirTrace` : trace d'écho avec activation et décroissance ;
- `ReservoirState` : réservoir court terme borné et déterministe ;
- `CognitiveCycleInput` ;
- `CognitiveCyclePlan::alpha_safe_default()`.

Contraintes :

- pure domain, pas d'I/O ;
- pas d'appel LLM ;
- pas de scheduler ;
- pas d'exécution ;
- Reservoir Echo n'est pas une mémoire persistante ;
- Reservoir Echo n'est pas le Compute Reservoir ;
- Graph Memory reste responsable de la mémoire durable.

Documentation dédiée : `docs/cognitive-runtime.md`.

### Runtime V0

État : crate expérimental `crates/runtime` ajouté.

Flux V0 :

```text
CognitiveCycleInput
-> CognitivePulse
-> ReservoirState
-> LlmProvider
-> ProposedAction
```

Contraintes :

- la boucle s'arrête à `ProposedAction` ;
- `DecisionGate` n'est pas appelé automatiquement ;
- aucun `Decision` ni `AuditEvent` n'est créé par ce crate ;
- pas d'exécution d'outils ;
- pas de scheduler ;
- pas d'I/O direct ;
- provider OpenAI possible via abstraction, mais tests sans réseau.

Documentation dédiée : `docs/runtime.md`.

### Terminal Interface

État : interface terminal alpha dans `crates/cli`, désormais considérée comme première surface locale de supervision.

Inclus :

- `arpagona chat` ;
- provider `mock` par défaut pour tests sans réseau ;
- provider `openai` optionnel ;
- commandes internes `/help`, `/quit`, `/tasks`, `/actions`, `/evaluate`, `/audit`, `/provider` ;
- vérification `/health` au démarrage ;
- affichage lisible des `ProposedAction`, `Decision` et `AuditEvent` ;
- readback audit décisionnel via `arpagona audit decision-summary <decision-id>` et `--json`.

Limites alpha / contraintes :

- pas de ratatui/crossterm ;
- pas de shell ;
- pas de scheduler ;
- pas d'exécution d'outils ;
- le Decision Gate reste déclenché explicitement par `/evaluate` ;
- le readback CLI ne vaut pas approbation, autorisation, orchestration ou exécution.

Documentation dédiée : `docs/terminal-interface.md` et `docs/causal-trace.md`.

## Workers d'ingestion

État : placeholder/deferred.

Objectif futur : ingestion documentaire, extraction de sources, observations et faits, raccordement contrôlé à Graph Memory.

Ne pas développer avant stabilisation de Graph Memory, Audit et gouvernance, sauf cadrage documentaire ou expérimentation explicitement isolée.
