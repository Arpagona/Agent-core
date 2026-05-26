# Holographic Memory — Symbolic Associative Memory Core

**Crate:** `crates/holographic-memory` · **Status:** Alpha V0 · **Stack:** Rust

> **Holographic Memory reactivates paths to truth. It does not replace truth.**

## V0 Constraints

This crate is a **symbolic associative memory kernel**. It is intentionally
limited in V0:

- ✅ **No LLM** — all encoding and retrieval is deterministic hashing
- ✅ **No vector database** — signatures are `Vec<u64>`, not `Vec<f32>`
- ✅ **No persistence** — `InMemoryHolographicMemoryStore` only (volatile)
- ✅ **No tool execution** — pure data operations, no shell/FS/network
- ✅ **No authorization** — retrieval is evidence-only, never an action approval
- ✅ **No replacement of Graph Memory** — Graph Memory remains the source of truth
- ✅ **No replacement of Decision Gate** — all actions still require governance
- ✅ **Deterministic** — same input → same signature → same retrieval

## Définition opérationnelle de “Mémoire Holographique”

Dans ARPAGONA Agent Core, la **mémoire holographique** désigne un
système de mémoire associative où les traces ne sont pas stockées comme
du texte brut mais comme des **signatures distribuées** (un ensemble de
positions binaires déterministes). La récupération se fait par
**résonance** : le système compare la signature d'une requête avec
celles des traces stockées, et retourne les traces dont la signature
"résonne" le plus.

Le terme "holographique" vient de l'analogie avec un hologramme
optique : chaque partie de la signature contient de l'information sur
le tout, et une requête partielle peut reconstruire le contexte
associé.

### Propriétés fondamentales

1. **Distribuée** : l'information n'est pas à un seul endroit mais
   répartie dans les bits de signature.
2. **Associative** : la récupération se fait par similarité, pas par
   identité exacte.
3. **Reconstructive** : le contexte est reconstruit à partir des traces
   existantes, pas inventé.
4. **Déterministe** : mêmes entrées → mêmes signatures → mêmes
   résultats de recherche.
5. **Traçable** : chaque trace garde ses `source_turn_ids` et peut être
   reliée à ses décisions et mémoires associées.

---

## Différence avec historique brut

Un **historique brut** stocke l'information textuellement et la
retourne telle quelle. Exemple :

```
[2026-05-26] Agent a proposé ReadMemory sur document:handbook
```

Une **mémoire holographique** ne stocke pas le texte mais une
signature dérivée de son contenu sémantique (mots-clés, concepts,
entités). La requête "recherche de documentation" peut réactiver la
trace ci-dessus même si les mots exacts ne correspondent pas, grâce à
la résonance entre signatures.

| Aspect               | Historique brut         | Mémoire holographique         |
|----------------------|-------------------------|-------------------------------|
| Stockage             | Texte intégral          | Signature distribuée (bits)   |
| Recherche            | Correspondance exacte   | Résonance (similarité bits)   |
| Reconstruction       | Aucune                  | Contexte lié (décisions, mémoires) |
| Espace               | Proportionnel au texte  | Compact (bits)                |

---

## Différence avec mémoire vectorielle

Une **mémoire vectorielle** (embedding) utilise des vecteurs de
nombres réels (typiquement f32, 384–1536 dimensions) produits par un
modèle de machine learning. La similarité est mesurée par produit
scalaire ou distance cosinus.

La **mémoire holographique symbolique** utilise des bits (u64)
produits par un hachage déterministe. Il n'y a aucun modèle, aucun
entraînement, aucune dépendance externe.

| Aspect               | Mémoire vectorielle           | Mémoire holographique symbolique |
|----------------------|-------------------------------|----------------------------------|
| Représentation      | Vecteurs f32 (embedding LLM) | Bits u64 (hachage déterministe)  |
| Génération          | Modèle de ML                  | Fonction de hachage pure         |
| Déterminisme        | Non (modèle peut varier)      | Oui (mêmes entrées → mêmes bits) |
| Dépendances         | LLM, base vectorielle         | Aucune                           |
| Similarité          | Cosinus / produit scalaire    | Jaccard / chevauchement de bits  |
| Coût                | Élevé (inférence + stockage)  | Très faible                      |

---

## Notion de signature distribuée

Une `DistributedSignature` est composée de quatre vecteurs de `u64` :

```rust
pub struct DistributedSignature {
    pub symbolic_bits: Vec<u64>,   // mots-clés
    pub concept_bits: Vec<u64>,    // concepts
    pub entity_bits: Vec<u64>,     // entités
    pub decision_bits: Vec<u64>,   // décisions liées
}
```

Chaque terme (mot-clé, concept, entité, ID de décision) est :
1. Normalisé (lowercase, trim, dédoublonné)
2. Haché avec `DefaultHasher` en utilisant des seeds différentes
   (42, 100, 200, 300) pour produire 3 positions u64 par terme

Ainsi, le terme `"rust"` produit 3 positions dans `symbolic_bits`,
le concept `"programmation"` produit 3 positions dans `concept_bits`,
etc. L'ensemble des bits constitue la signature distribuée.

### Propriétés

- **Déterministe** : même terme → mêmes bits.
- **Résistant aux collisions** : chaque seed de champ est différente,
  donc les mots-clés n'interfèrent pas avec les concepts.
- **Compact** : les bits sont stockés comme `Vec<u64>`, ordonnés et
  dédoublonnés.

---

## Notion de résonance

La **résonance** est la mesure de similarité entre la signature d'une
requête et celle d'une trace. Elle est calculée par la fonction
`signature_overlap()` :

1. Pour chaque champ (`symbolic_bits`, `concept_bits`, `entity_bits`,
   `decision_bits`), calculer l'indice de Jaccard :
   `|A ∩ B| / |A ∪ B|`
2. Combiner les indices pondérés :
   `total = symbolic * 0.30 + concept * 0.30 + entity * 0.30 + decision * 0.10`
3. Ajouter les boosts :
   - `importance_boost = importance × 0.10`
   - `confidence_boost = confidence × 0.05`
   - `activation_boost = min(activation_count × 0.01, 0.20)`

Les boosts sont des **facteurs de classement**, pas des créateurs de
correspondance. Une trace avec un chevauchement nul dans toutes les
dimensions est exclue même si les boosts sont positifs.

### Seuils

- Une trace n'est incluse dans les résultats que si au moins une
  dimension de chevauchement est > 0 (seuil : 1e-9).
- Les résultats sont triés par score total décroissant.
- Le paramètre `limit` contrôle le nombre maximal de résultats.

---

## Notion de reconstruction contrôlée

La `ReconstructedContext` est le résultat d'une requête par résonance.
Elle contient :

```rust
pub struct ReconstructedContext {
    pub project_id: String,
    pub query: String,
    pub matches: Vec<ResonanceMatch>,          // résultats triés
    pub activated_trace_ids: Vec<String>,       // traces activées
    pub linked_memory_ids: Vec<String>,         // mémoires liées (dédupl.)
    pub linked_decision_ids: Vec<String>,       // décisions liées (dédupl.)
    pub reconstruction_summary: String,         // résumé déterministe
}
```

### Règles de reconstruction

1. **Aucune invention** : le contexte est construit uniquement à partir
   des données des traces existantes.
2. **Expansion associative** : après les premiers matches, les IDs de
   mémoires et décisions liées sont collectés depuis toutes les traces
   correspondantes.
3. **Résumé déterministe** : le `reconstruction_summary` est une chaîne
   sans LLM, de la forme :
   ```
   Found 3 matching traces (top score: 0.8472).
   Activated traces (3): trace-1, trace-2, trace-3
   Linked decisions (2): decision-abc, decision-xyz
   Linked memories (2): mem-001, mem-002
   ```
4. **Traçabilité** : chaque `ResonanceMatch` contient la trace complète
   avec ses `source_turn_ids` et sa signature.

---

## Limites actuelles

1. **Store in-memory uniquement** : pas de persistance entre les
   redémarrages.
2. **Pas d'embeddings locaux** : la signature est purement symbolique
   (hachage de termes), pas sémantique (pas de généralisation à des
   termes proches non listés).
3. **Pas de graphe mémoire** : les liens entre traces sont explicites
   (`linked_memory_ids`) mais pas explorés récursivement.
4. **Pas de consolidation** : les traces ne sont pas fusionnées,
   résumées ou nettoyées automatiquement.
5. **Pas d'intégration avec le Decision Gate** : la mémoire ne peut
   pas encore être consultée pendant l'évaluation des décisions.
6. **Recherche linéaire** : `retrieve_by_resonance` scanne toutes les
   traces du projet à chaque requête. Pas d'index.

---

## Prochaines étapes

### 1. Intégration avec conversation-memory

Connecter `HolographicMemoryStore` au système de
`conversation-memory` existant : lorsqu'une conversation est
archivée, encoder ses tours comme des `HolographicTrace` et les
ajouter au store.

### 2. Embeddings locaux

Ajouter un encodeur local optionnel (hors ligne, sans LLM) qui
produit des signatures bitmap à partir d'embedding de mots simples
(type word2vec léger ou SVD sur co-occurrence). Cela permettrait une
généralisation sémantique (ex: "voiture" ≈ "automobile") sans
dépendre d'un LLM.

### 3. Graphe mémoire

Implémenter une expansion récursive : après un premier match, suivre
les `linked_memory_ids` pour récupérer les traces voisines. Ajouter
un paramètre `depth` à `retrieve_by_resonance`.

### 4. Persistance

Ajouter un store basé sur SQLite ou SurrealDB (comme
`graph-memory`). Sauvegarder/charger les traces, avec transactions
et récupération.

### 5. Consolidation périodique

Fusionner les traces redondantes :
- Traces avec des signatures très proches (Jaccard > 0.9)
- Augmenter `importance` et `confidence` des traces consolidées
- Supprimer les doublons explicites

### 6. Gouvernance des écritures par Decision Gate

Toute écriture dans la mémoire holographique devrait passer par le
Decision Gate. Créer un `MemoryWriteKind::HolographicTrace` et
intégrer l'`HolographicMemoryStore` comme une cible d'écriture
gouvernée.

---

## Tests

18 tests implémentés dans `crates/holographic-memory/src/lib.rs` :

| #  | Nom du test                                      | Ce qu'il vérifie                              |
|----|--------------------------------------------------|-----------------------------------------------|
| 1  | `add_trace_and_list_by_project`                  | Ajout et liste par projet                     |
| 2  | `project_scope_prevents_memory_leak`             | Isolation entre projets                       |
| 3  | `deterministic_signature_encoding`               | Mêmes entrées → mêmes signatures              |
| 4  | `same_terms_same_signature`                      | Ordre des termes n'affecte pas la signature   |
| 5  | `different_terms_different_signature`            | Termes différents → signatures différentes    |
| 6  | `resonance_retrieval_matches_keyword`            | Recherche par mot-clé fonctionne              |
| 7  | `resonance_retrieval_matches_concept`            | Recherche par concept fonctionne              |
| 8  | `resonance_retrieval_matches_entity`             | Recherche par entité fonctionne               |
| 9  | `high_confidence_scores_above_low_confidence`    | La confiance affecte le classement            |
| 10 | `importance_boost_affects_ranking`               | L'importance affecte le classement            |
| 11 | `activation_count_increases_after_retrieval`     | L'activation est incrémentée                  |
| 12 | `empty_query_returns_empty_context`              | Requête vide = contexte vide                  |
| 13 | `linked_decisions_are_returned`                  | Décisions liées sont collectées               |
| 14 | `linked_memories_are_returned`                   | Mémoires liées sont collectées                |
| 15 | `source_turn_ids_are_preserved`                  | IDs de tours sont préservés                   |
| 16 | `no_trace_above_threshold_returns_empty_context` | Termes différents = aucun résultat            |
| 17 | `retrieval_order_is_score_descending`            | Ordre décroissant par score                   |
| 18 | `activation_does_not_cross_project_scope`        | L'activation ne traverse pas les projets      |
