# Cognitive Continuity Layer

## Intention

La Cognitive Continuity Layer est une couche conceptuelle et technique destinée à donner à ARPAGONA Agent Core une continuité d'état entre les cycles agentiques.

Elle ne doit pas être comprise comme une conscience artificielle, ni comme un nouveau LLM, ni comme un simple scheduler amélioré. Son rôle est plus précis : maintenir un état latent dynamique qui conserve une rémanence des objectifs, tensions, idées en incubation, priorités émergentes et orientations stratégiques du système.

L'objectif est d'éviter une architecture strictement discontinue du type :

```text
cron -> réveil agent -> lecture mémoire -> action -> arrêt
```

et de tendre vers une boucle plus organique :

```text
état latent persistant -> perception -> résonance -> intention -> action -> mise à jour de l'état latent
```

Cette couche doit permettre au système de ne pas seulement se souvenir de ce qui a été dit ou fait, mais de conserver une forme d'orientation cognitive évolutive.

## Problème adressé

Les architectures agentiques classiques sont souvent discontinues. À chaque cycle, l'agent relit un contexte, produit une réponse ou une action, écrit éventuellement un résumé, puis s'arrête. Le cycle suivant repart d'une reconstruction textuelle ou documentaire.

Cette approche pose plusieurs problèmes :

- perte de continuité entre les cycles ;
- dépendance excessive au contexte injecté dans le LLM ;
- difficulté à maintenir des priorités implicites ;
- faible capacité à détecter les tendances lentes ;
- risque d'oublier les tensions non résolues ;
- fonctionnement trop mécanique du scheduler ;
- absence d'état interne durable autre que la mémoire explicite.

La Cognitive Continuity Layer vise à ajouter un état intermédiaire entre la mémoire structurée et l'orchestration active.

## Différence avec les autres mémoires

La Cognitive Continuity Layer ne remplace pas la Graph Memory ni la mémoire vectorielle.

```text
Graph Memory      = structure stable des faits, décisions, relations, contraintes.
Vector Memory     = recherche sémantique dans les textes et documents.
Audit Store       = trace factuelle des événements et décisions.
Cognitive State   = état latent dynamique, rémanent et évolutif.
LLM               = langage, raisonnement explicite, planification et formulation.
Policy Engine     = règles de décision et garde-fous d'exécution.
```

La Graph Memory répond principalement à la question :

```text
Qu'est-ce que le système sait et quelles relations sont établies ?
```

La Cognitive Continuity Layer répond plutôt à :

```text
Dans quel état d'orientation le système se trouve-t-il actuellement ?
```

Exemple :

```text
Mémoire classique :
"Le projet a évoqué la graph memory, le reservoir computing et le heartbeat."

Cognitive Continuity Layer :
"Le projet converge vers une architecture d'agent autonome avec continuité cognitive, mémoire structurée, gouvernance d'action et sobriété locale-first."
```

## Inspiration : reservoir computing

L'inspiration vient du reservoir computing, mais l'implémentation initiale ne doit pas nécessairement être un Echo State Network complet.

L'idée utile à retenir est celle d'un système dynamique récurrent :

```text
état(t+1) = dynamique(état(t), entrée(t))
```

Un tel système ne repart pas de zéro à chaque cycle. Il conserve une rémanence des signaux précédents, avec une inertie, une décroissance et une sensibilité aux répétitions.

Pour ARPAGONA Agent Core, cette inspiration doit être traduite en une couche pragmatique :

- état latent persistant ;
- oubli contrôlé ;
- rythmes temporels différenciés ;
- interprétation périodique de l'état ;
- influence sur le heartbeat et l'orchestration ;
- consolidation sélective vers la Graph Memory.

## État latent persistant

Le coeur de cette couche est un état latent persistant, stocké et mis à jour au fil des événements.

Cet état peut contenir plusieurs familles de dimensions.

### Focus actifs

```text
focus_architecture
focus_memory
focus_research
focus_code
focus_business
focus_local_models
focus_security
focus_productization
```

Ces dimensions indiquent les domaines cognitifs actuellement activés.

### Tensions

```text
complexity_risk
need_for_clarification
implementation_readiness
strategic_value
risk_of_drift
need_for_human_decision
need_for_consolidation
```

Ces dimensions indiquent les contradictions, incertitudes ou pressions qui doivent influencer les cycles suivants.

### Dynamique

```text
momentum
stagnation
novelty
repetition
coherence
fragmentation
```

Ces dimensions aident à distinguer exploration, convergence, blocage, dispersion ou maturation.

### Fils actifs

```text
active_threads = [
  "graph_memory",
  "cognitive_continuity",
  "reservoir_computing",
  "agent_core_architecture"
]
```

Les fils actifs représentent des thèmes ou lignes de pensée qui persistent entre les cycles.

## Rythmes temporels

La couche doit gérer plusieurs constantes de temps.

```text
fast_state      = événements immédiats, erreurs, actions récentes.
medium_state    = état des tâches, progression, blocages, momentum.
slow_state      = orientation stratégique, thèmes récurrents, idées incubées.
identity_state  = principes stables du projet, doctrine, contraintes fondatrices.
```

Chaque niveau doit avoir un facteur d'oubli différent.

Le fast_state réagit rapidement et décroît vite.
Le medium_state conserve les dynamiques de travail.
Le slow_state repère les convergences longues.
L'identity_state change rarement et doit être explicitement validé.

Cette séparation évite deux risques opposés :

- oubli trop rapide des tendances importantes ;
- rumination artificielle sur des signaux obsolètes.

## Cognitive Heartbeat

Le scheduler technique reste utile, mais il ne doit pas être le décideur cognitif.

La Cognitive Continuity Layer alimente un Cognitive Heartbeat, dont le rôle est de choisir le type de cycle à exécuter selon l'état latent.

Modes possibles :

```text
SLEEP       = ne rien déclencher, maintenir seulement l'état.
WATCH       = observer les nouveaux événements.
REFLECT     = produire une synthèse courte ou une clarification.
CONSOLIDATE = mettre à jour la mémoire, le graphe ou les documents projet.
ACT         = lancer une action opérationnelle contrôlée.
ESCALATE    = demander une validation ou une décision humaine.
```

Exemples de politiques :

```text
Si novelty élevée + strategic_value élevée + implementation_readiness faible :
  -> REFLECT

Si implementation_readiness élevée + urgency élevée :
  -> ACT

Si fragmentation élevée :
  -> CONSOLIDATE

Si uncertainty élevée + risk élevé :
  -> ESCALATE
```

Le cron peut donc continuer à produire une pulsation technique, mais le heartbeat décide ce que cette pulsation signifie.

## Relation avec la Graph Memory

La Cognitive Continuity Layer ne doit pas stocker toutes les vérités du système. Elle maintient des activations dynamiques.

La Graph Memory reste la source structurée des faits stabilisés, décisions, contraintes, relations et règles.

Flux recommandé :

```text
Événements -> Cognitive State -> interprétation -> signaux -> consolidation sélective -> Graph Memory
```

Exemple :

```text
Le thème "continuité cognitive" revient dans plusieurs cycles.
Le slow_state augmente son activation.
Le state interpreter le marque comme idée en maturation.
Le heartbeat déclenche un cycle REFLECT.
Une synthèse est validée.
La Graph Memory reçoit une décision ou une orientation structurée.
```

La règle importante :

```text
Le Cognitive State observe et résonne.
La Graph Memory stabilise et structure.
Le Decision Gate autorise ou bloque.
L'Orchestrator agit.
```

## Relation avec l'Orchestrator

L'Orchestrator ne doit pas être remplacé par cette couche. Il doit la consulter.

Avant de lancer un cycle agentique, l'Orchestrator peut recevoir un résumé court de l'état cognitif courant :

```json
{
  "current_cognitive_state": {
    "fast": "technical_instability_low",
    "medium": "architecture_clarification_needed",
    "slow": "convergence_on_cognitive_continuity",
    "identity": "local_first_explainable_agentic_ai"
  },
  "active_focus": ["architecture", "memory", "agentic_continuity"],
  "latent_tensions": ["avoid_overengineering", "need_for_mvp"],
  "recommended_mode": "REFLECT"
}
```

Ce contexte doit rester compact. Il ne doit pas devenir une nouvelle injection massive de mémoire.

## V0 recommandée

La V0 ne doit pas commencer par un système neuronal complexe.

La première version doit être contrôlable, lisible et testable :

```text
Cognitive State Vector + decay temporel + règles simples + interprétation périodique
```

Modules possibles :

```text
core/cognitive/
  cognitive_state.rs
  feature_extractor.rs
  decay_engine.rs
  heartbeat.rs
  state_interpreter.rs
  reflection_policy.rs
```

Ou, si la première expérimentation est faite hors core Rust :

```text
workers/cognitive-continuity/
  cognitive_state.py
  feature_extractor.py
  decay_engine.py
  heartbeat.py
  state_interpreter.py
```

La V0 doit permettre :

- d'ingérer des événements standardisés ;
- de mettre à jour un état latent persistant ;
- d'appliquer un oubli contrôlé ;
- de produire un résumé compact de l'état courant ;
- de recommander un mode de heartbeat ;
- de journaliser les mises à jour ;
- de préparer une future consolidation vers la Graph Memory.

## Exemple d'état JSON

```json
{
  "timestamp": "2026-05-17T18:20:00Z",
  "focus": {
    "architecture": 0.92,
    "memory": 0.87,
    "research": 0.78,
    "business": 0.31,
    "code": 0.44
  },
  "tensions": {
    "complexity_risk": 0.81,
    "need_for_clarification": 0.88,
    "implementation_readiness": 0.39,
    "strategic_value": 0.91
  },
  "rhythms": {
    "fast_instability": 0.22,
    "medium_momentum": 0.66,
    "slow_convergence": 0.84
  },
  "active_threads": [
    "graph_memory",
    "cognitive_continuity",
    "reservoir_computing",
    "agent_core_architecture"
  ]
}
```

## V1 possible : vrai reservoir computing

Une fois la V0 stabilisée, un vrai module inspiré du reservoir computing peut être ajouté.

Flux possible :

```text
Input vector -> Echo State Network -> latent vector -> State Interpreter -> Heartbeat Policy
```

Rôle du réservoir :

- capter les répétitions sous différentes formes ;
- détecter les tendances lentes ;
- maintenir une rémanence plus riche que de simples compteurs ;
- faire émerger des signaux de convergence, dispersion, blocage ou maturation ;
- alimenter le heartbeat sans dépendre entièrement du LLM.

Le réservoir ne doit pas décider seul. Il produit un état latent et des signaux. Les décisions restent gouvernées par les politiques, le Decision Gate et, si nécessaire, la validation humaine.

## Garde-fous conceptuels

Cette couche doit éviter plusieurs dérives.

### Ne pas prétendre à la conscience

La Cognitive Continuity Layer n'est pas une conscience artificielle. C'est une continuité d'état logiciel inspirée de systèmes dynamiques.

### Ne pas remplacer la mémoire structurée

Elle ne doit pas devenir une base de vérité. Les faits stabilisés appartiennent à la Graph Memory et à l'Audit Store.

### Ne pas créer de rumination artificielle

Tout état latent doit avoir une décroissance, un archivage ou une consolidation possible.

### Ne pas surcharger le LLM

Le résumé transmis à l'Orchestrator doit rester court, typé et opérationnel.

### Ne pas agir sans gouvernance

Même si le heartbeat recommande ACT, les actions proposées doivent passer par le flux :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

## Critères de réussite

Une première implémentation sera utile si elle permet de répondre à ces questions :

```text
Quels sujets sont actuellement actifs dans le système ?
Quelles tensions persistent depuis plusieurs cycles ?
Quels thèmes sont en train de mûrir ?
Le système est-il en exploration, en consolidation ou prêt pour l'action ?
Faut-il réfléchir, consolider, agir, attendre ou demander une décision humaine ?
```

Elle sera réussie si elle rend ARPAGONA Agent Core moins discontinu, moins dépendant du contexte brut injecté au LLM, et plus capable de maintenir une orientation cohérente entre plusieurs sessions de programmation.

## Résumé court

La Cognitive Continuity Layer est une couche d'état latent persistant inspirée du reservoir computing. Elle maintient une rémanence dynamique des focus, tensions, signaux faibles, idées en incubation et priorités émergentes. Elle transforme le scheduler en heartbeat intelligent et sert d'interface entre événements, mémoire, orchestration et consolidation.

Elle ne remplace ni la Graph Memory, ni le Decision Gate, ni l'Orchestrator. Elle leur donne une continuité dynamique.