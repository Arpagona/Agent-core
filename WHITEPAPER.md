# ARPAGONA Agent Core — Foundational Whitepaper

## Runtime agentique local-first, gouverné par graphe et orienté usage professionnel

---

## 1. Résumé exécutif

ARPAGONA Agent Core est un projet de runtime agentique professionnel destiné à faire fonctionner des agents IA dans un environnement local-first, contrôlé, auditable et extensible.

L'objectif n'est pas de créer un simple chatbot, ni de reproduire un framework existant comme Hermes Agent, OpenClaw ou autre système agentique généraliste. L'objectif est de construire un socle logiciel souverain et professionnel, capable de faire interagir des agents IA avec de la mémoire, des outils, des règles, des ressources de calcul et des humains, sans perdre le contrôle.

La philosophie centrale est simple :

```text
Les agents ne doivent pas agir directement.
Ils doivent proposer des actions.
Le runtime décide ensuite si ces actions sont autorisées, bloquées ou soumises à validation humaine.
```

ARPAGONA Agent Core vise donc à devenir une couche d'orchestration agentique fiable, explicable et adaptable à différents contextes : usage personnel avancé, entreprise locale, automatisation documentaire, assistance métier, recherche, développement logiciel, supervision de projets et systèmes multi-agents.

---

## 2. Problème identifié

Les systèmes agentiques actuels montrent rapidement plusieurs limites lorsqu'on tente de les utiliser sérieusement :

```text
- accumulation excessive de contexte ;
- coût incontrôlé des appels aux grands modèles cloud ;
- mémoire long terme peu fiable ;
- difficulté à invalider les informations obsolètes ;
- manque de traçabilité des décisions ;
- absence de séparation claire entre raisonnement et action ;
- sécurité insuffisante autour des outils ;
- difficulté à combiner modèles locaux, cloud et workers spécialisés ;
- autonomie parfois ajoutée avant la gouvernance.
```

Un agent puissant mais mal encadré peut produire des résultats impressionnants à court terme, mais devenir dangereux, coûteux, opaque ou instable dès qu'il interagit avec des données réelles, des outils réels ou des décisions professionnelles.

Le besoin n'est donc pas seulement de créer un meilleur agent. Le besoin est de créer un runtime agentique gouverné.

---

## 3. Vision

ARPAGONA Agent Core doit devenir une plateforme locale capable de faire fonctionner des agents IA selon une boucle contrôlée :

```text
Objectif utilisateur
-> interprétation
-> rappel du contexte pertinent
-> choix de ressource cognitive
-> proposition d'action
-> évaluation par le Decision Gate
-> validation humaine si nécessaire
-> exécution contrôlée éventuelle
-> observation
-> audit
-> mise à jour de la mémoire
-> réflexion et amélioration contrôlée
```

Le système doit permettre aux agents d'être utiles, proactifs et capables de raisonner, sans leur donner un pouvoir d'exécution direct.

La vision long terme est celle d'un système agentique auto-améliorant, mais selon une règle stricte :

```text
Le système peut observer ses erreurs, proposer des améliorations et ajuster ses stratégies,
mais toute modification structurelle sensible doit rester contrôlée, traçable et validée.
```

Autonomie ne signifie pas absence de contrôle. Autonomie signifie capacité à proposer, apprendre et s'améliorer dans un cadre gouverné.

---

## 4. Principe fondateur

Le principe non négociable d'ARPAGONA Agent Core est :

```text
No agent executes directly.
Agents only propose actions.
Every action passes through the Decision Gate.
Every important decision is recorded in the graph.
Every sensitive action requires human approval.
```

En français :

```text
Aucun agent n'agit directement.
Un agent propose une action.
Le Decision Gate évalue l'action.
L'action est approuvée, bloquée, reroutée ou soumise à validation humaine.
Toute décision importante est tracée dans le graphe.
```

Cette séparation entre proposition, décision et exécution est le cœur de l'architecture.

---

## 5. Objectifs principaux

ARPAGONA Agent Core doit fournir les briques suivantes.

### 5.1 Core Domain

Le Core Domain définit le vocabulaire fondamental du système :

```text
Agent
Human
Workspace
Project
Goal
Task
Action
ProposedAction
Decision
Policy
Tool
Permission
Risk
Fact
Source
Episode
Observation
AuditEvent
Memory
ComputeNode
ModelProfile
ComputeAllocation
```

Ce domaine doit rester stable, typé, modulaire et indépendant des détails d'implémentation.

### 5.2 Graph Memory

La mémoire ne doit pas être une simple conversation sauvegardée ni une base vectorielle brute.

Elle doit être une mémoire structurée en graphe, capable de représenter :

```text
- faits ;
- entités ;
- relations ;
- sources ;
- épisodes ;
- observations ;
- décisions ;
- règles ;
- validité temporelle ;
- confiance ;
- provenance ;
- invalidation.
```

Le système doit pouvoir répondre à des questions comme :

```text
D'où vient cette information ?
Est-elle encore valide ?
Quelle décision l'a utilisée ?
Quelle règle s'appliquait à ce moment ?
Cette information a-t-elle été remplacée ou révoquée ?
```

Les primitives essentielles sont :

```text
remember   -> mémoriser un fait structuré ;
relate     -> relier deux entités ou faits ;
recall     -> rappeler un contexte applicable ;
invalidate -> expirer, remplacer, révoquer ou marquer comme incertain.
```

C'est une inspiration directe des approches modernes de context graph et des architectures de type Rippletide : il ne suffit pas de retrouver une information similaire, il faut retrouver une information applicable, valide et autorisée.

### 5.3 Decision Gate

Le Decision Gate est la couche de contrôle pré-exécution.

Il reçoit une `ProposedAction` et retourne une décision :

```text
Approved
Blocked
NeedsHumanApproval
NeedsMoreContext
```

Il évalue notamment :

```text
- type d'action ;
- outil demandé ;
- permissions requises ;
- niveau de risque ;
- policies applicables ;
- validité du contexte ;
- nécessité de validation humaine.
```

Le Decision Gate ne remplace pas le LLM. Il est au contraire volontairement extérieur au LLM.

Le LLM peut proposer. Le runtime décide.

### 5.4 Tool Registry

Le Tool Registry déclare les outils disponibles.

Un agent ne doit jamais recevoir un accès libre au système. Il doit uniquement pouvoir proposer une action utilisant un outil déclaré.

Chaque outil doit avoir :

```text
- nom ;
- description ;
- schéma d'entrée ;
- schéma de sortie ;
- permissions requises ;
- niveau de risque ;
- statut activé/désactivé ;
- mode dry-run éventuel.
```

La logique est :

```text
Tool Registry = ce qui existe
Decision Gate = ce qui est autorisé
Tool Runtime = ce qui exécute réellement
Audit = ce qui trace
```

### 5.5 Compute Reservoir

Le Compute Reservoir est une brique centrale du projet.

Son rôle est de donner au système une conscience de ses ressources de calcul et de raisonnement.

ARPAGONA Agent Core ne doit pas considérer l'intelligence artificielle comme un seul modèle abstrait, mais comme un réservoir de ressources cognitives hétérogènes :

```text
- grands modèles cloud ;
- modèles locaux ;
- modèles d'embedding ;
- workers Python ;
- outils déterministes ;
- GPU local ;
- CPU local ;
- serveurs distants ;
- traitements différés ;
- fallbacks.
```

Le Compute Reservoir doit répondre à des questions comme :

```text
Quelle ressource doit traiter cette tâche ?
Pourquoi ce modèle plutôt qu'un autre ?
Le cloud est-il autorisé pour ces données ?
Le modèle local suffit-il ?
Quel est le coût estimé ?
Quelle est la latence attendue ?
Existe-t-il un fallback ?
Cette tâche doit-elle être prétraitée localement avant synthèse cloud ?
```

Exemples de règles :

```text
Données sensibles -> local-first.
Tâche longue -> prétraitement local.
Tâche stratégique complexe -> modèle fort autorisé.
Budget faible -> éviter les modèles coûteux.
Risque élevé -> modèle fort + Decision Gate + validation humaine.
```

Cette brique répond à une faiblesse majeure des systèmes actuels : ils appellent souvent les modèles sans stratégie de coût, de confidentialité ou de performance.

### 5.6 Neutral Orchestrator

L'orchestrateur neutre coordonne les objectifs, les tâches, les agents, la mémoire, les ressources et les actions proposées.

Il ne doit pas être spécialisé dès le départ dans un seul cas métier. Il doit pouvoir s'adapter ensuite à plusieurs usages :

```text
- assistant personnel/professionnel ;
- agent documentaire ;
- agent de recherche ;
- agent de développement logiciel ;
- agent administratif ;
- assistant d'entreprise ;
- système local multi-agents.
```

Son rôle :

```text
recevoir un objectif
-> créer une tâche
-> rappeler le contexte
-> demander une allocation compute
-> construire un plan
-> proposer une action
-> soumettre au Decision Gate
-> enregistrer le résultat
-> mettre à jour la mémoire
```

### 5.7 Audit System

L'audit doit être natif.

Chaque événement important doit être traçable :

```text
- demande utilisateur ;
- création de tâche ;
- contexte rappelé ;
- allocation compute ;
- action proposée ;
- décision ;
- validation humaine ;
- exécution éventuelle ;
- observation ;
- fait mémorisé ;
- fait invalidé ;
- erreur ;
- post-mortem.
```

L'objectif est de pouvoir répondre à tout moment :

```text
Qui a demandé quoi ?
Quel agent a proposé l'action ?
Quel contexte a été utilisé ?
Quelle règle a été appliquée ?
Pourquoi l'action a-t-elle été acceptée ou bloquée ?
Quel humain a validé ?
Quel résultat a été observé ?
```

### 5.8 Mission Control

Mission Control est le cockpit web du système.

Il doit permettre de visualiser et piloter :

```text
- agents ;
- workspaces ;
- objectifs ;
- tâches ;
- actions proposées ;
- décisions ;
- validations humaines ;
- graphe mémoire ;
- audit ;
- ressources compute ;
- modèles disponibles ;
- outils ;
- policies ;
- erreurs ;
- boucles autonomes.
```

Mission Control est essentiel parce qu'un système agentique professionnel doit être observable. Un agent qui agit dans une boîte noire n'est pas acceptable en contexte professionnel.

---

## 6. Inspiration Rippletide

ARPAGONA Agent Core s'inspire conceptuellement d'une idée forte : les agents professionnels ont besoin d'une couche de décision entre eux et le monde réel.

La leçon importante n'est pas « faire des agents plus intelligents ». La leçon est :

```text
Rendre les agents contrôlables, traçables et gouvernés avant exécution.
```

Les points à retenir :

```text
- pre-execution enforcement ;
- context graph ;
- règles/policies explicites ;
- contexte applicable plutôt que seulement similaire ;
- audit causal ;
- séparation entre intention et action ;
- invalidation des informations obsolètes.
```

ARPAGONA Agent Core ne doit pas copier Rippletide, mais peut s'inspirer de cette philosophie pour construire son propre runtime local-first, adapté à des usages professionnels, personnels avancés et entreprises locales.

---

## 7. Auto-amélioration contrôlée

Le système doit évoluer vers une capacité d'auto-amélioration, mais sans auto-modification dangereuse.

La boucle cible :

```text
exécution ou tentative
-> observation du résultat
-> audit
-> détection d'erreur ou faiblesse
-> post-mortem
-> proposition d'amélioration
-> validation humaine
-> mise à jour contrôlée
```

Le système peut proposer :

```text
- invalidation d'un fait ;
- modification d'une policy ;
- amélioration d'un outil ;
- changement de modèle pour un type de tâche ;
- ajout d'un test ;
- amélioration d'un prompt système ;
- mise à jour de la mémoire ;
- amélioration d'une règle de routage compute.
```

Mais il ne doit pas modifier librement ses propres règles critiques.

Formule clé :

```text
ARPAGONA Agent Core ne doit pas seulement exécuter des workflows agentiques ;
il doit observer ses propres erreurs, coûts, décisions et résultats afin de proposer
des améliorations contrôlées de sa mémoire, de ses règles, de ses outils et de son routage compute.
```

---

## 8. Technologies cibles

### Backend

```text
Rust
Axum
Tokio
Serde
tracing
OpenAPI / utoipa
```

Rust est choisi pour :

```text
- typage fort ;
- fiabilité ;
- sécurité mémoire ;
- maintenabilité ;
- pertinence pour un runtime local installable.
```

### Frontend

```text
Next.js
React
TypeScript
Tailwind
shadcn/ui
React Flow ou Cytoscape.js pour la visualisation graphe
```

### Base principale

```text
SurrealDB
```

Rôle :

```text
- graphe ;
- documents ;
- relations ;
- mémoire ;
- audit ;
- décisions ;
- policies ;
- compute allocations.
```

### Workers

```text
Python
PyMuPDF / OCR / parsing documentaire / embeddings
```

### Modèles

```text
OpenAI / GPT-5.5
Ollama
modèles locaux
providers futurs : Mistral, Anthropic, OpenRouter, custom HTTP
```

### Déploiement futur

```text
Docker / Podman
systemd
sauvegarde/restauration
secret vault
sandbox d'exécution
```

---

## 9. Ce que la V0 doit prouver

La V0 n'a pas besoin d'être autonome ni prête pour un client. Elle doit prouver que la boucle centrale fonctionne.

Scénario V0 :

```text
1. Un utilisateur crée un objectif.
2. L'orchestrateur crée une tâche.
3. Graph Memory rappelle le contexte applicable.
4. Compute Reservoir choisit une ressource cognitive.
5. Un agent propose une action.
6. Decision Gate évalue l'action.
7. La décision est enregistrée.
8. L'audit trace la chaîne causale.
9. Mission Control ou l'interface alpha rend la chaîne visible.
```

La V0 doit démontrer la structure, pas la puissance brute.

---

## 10. Ce que la V0 ne doit pas faire

Interdits V0 :

```text
- pas de shell libre ;
- pas de suppression fichier ;
- pas de modification système ;
- pas d'envoi email réel ;
- pas d'accès secret par le LLM ;
- pas d'action financière ;
- pas d'auto-modification du runtime ;
- pas d'outil réel sans Tool Registry ;
- pas d'exécution sans Decision Gate ;
- pas d'autonomie scheduler sans garde-fous.
```

L'ambition doit être forte, mais la sécurité doit être native.

---

## 11. Ordre de développement recommandé

L'ordre logique est :

```text
1. Core Domain Types
2. Decision Gate séparé
3. Compute Reservoir minimal
4. Tool Registry
5. Graph Memory + SurrealDB
6. Audit System
7. Neutral Orchestrator
8. API Server Axum
9. Mission Control Web
10. Scheduler et boucles autonomes contrôlées
11. LLM Provider abstraction
12. Démo end-to-end
13. Security hardening
```

Même si certaines briques sont prototypées plus tôt, l'ordre de consolidation doit rester celui-ci.

Priorité actuelle :

```text
Stop feature expansion.
Stabiliser les couches de gouvernance.
Extraire le Decision Gate.
Créer le Compute Reservoir.
Créer le Tool Registry.
Puis seulement continuer API, CLI, runtime et UI.
```

---

## 12. Risques principaux

### 12.1 Surcomplexité

Risque : vouloir tout construire en même temps.

Réponse :

```text
Chaque brique doit être testable seule.
Chaque phase doit produire un résultat visible.
```

### 12.2 Dérive prototype

Risque : ajouter API, CLI, LLM, runtime, UI avant d'avoir stabilisé la gouvernance.

Réponse :

```text
Feature freeze temporaire.
Consolidation de l'architecture.
```

### 12.3 Graphe inutilisable

Risque : créer un graphe trop vague.

Réponse :

```text
ontologie minimale ;
relations typées ;
sources ;
validité ;
invalidation.
```

### 12.4 Sécurité insuffisante

Risque : donner trop vite du pouvoir aux agents.

Réponse :

```text
aucun shell ;
aucun secret ;
Tool Registry ;
Decision Gate ;
Audit ;
validation humaine.
```

### 12.5 Coûts LLM incontrôlés

Risque : envoyer trop de contexte aux modèles cloud.

Réponse :

```text
Compute Reservoir ;
budgets ;
local-first ;
prétraitement local ;
routage intelligent.
```

---

## 13. Vision long terme

À long terme, ARPAGONA Agent Core peut devenir un véritable agentic operating layer :

```text
- assistant personnel/professionnel ;
- système local d'entreprise ;
- mémoire organisationnelle ;
- assistant documentaire ;
- assistant R&D ;
- assistant d'ingénierie ;
- agent de développement logiciel ;
- système de veille ;
- assistant administratif ;
- orchestration multi-agents ;
- cerveau d'entreprise local-first.
```

Mais l'autonomie doit être construite progressivement.

La formule directrice :

```text
Autonomy must be earned, not assumed.
```

En français :

```text
L'autonomie doit être méritée, pas supposée.
```

---

## 14. Phrase de synthèse

```text
ARPAGONA Agent Core est un runtime agentique local-first, graph-native et compute-aware,
conçu pour permettre à des agents IA de raisonner, mémoriser et proposer des actions,
tandis que le runtime gouverne ce qui peut réellement se produire.
```

Ou en version plus courte :

```text
Des agents capables de penser et proposer,
un runtime conçu pour contrôler, tracer et améliorer.
```
