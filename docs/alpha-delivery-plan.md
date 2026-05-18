# Alpha Delivery Plan

Objectif : livrer une version alpha installable d'ARPAGONA Agent Core avant la fin de la semaine du 18 mai 2026.

Cette alpha doit prouver le flux essentiel du produit, pas finaliser toute l'architecture cible.

## Définition de l'alpha

Un utilisateur doit pouvoir installer/lancer ARPAGONA Agent Core localement, créer une tâche, proposer une action, faire évaluer cette action par un Decision Gate minimal, enregistrer une décision et un événement d'audit, puis consulter l'état du système.

Flux obligatoire :

```text
Workspace / Task -> ProposedAction -> DecisionGate -> Decision -> AuditEvent -> Consultation
```

## Principes non négociables

- Aucun agent n'exécute directement.
- Une `ProposedAction` est une intention structurée, pas une exécution.
- Toute action proposée passe par le Decision Gate.
- Toute décision importante produit un `AuditEvent`.
- Aucune exécution d'outil réelle dans l'alpha.
- Aucun shell libre.
- Aucun secret exposé au LLM.
- Aucun appel LLM obligatoire dans l'alpha.
- Le système doit rester local-first et installable.

## Périmètre inclus

### 1. Graph Memory V0

Statut attendu : merge de la branche `feature/graph-memory-v0` si `cargo check` et `cargo test` passent.

Doit fournir :

- contrat domaine `GraphMemoryStore` dans `crates/core` ;
- `InMemoryGraphMemoryStore` pour tests et runtime alpha ;
- `GraphRelation` minimal ;
- adapter SurrealDB expérimental dans `crates/graph-memory` ;
- persistance ou stockage de `Source`, `Fact`, `Episode`, `Observation`, `AuditEvent`, `GraphRelation`.

### 2. Decision Gate minimal

But : transformer une `ProposedAction` en `Decision` selon des règles simples, testables et explicables.

Règles alpha recommandées :

- `RiskLevel::Informational` ou `RiskLevel::Low` -> `Approved`, sauf policy contraire ;
- `RiskLevel::Medium` -> `NeedsHumanApproval` par défaut ;
- `RiskLevel::High` ou `RiskLevel::Critical` -> `NeedsHumanApproval` ou `Blocked` selon policy ;
- permission manquante -> `Blocked` ;
- action inconnue ou `Custom` non explicitement autorisée -> `NeedsHumanApproval` ;
- la raison de la décision doit être lisible par un humain.

Livrables :

- module ou crate Decision Gate pur Rust ;
- tests unitaires ;
- helper pour créer un `AuditEvent` après décision ;
- documentation `docs/decision-gate.md`.

### 3. API Server minimal

But : exposer une vertical slice locale.

Application proposée :

```text
apps/api-server
```

Endpoints alpha :

```text
GET  /health
POST /tasks
GET  /tasks
POST /proposed-actions
GET  /proposed-actions
POST /decision-gate/evaluate
GET  /decisions
GET  /audit
```

Stockage alpha :

- état HTTP in-memory pour `Task`, `ProposedAction`, `Decision` et `AuditEvent` dans l'API server alpha ;
- `InMemoryGraphMemoryStore` reste disponible côté domaine pour tests et développements mémoire ;
- SurrealDB peut rester expérimental et optionnel.

Contraintes :

- pas d'exécution d'outil ;
- pas de LLM ;
- pas de secrets ;
- pas de scheduler.

### 4. CLI installable

But : donner une sensation produit utilisable sans UI web.

Binaire proposé :

```text
arpagona
```

Commandes alpha :

```bash
arpagona serve
arpagona health
arpagona task create "Préparer une réponse client"
arpagona action propose --type simulate_email --risk medium
arpagona audit list
```

L'interface CLI peut appeler l'API locale ou partager un runtime in-memory selon le choix d'implémentation le plus rapide.

### 5. Packaging alpha

Doit permettre au minimum :

```bash
cargo install --path crates/cli
arpagona serve
```

ou :

```bash
cargo run -p arpagona-cli -- serve
```

Livrables :

- `README_ALPHA.md` ;
- commandes d'installation ;
- commandes de démonstration ;
- tag `v0.1.0-alpha` quand la vertical slice est validée.

## Périmètre explicitement exclu

Pour tenir le délai, exclure de l'alpha :

- Mission Control Next.js complet ;
- multi-agent réel ;
- scheduler autonome ;
- navigateur contrôlé ;
- shell ;
- exécution réelle d'outils ;
- appel LLM obligatoire ;
- intégration Telegram ou Discord ;
- mémoire vectorielle ;
- RAG documentaire complet ;
- reservoir computing ;
- UI avancée.

Ces éléments restent dans la vision, mais ne doivent pas bloquer l'alpha.

## Plan de livraison recommandé

### Jour 1

- Merge Graph Memory V0 après tests.
- Implémenter Decision Gate minimal.
- Ajouter tests Decision Gate.

### Jour 2

- Créer `apps/api-server`.
- Ajouter endpoints `health`, `tasks`, `proposed-actions`, `decision-gate/evaluate`, `audit`.
- Utiliser stockage in-memory par défaut.

### Jour 3

- Brancher le Decision Gate dans l'API.
- Enregistrer `Decision` et `AuditEvent`.
- Ajouter scénarios end-to-end.

### Jour 4

- Créer `crates/cli` ou `apps/cli`.
- Ajouter commandes `serve`, `health`, `task create`, `action propose`, `audit list`.

### Jour 5

- Nettoyage.
- `README_ALPHA.md`.
- GitHub Actions : `cargo check`, `cargo test`.
- Test sur machine propre.

### Jour 6-7

- Corrections bugs.
- Gel du périmètre.
- Tag `v0.1.0-alpha`.

## Définition de terminé

La release alpha est terminée si les commandes suivantes fonctionnent sur une machine de développement fraîche :

```bash
git clone <repo>
cd Agent-core
cargo test
cargo run -p arpagona-cli -- serve
cargo run -p arpagona-cli -- task create "Préparer une réponse client"
cargo run -p arpagona-cli -- action propose --type simulate_email --risk medium
cargo run -p arpagona-cli -- audit list
```

Résultat attendu :

```text
Action proposed: simulate_email
Decision: needs_human_approval
Reason: medium risk action requires validation
Audit event recorded
```

## Risque principal

Le risque principal est de confondre alpha installable et produit complet.

La bonne question pour chaque tâche :

```text
Est-ce indispensable pour démontrer le flux Workspace/Task -> ProposedAction -> Decision -> Audit ?
```

Si la réponse est non, reporter après `v0.1.0-alpha`.
