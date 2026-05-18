# Decision Gate

Le Decision Gate est le premier composant concret du flux :

```text
ProposedAction -> DecisionGate -> Decision -> AuditEvent
```

Il vit dans `crates/core` et reste volontairement pur Rust : pas d'API Axum, pas de LLM, pas d'exécution d'outils, pas de shell et pas d'I/O. Les agents proposent ; le système décide.

## API domaine

La fonction principale est :

```rust
evaluate_proposed_action(action, policies, granted_permissions) -> Decision
```

Entrées :

- `ProposedAction` : intention structurée produite par un agent.
- `Policy` : politiques actives ou inactives applicables au type d'action et au niveau de risque.
- `granted_permissions` : permissions actuellement accordées au contexte d'évaluation.

Sortie :

- `Decision` avec un statut explicite : `Approved`, `NeedsHumanApproval` ou `Blocked`.
- Une raison lisible dans `Decision.reason`.
- Les politiques appliquées dans `Decision.policies_applied`.
- `decided_by = System`, car l'alpha encode une décision système déterministe.

## Règles alpha

Ordre d'évaluation :

1. Permission manquante : `Blocked`.
2. Action `Custom` sans politique d'action active explicite : `NeedsHumanApproval`.
3. Politique applicable qui exige une validation humaine : `NeedsHumanApproval`.
4. Action `High` ou `Critical` couverte par une politique applicable qui ne demande pas de validation humaine : `Blocked`.
5. `Informational` ou `Low` : `Approved` si les permissions sont accordées et qu'aucune politique applicable ne force l'escalade.
6. `Medium` : `NeedsHumanApproval`.
7. `High` ou `Critical` sans politique bloquante : `NeedsHumanApproval`.

## Sémantique policy minimale

La structure `Policy` actuelle est volontairement simple pour l'alpha :

- `enabled = false` : politique ignorée.
- `applies_to_action_type = None` : politique globale pour tous les types d'action.
- `risk_threshold = None` : politique applicable à tous les niveaux de risque.
- `risk_threshold = Some(x)` : applicable à partir de ce niveau de risque.
- `requires_human_approval = true` : escalade en validation humaine.
- `requires_human_approval = false` sur `High` / `Critical` : politique bloquante alpha.

Cette sémantique est minimale et devra probablement évoluer vers un effet explicite (`allow`, `require_human_approval`, `block`) quand les politiques deviendront configurables via API.

## Audit

Le helper :

```rust
audit_event_for_decision(action, decision) -> AuditEvent
```

crée un `AuditEventType::DecisionCreated` qui relie :

- le workspace ;
- la tâche éventuelle ;
- la `ProposedAction` ;
- la `Decision` ;
- le statut, la raison et les politiques appliquées dans le payload.

Le helper ne persiste rien. Le stockage reste la responsabilité de Graph Memory / Audit Store.

## Limites alpha

- Pas de persistance.
- Pas d'API HTTP.
- Pas de modèle policy riche avec effet explicite.
- Pas de contexte workspace avancé.
- Pas de validation humaine interactive.
- Pas d'exécution d'outils après approbation.

La prochaine étape logique est un serveur API minimal qui expose les actions proposées, les décisions et les événements d'audit sans introduire encore d'exécution d'outils.
