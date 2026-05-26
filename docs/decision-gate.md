# Decision Gate

Le Decision Gate est le premier composant concret du flux :

```text
ProposedAction → DecisionGate → Decision → AuditEvent
                                    ↓
                         OverrideEngine (si RequiresOverride)
                                    ↓
                         ApprovedByOverride / Blocked
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

Sortie : `Decision` avec statut explicite, raison lisible, politiques appliquées.

## Règles alpha

Ordre d'évaluation :

1. **Auto-grant read-only informational** : si l'action a un risque `Informational`, un type read-only (`ReadMemory`, `ReadTasks`, `ReadProposedActions`, `ReadPendingActions`, `ReadDecisions`, `ReadAudit`, `ReadStatus`) ET que son payload contient `"read_only": true` (posé par `read_only_turn` quand l'utilisateur demande explicitement une lecture) → `Approved` sans vérification de permission.

2. **Permission manquante avec override possible** : si une permission requise n'est pas accordée ET que l'action est overridable (read-only, risque Informational/Low, non destructive) → `RequiresOverride` avec `override_hint` renseigné.

3. **Permission manquante sans override** : si une permission requise n'est pas accordée ET que l'action n'est pas overridable (write, destructive, risque ≥ Medium) → `Blocked` sans `override_hint`.

4. Action `Custom` sans politique d'action active explicite → `NeedsHumanApproval`.

5. Politique applicable qui exige une validation humaine → `NeedsHumanApproval`.

6. Action `High` ou `Critical` couverte par une politique applicable qui ne demande pas de validation humaine → `Blocked`.

7. Risque `Informational` ou `Low` → `Approved` si permissions accordées et aucune politique applicable ne force l'escalade.

8. Risque `Medium` → `NeedsHumanApproval`.

9. Risque `High` ou `Critical` sans politique bloquante → `NeedsHumanApproval`.

## Mécanisme d'Override

### Statuts

| Statut | Signification |
|--------|---------------|
| `RequiresOverride` | Blocé, mais un administrateur peut outrepasse |
| `ApprovedByOverride` | Action approuvée après override réussi |

### Classification

La fonction `classify_override_policy(action) → OverridePolicy` détermine si une action est overridable :

| Action | Overridable ? |
|--------|---------------|
| Read-only, Informational/Low, permissions manquantes | ✅ `PasswordRequired` |
| Write (WriteMemory, WriteDocument, etc.) | ❌ `NotOverridable` |
| Destructive (SimulateEmail, ProposeToolUse, Custom) | ❌ `NotOverridable` |
| Risque Medium ou plus | ❌ `NotOverridable` |

### Flux d'override

```text
RequiresOverride
    ↓
POST /proposed-actions/{id}/override  { "password": "...", "actor": "..." }
    ↓
OverrideEngine.attempt_override(password)
    ↓
├── Password correct → ApprovedByOverride (action uniquement pour {id})
├── Mot de passe incorrect → OverrideOutcome::Failed
├── Trop d'échecs → OverrideOutcome::Locked
└── NotOverridable → OverrideOutcome::NotOverridable
```

### Endpoint HTTP

`POST /proposed-actions/:id/override`

**Request body :**
```json
{
  "password": "alpha-override-password",
  "actor": "admin-thibaud"
}
```

**Response body :**
```json
{
  "decision": { "status": "approved_by_override", ... },
  "audit_event": { "event_type": "override_approved", ... },
  "outcome": "approved"
}
```

Le mot de passe est lu depuis la variable d'environnement `ARPAGONA_OVERRIDE_PASSWORD`. Par défaut (développement) : `alpha-override-password`.

### Configuration

- `OverrideConfig.max_failed_attempts` : tentatives avant verrouillage (défaut : 3)
- `OverrideConfig.lockout_seconds` : durée du verrouillage (défaut : 300s = 5 min)
- Le TTL a été supprimé : il n'y a pas de session d'override. Chaque tentative vérifie le mot de passe indépendamment.

### Vérification du mot de passe

L'override engine utilise un trait `PasswordVerifier` :

```rust
pub trait PasswordVerifier: Debug + Send + Sync {
    fn verify(&self, password: &str) -> bool;
}
```

Trois implémentations fournies :

- **`Argon2PasswordVerifier`** (production) : utilise Argon2id (Algorithme de référence OWASP). Lit le hash depuis `ARPAGONA_OVERRIDE_PASSWORD_HASH`.
- **`DefaultHasherVerifier`** (alpha/dev) : utilise `std::hash::DefaultHasher` avec un sel. **Non cryptographique** — développement/test uniquement.
- **`StaticTestPasswordVerifier`** (tests unitaires) : compare directement le mot de passe attendu. **Jamais en production.**

### Configuration production

La variable d'environnement `ARPAGONA_OVERRIDE_PASSWORD_HASH` attend un hash Argon2id au format PHC :

```
$argon2id$v=19$m=19456,t=2,p=1$<base64-salt>$<base64-hash>
```

Générer un hash pour le mot de passe "mon-mot-de-passe" :

```bash
# Option 1 : avec l'outil argon2 CLI
echo -n "mon-mot-de-passe" | argon2 "$(openssl rand -base64 16)" -id -t 2 -m 19 -p 1 -l 32

# Option 2 : avec Python
python3 -c "
from argon2 import PasswordHasher
ph = PasswordHasher(time_cost=2, memory_cost=19456, parallelism=1, hash_len=32)
print(ph.hash('mon-mot-de-passe'))
"
```

Puis exporter le hash :

```bash
export ARPAGONA_OVERRIDE_PASSWORD_HASH='$argon2id$v=19$m=19456,t=2,p=1$...'
```

**Attention** : ne pas mettre le hash entre `'...'` quotes simples dans le shell si le hash contient des caractères spéciaux `$`.

### Configuration développement

En développement uniquement, deux options :

1. **Mode dev explicite** : `ARPAGONA_ALLOW_DEV_OVERRIDE=true` — utilise un mot de passe fixe `alpha-override-password` si aucun hash ni mot de passe n'est configuré.
2. **Mot de passe personnalisé** : `ARPAGONA_OVERRIDE_PASSWORD=mon-pass-dev` + `ARPAGONA_ALLOW_DEV_OVERRIDE=true`

⚠️ Ne pas utiliser le mode dev en production. Le hash Argon2 est la seule configuration production.

### Ordonnancement de la configuration

L'initialisation suit cet ordre de priorité :

1. `ARPAGONA_OVERRIDE_PASSWORD_HASH` → `Argon2PasswordVerifier` **(production)**
2. `ARPAGONA_OVERRIDE_PASSWORD` + `ARPAGONA_ALLOW_DEV_OVERRIDE=true` → `DefaultHasherVerifier` **(dev)**
3. `ARPAGONA_ALLOW_DEV_OVERRIDE=true` seul → fallback `alpha-override-password` **(dev)**
4. Aucune variable → override désactivé (endpoint retourne `override_not_configured`)

### Sécurité

- Override est strictement **single-action scoped** : un override réussi sur action-25 n'affecte **pas** action-26. Chaque action doit être override individuellement.
- **No global override session.** L'approbation d'une action ne crée jamais de session admin globale ou temporaire.
- Chaque tentative d'override vérifie le mot de passe indépendamment — il n'y a pas de fenêtre TTL qui bypasserait la vérification.
- Le mot de passe n'est jamais stocké en clair (Argon2PasswordVerifier stocke le hash PHC ; DefaultHasherVerifier stocke le hash u64)
- Le mot de passe n'est jamais inclus dans les logs, les événements d'audit, les messages d'erreur ou le Debug output
- Verrouillage temporaire après N échecs consécutifs (anti brute-force)
- Pas d'override global — chaque action est évaluée individuellement
- Les actions destructrices (SimulateEmail, ProposeToolUse, Custom), les écritures et les actions à risque ≥ Medium ne sont jamais overridables

### Audit

Les événements d'audit contiennent toujours : `action_id`, `decision_id` (si disponible), `actor`, `override_policy` et `timestamp`.

## Audit

| Type | Produit quand |
|------|---------------|
| `DecisionCreated` | `evaluate_decision_gate` est appelée |
| `OverrideRequested` | Une décision `RequiresOverride` est émise par le Decision Gate |
| `OverrideApproved` | Le mot de passe soumis est correct |
| `OverrideFailed` | Le mot de passe soumis est incorrect |
| `OverrideExpired` | Ce type n'est plus produit (TTL retiré du moteur d'override) |

### Construction

Le helper `AuditEvent::override_event(...)` crée un événement avec :
- `action_id`, `decision_id` (si disponible)
- `override_status` ("approved", "failed", "locked", "expired")
- `action_type`, `risk_level`
- `timestamp`

Le mot de passe n'est JAMAIS inclus dans l'audit event, les logs ou le Debug output.

## Sémantique policy minimale

La structure `Policy` actuelle est volontairement simple pour l'alpha :
- `enabled = false` : politique ignorée.
- `applies_to_action_type = None` : politique globale pour tous les types d'action.
- `risk_threshold = None` : politique applicable à tous les niveaux de risque.
- `risk_threshold = Some(x)` : applicable à partir de ce niveau de risque.
- `requires_human_approval = true` : escalade en validation humaine.
- `requires_human_approval = false` sur `High` / `Critical` : politique bloquante alpha.

## Limites alpha

- Pas de persistance de l'override engine (l'état de verrouillage est en mémoire, perdu au redémarrage)
- Le mot de passe administrateur est lu depuis une variable d'environnement mais pas depuis un fichier sécurisé / vault
- Pas d'API HTTP pour configurer l'engine (max_attempts, lockout_duration)
- Le verrouillage est global (toutes les actions partagent le compteur d'échecs)
- Pas de tests d'intégration HTTP pour l'endpoint d'override (tests unitaires uniquement)
- Pas d'exécution d'outils après approbation
- `Argon2PasswordVerifier` utilise les paramètres par défaut d'Argon2id (m=19456, t=2, p=1). Ces paramètres sont encapsulés dans le hash PHC et ne sont pas configurables séparément.

## Garanties architecturales

### 1. Single-action scoping

Override est strictement **single-action scoped** :
1. Un override réussi sur action-25 produit `ApprovedByOverride` **uniquement** pour action-25. Aucune autre action n'est modifiée.
2. Aucune session admin globale ou temporaire n'est créée. L'approbation d'une action ne donne pas accès aux autres.
3. Chaque tentative d'override vérifie le mot de passe indépendamment — il n'y a pas de fenêtre TTL qui bypasserait la vérification.
4. L'audit `OverrideApproved` contient toujours l'`action_id` ciblé.
5. Aucun état global de type `admin_session_active` n'existe dans le système.

### 2. Idempotence

Si une action est déjà `ApprovedByOverride`, une nouvelle tentative d'override :
- Ne crée **pas** de nouvelle décision
- Ne crée **pas** d'audit event dupliqué dangereux
- Retourne `already_approved` avec la décision existante
- Produit un audit event léger (`override_already_approved`)

### 3. Anti-mutation fingerprint

Quand une décision `RequiresOverride` est créée, un fingerprint stable de l'action est calculé et stocké dans `Decision.action_fingerprint`. Le fingerprint inclut :
- `action_id`
- `action_type` (sérialisé)
- `risk_level` (sérialisé)
- `payload` (sérialisé)
- `required_permissions` (sérialisé)

Lors de la tentative d'override, le fingerprint est vérifié. Si l'action a changé depuis la décision, l'override est refusé et un audit `OverrideFailed` est produit.

### 4. Séparation décision / exécution

| Statut | Signification |
|--------|---------------|
| `ApprovedByOverride` | **Autorisé** — le Decision Gate a approuvé l'action après override |
| `ExecutionSucceeded / ExecutionFailed` | **Résultat** — l'exécution réelle de l'outil (non branchée dans cette version) |

`ApprovedByOverride` ne déclenche **pas** l'exécution réelle. L'exécution est un mécanisme séparé, non branché dans la version alpha.
