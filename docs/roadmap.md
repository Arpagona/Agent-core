# Roadmap

La roadmap reflète à la fois l'avancement réel et l'ordre architectural cible. Certaines briques ont été prototypées hors ordre pour explorer rapidement le système. Elles doivent rester alpha/expérimentales tant que les couches de gouvernance ne sont pas stabilisées, mais cela ne doit pas bloquer les incréments read-only utiles.

Référence canonique : lire `PROJECT_OBJECTIVES.md` pour la vision, `PROJECT_STATUS.md` pour l'état opérationnel courant, `AGENT_FOCUS_LOOP.md` pour la queue active, `FOCUS_LOOP_NEXT.md` pour le prochain geste opérationnel, `docs/operating-doctrine.md` pour la doctrine de travail, `docs/development-acceleration.md` pour la direction actuelle d'accélération et `docs/failure-to-insight.md` pour la transformation des échecs en apprentissages durables non autorisants.

## Snapshot opérationnel — 2026-05-29

La roadmap active est maintenant **Phase 3 — Neutral Orchestrator / governed cognitive runtime**.

État vérifié :

- Phase 1 et Phase 2 sont considérées livrées sauf régression.
- Le backlog de validation quotidienne est vide : aucun candidat ouvert dans `DAILY_VALIDATION_BACKLOG.md`.
- Le `main` récent est vert d'après les PRs mergées et les handoffs, mais la branche locale courante peut être une branche de rebase/stack en cours ; vérifier `git status` avant tout merge.
- Neutral Orchestrator n'est plus simplement “deferred” : une couche alpha existe et coordonne contexte, route compute, génération de proposition, Decision Gate, audit/readback et CycleTrace, sans autorisation implicite.
- Les adapters de contexte mémoire sont livrés en alpha : Graph Memory, Holographic Memory, Reservoir Echo, Compressed Cognitive Attention et Tool Runtime alimentent le contexte comme signaux advisory.
- Le chemin LLM a avancé au-delà du proposal-only strict : les intents de tool-call directs sont acceptables uniquement dans l'enveloppe gouvernée `LLM ToolCall Intent -> Decision Gate -> Tool Runtime/MCP -> Observation -> Audit -> Reflection`.
- Travail ouvert au moment de ce snapshot : PRs #197/#198 vertes, #199 mergeable mais CI rouge, #200/#202 vertes et mergeables. Le prochain travail sûr est de résoudre/merger cette pile avant d'ouvrir un nouveau chantier.

La priorité immédiate n'est donc plus de définir Phase 3, mais de **consolider la pile P3-15/P3-16/P3-17**, corriger le rouge CI de #199, puis seulement décider entre P3-18+ et l'entrée en Phase 4.

## Recentrage architectural et accélération contrôlée

Priorité immédiate : accélération contrôlée vers une alpha fonctionnelle.

Le projet doit avancer vers une ergonomie Hermes-like tout en conservant l'architecture ARPAGONA : Rust-first, local-first, graph-native, compute-aware, auditable et gouvernée.

Le frein principal reste le même : aucune capacité dangereuse non gouvernée — shell libre, navigateur, écriture/fichier non bornée, envoi email, autonomie scheduler ou accès secrets par LLM — ne doit être ajoutée. MCP lui-même est désormais une surface alpha gouvernée ; ce qui reste interdit, ce sont les capacités MCP dangereuses ou l'usage de MCP comme contournement du Decision Gate.

Mais les surfaces read-only de supervision sont désormais prioritaires. En particulier, la CLI est le premier Mission Control local.

Ordre de développement actuel :

1. conserver les Core Domain Types propres ;
2. maintenir le Decision Gate séparé ;
3. maintenir le Compute Reservoir minimal ;
4. maintenir le Tool Registry déclaratif ;
5. stabiliser Graph Memory + SurrealDB autant que nécessaire pour le readback ;
6. stabiliser Audit autant que nécessaire pour le readback ;
7. intégrer Failure-to-Insight par doctrine documentaire, puis vocabulaire borné et conventions d'audit ;
8. développer la CLI de supervision read-only ;
9. poursuivre Runtime / API / Orchestrator par petits incréments gouvernés ;
10. différer Mission Control Web tant que la CLI locale n'a pas prouvé les bons patterns.

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


## Brique 6bis — Failure-to-Insight

État : doctrine documentaire canonique, vocabulaire domaine minimal et premiers chemins alpha de readback/persistence gouvernée. Les PRs Phase 3 récentes ont ajouté ou proposé des ponts CycleTrace -> FailureInsightCandidate et de l'analyse d'efficacité compute, toujours non autorisants.

Objectif : transformer les échecs, blocages, mauvaises propositions, mauvais routages, contextes manquants, policy gaps et corrections humaines en apprentissages durables. Ces apprentissages peuvent améliorer documentation, tests, conventions d'audit, mémoire, politiques, routage Compute Reservoir et futures décisions, mais ils ne valent jamais autorisation, approbation, exécution ni gouvernance autonome.

Ordre recommandé actualisé :

1. doctrine documentaire — livré ;
2. vocabulaire domaine `FailureInsight` — livré en alpha ;
3. conventions Audit et readback non autorisant — livré progressivement ;
4. readback CLI et CycleTrace operator-facing — en cours Phase 3 ;
5. tests de régression — en cours et à maintenir ;
6. intégration Graph Memory / persistence approuvée — alpha ;
7. influence future sur Compute Reservoir et Decision Gate — seulement comme signal/contexte, jamais comme autorisation.

Contraintes alpha : ne pas implémenter d'auto-amélioration autonome, de self-modification, de réécriture automatique de policy, de mutation mémoire non revue, de scheduler ou d'exécution réelle. Failure-to-Insight est une couche d'apprentissage et d'observabilité, pas une couche d'exécution ni une autorisation implicite.

## Brique 7 — CLI supervision locale

État : alpha, première surface locale de supervision.

Objectif : faire de la CLI le premier Mission Control local.

Commandes existantes ou en cours :

- `arpagona audit decision-summary <decision-id>` ;
- `arpagona audit decision-summary <decision-id> --json` ;
- `arpagona status --json` ;
- `arpagona memory demo failure-insight --json` ;
- `arpagona tool list|inspect|demo ... --json` ;
- `arpagona cognitive run --llm --provider ...` ;
- `arpagona orchestrator run --proposal-generator simulated|llm ...` avec readback non autorisant.

Prochaines commandes souhaitées / à consolider :

- `arpagona audit task-summary <task-id>` ;
- `arpagona audit workspace-summary <workspace-id>` ;
- commandes de status/readback permettant de comprendre tâches, actions proposées, décisions, risques, politiques, événements d'audit, CycleTrace et FailureInsightCandidates ;
- explication operator-facing de coût/qualité/efficacité compute quand la pile P3-15/P3-17 sera mergée.

Contraintes :

- read-only par défaut ;
- pas d'approbation implicite ;
- pas d'exécution ;
- pas de contournement du Decision Gate ;
- pas de privilège supérieur aux couches métier ;
- ne pas transformer la CLI en gouvernance cachée.

## Brique 8 — Neutral Orchestrator

État : brique alpha active dans `crates/neutral-orchestrator`, pas encore stable produit.

Objectif : coordonner objectifs, tâches, rappel mémoire, allocation Compute Reservoir, propositions d'action, Decision Gate et Audit.

Contrainte : l'orchestrateur ne doit jamais devenir un agent autonome non gouverné.

Implémentation actuelle : contrats de cycle, contexte advisory, adapters mémoire/contexte, route compute, génération de proposition simulée ou LLM, passage par Decision Gate, audit/readback, demo CLI et CycleTrace. Les travaux récents P3-13/P3-14 sont mergés ; P3-15/P3-16/P3-17 sont en pile PR et doivent être terminés avant d'élargir la surface.

Direction actuelle : stabiliser l'inspectabilité opérateur du CycleTrace, des signaux Failure-to-Insight et de l'efficacité compute. Ne pas transformer l'orchestrateur en scheduler, approbateur, exécuteur ou couche d'autonomie cachée.

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

État : V0 expérimentale dans `crates/llm`, endpoint `POST /agent/propose`, synthèse cognitive locale/Ollama et chemins de tool-call gouvernés via runtime/orchestrator.

- `LlmProvider` abstrait.
- `MockProvider` pour tests et démos sans réseau.
- `OpenAiProvider` utilisant l'API Responses via `OPENAI_API_KEY`.
- `OllamaProvider` pour usage local quand disponible, sans pull automatique dans les protocoles de validation.
- `ProposedActionDraft` transformé en `ProposedAction` avec `PendingDecision`.
- Aucune exécution par provider, aucun tool OpenAI direct, aucun contournement du Decision Gate.
- Refus/gouvernance explicite pour objectifs secrets `.env`, credentials, shell libre et commandes système non bornées.
- Documentation dédiée : `docs/llm-provider.md`.

Contrainte : le provider LLM propose ou synthétise, mais ne gouverne pas et n'exécute pas. Les tool-call intents directs ne sont valides que s'ils passent par Decision Gate puis Tool Runtime/MCP borné, avec observation et audit.

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

### Holographic Memory Kernel

**Crate:** `crates/holographic-memory` · **Statut:** Alpha V0

**Objectif :** Implémenter un noyau de mémoire associative symbolique :
- traces distribuées avec signatures déterministes ;
- récupération par résonance (Jaccard entre bits) ;
- contexte reconstruit lié aux sources (décisions, mémoires, tours) ;
- isolation par `project_id` ;
- store in-memory pour V0.

**Phrase canonique :**
> Holographic Memory reactivates paths to truth. It does not replace truth.

**Contraintes V0 :**
- Pas de LLM obligatoire, pas de base vectorielle externe, pas d'exécution.
- Persistance SQLite locale existe en alpha et doit rester isolée/gouvernée.
- Pas d'autorisation — le contexte reconstruit est une preuve, pas une approbation.
- Ne remplace pas Graph Memory (source de vérité) ni le Decision Gate.
- Code déterministe et testable.

**Prochaines étapes :** intégration avec conversation-memory, embeddings locaux optionnels, graphe mémoire récursif, persistance, consolidation, gouvernance des écritures par Decision Gate.

Documentation dédiée : `docs/holographic-memory.md`.

## Workers d'ingestion

État : placeholder/deferred.

Objectif futur : ingestion documentaire, extraction de sources, observations et faits, raccordement contrôlé à Graph Memory.

Ne pas développer avant stabilisation de Graph Memory, Audit et gouvernance, sauf cadrage documentaire ou expérimentation explicitement isolée.
