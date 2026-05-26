# Mémoire holographique symbolique

> **Crate :** `arpagona-conversation-memory`  
> **Périmètre :** Résonance conversationnelle — retrieval par mots-clés, concepts, entités et liens de décision  
> **Dépendances :** chrono, serde, serde_json (aucune dépendance vers le core crate)

## Ce qu'est la mémoire holographique (symbolique)

Une **mémoire holographique** stocke l'information de façon *distribuée* : chaque trace contient une signature composite (mots-clés, concepts, entités, liens de décision) plutôt qu'un texte brut. Cette signature permet à une requête partielle de *résonner* avec des traces connexes — un peu comme un hologramme dont chaque fragment contient l'image du tout.

Dans cette implémentation, **il n'y a pas d'embeddings ni de vecteurs**. La résonance est purement **symbolique** :

1. Une trace est stockée avec ses mots-clés, concepts, entités explicites.
2. Une requête fournit des mots-clés, concepts, entités à chercher.
3. Le scoring calcule le chevauchement entre requête et trace.
4. Les traces les plus résonantes sont retournées.

## Architecture

### Types principaux

| Type | Rôle |
|------|------|
| `HolographicTrace` | Une trace de moment conversationnel avec signature symbolique |
| `ResonanceQuery` | Requête de résonance (mots-clés, concepts, entités + seuils) |
| `ResonanceMatch` | Résultat d'appariement : trace + score total + détail des facteurs |
| `ResonanceScore` | Score individuel d'un facteur (ex: "keyword_match" = 0.5) |
| `ReconstructedContext` | Contexte reconstruit à partir des traces ayant résonné |
| `ConversationMemoryStore` | Store in-memory, scope par `project_id` |
| `HolographicTraceBuilder` | Builder fluent pour créer des traces |

### HolographicTrace

```rust
pub struct HolographicTrace {
    pub id: String,
    pub project_id: String,
    pub source_memory_id: Option<String>,
    pub source_turn_ids: Vec<String>,
    pub content_summary: String,
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub concepts: Vec<String>,
    pub linked_decision_ids: Vec<String>,
    pub importance: f64,
    pub confidence: f64,
    pub recency_score: f64,
    pub activation_count: u64,
    pub created_at: DateTime<Utc>,
    pub last_activated_at: Option<DateTime<Utc>>,
}
```

### ConversationMemoryStore

Méthodes publiques :

| Méthode | Description |
|---------|-------------|
| `add_holographic_trace(trace)` | Ajoute une trace dans le store. Retourne l'ID. |
| `list_holographic_traces(project_id)` | Liste les traces d'un projet (plus récentes d'abord). |
| `retrieve_by_resonance(project_id, query, top_k)` | Recherche par résonance symbolique. |
| `activate_trace(trace_id)` | Incrémente `activation_count`, met à jour `last_activated_at`. |
| `reconstruct_context(project_id, query, top_k)` | Collationne concepts/entités/mots-clés/décisions des traces résonantes. |

## Règles de scoring

Chaque trace est notée selon ces facteurs quand une requête contient des signaux symboliques :

| Facteur | Contribution max | Condition |
|---------|-----------------|-----------|
| Keyword match | +0.75 (0.25/match) | Mot-clé de la requête présent dans les keywords de la trace |
| Concept match | +0.60 (0.20/match) | Concept de la requête présent dans les concepts de la trace |
| Entity match | +0.45 (0.15/match) | Entité de la requête présente dans les entités de la trace |
| Importance | +0.10 × importance | Toujours ajouté si au moins un match symbolique |
| Confidence | +0.10 × confidence | Toujours ajouté si au moins un match symbolique |
| Recency | +0.10 × recency_score | Toujours ajouté si au moins un match symbolique |
| Activation | +0.05 × min(count,5)/5 | Toujours ajouté si au moins un match symbolique |

**Règle fondamentale** : Un résultat n'est retourné que si **au moins un match symbolique** (keyword, concept ou entity) est trouvé. Ceci empêche :
- Les hallucinations (requête vide → 0 résultats)
- Les fuites entre projets (mauvais mot de keywords → 0 résultats)
- Le bruit (tous les enregistrements ne sont pas retournés par défaut)

Score total plafonné à 1.0.

## Ordonnancement

Les résultats sont triés par score total décroissant, puis limités par `top_k`.

## Différence avec la recherche vectorielle

| Aspect | Vectorielle (future) | Symbolique (actuelle) |
|--------|---------------------|----------------------|
| Similarité | Cosine similarity sur embeddings | Chevauchement de keywords/concepts/entités |
| Déterminisme | Non (dépend du modèle) | Oui (pure logique déterministe) |
| LLM | Requis pour embeddings optionnel | Zéro appel LLM |
| DB externe | Vector database optionnelle | HashMap mémoire |
| Hallucination | Possible (faux positifs sémantiques) | Impossible (match exact uniquement) |
| Testabilité | Difficile (dépend du modèle de plongement) | Triviale (déterministe) |
| Coût | Élevé (inférence + stockage vecteurs) | Négligeable |

## Différence avec la mémoire brute / faits structurés

- `GraphMemoryStore` (crate `graph-memory`) stocke des faits autoritatifs sur des entités (facts, sources, episodes).
- `HolographicPattern` (core crate) stocke des patterns vectoriels pour la détection de motifs récurrents.
- `HolographicTrace` (ce crate) stocke des **signatures de résonance** pour la mémoire conversationnelle.

Cette couche **n'invente jamais de contenu**. Elle ne fait que lier des traces existantes entre elles via des métadonnées explicites.

## Project scope

Toutes les opérations sont scopées par `project_id`. Aucune trace d'un projet A ne peut fuiter dans une requête de résonance pour le projet B.

## Tests (14 tests)

| # | Test | Vérifie |
|---|------|---------|
| 1 | `test_add_and_list_by_project` | Ajout et listage scope par projet |
| 2 | `test_resonance_keyword_match` | Match par mot-clé |
| 3 | `test_resonance_concept_match` | Match par concept (même si texte exact différent) |
| 4 | `test_no_project_leak` | Pas de fuite entre projets |
| 5 | `test_activation_count_increments` | Incrémentation de l'activation |
| 6 | `test_linked_decisions_in_reconstructed_context` | Décisions liées dans ReconstructedContext |
| 7 | `test_confidence_affects_score` | La confiance haute score plus haut |
| 8 | `test_source_turn_ids_preserved` | source_turn_ids conservés |
| 9 | `test_empty_query_no_hallucination` | Requête vide → 0 résultats |
| 10 | `test_auto_generated_id` | ID auto-généré si absent |
| 11 | `test_threshold_filters_exclude_low_confidence` | Filtre min_confidence |
| 12 | `test_threshold_filters_exclude_low_importance` | Filtre min_importance |
| 13 | `test_top_k_limits_results` | Limite top_k respectée |
| 14 | `test_reconstruct_context_empty_when_no_match` | reconstruct_context vide si pas de match |

## Limites actuelles

- Store purement en mémoire (pas de persistance)
- Pas de timeout / éviction des traces anciennes
- Pas d'embeddings locaux (future extension)
- Pas de détection de patterns transversaux (future extension)
- Pas de graphe mémoire (chaque trace est indépendante)
- Pas d'API HTTP (utilisable uniquement en Rust pour l'instant)
- Le scoring est heuristique et non appris

## Futures extensions

1. **Embeddings locaux** : ajouter un `vector: Vec<f32>` optionnel pour affiner la résonance par similarité cosinus, sans dépendre d'une vector database externe.
2. **Graphe mémoire** : lier les traces entre elles par des relations de cause/conséquence/succession pour permettre une reconstruction de contexte plus riche.
3. **Persistance** : backend SQLite ou fichier pour conserver les traces entre les redémarrages.
4. **Éviction** : politique de rétention (TTL, nombre max de traces par projet).
5. **API** : exposition HTTP via l'API server existant.
6. **Pattern detection** : détecter des patterns récurrents (mêmes concepts qui reviennent dans plusieurs traces) et les promouvoir en `HolographicPattern`.
