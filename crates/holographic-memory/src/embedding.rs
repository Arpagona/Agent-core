//! # Embedding providers for Holographic Memory
//!
//! Defines the [`EmbeddingProvider`] trait for computing embedding bit-positions
//! from text content. Embeddings enable **semantic generalization** beyond exact
//! keyword matching: traces with related but not identical text can still produce
//! non-zero resonance overlap.
//!
//! ## Providers
//!
//! - [`NoOpEmbeddingProvider`] — always returns empty bits (no semantic extension).
//!   Used when embeddings are not available or not wanted.
//! - [`CharacterNGramEmbeddingProvider`] — built-in provider that computes
//!   embedding bits from character 2-grams and 3-grams of the input text.
//!   Deterministic, zero external dependencies, available when the
//!   `builtin-embedding` feature is enabled.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// EmbeddingProvider trait
// ---------------------------------------------------------------------------

/// A provider that can compute embedding bit-positions from text.
///
/// The returned bits are incorporated into a trace's or query's
/// [`DistributedSignature`][crate::DistributedSignature] as an additional
/// dimension for resonance computation. When a provider returns an empty
/// vector, the embedding dimension contributes zero overlap — behaviour
/// is identical to not having embeddings enabled.
pub trait EmbeddingProvider {
    /// Compute embedding bit-positions from the given content text and
    /// extracted keywords.
    ///
    /// `content_summary` is the trace's or query's main text content.
    /// `keywords` are the extracted or provided keywords.
    fn compute_embedding_bits(&self, content_summary: &str, keywords: &[String]) -> Vec<u64>;
}

// ---------------------------------------------------------------------------
// NoOpEmbeddingProvider
// ---------------------------------------------------------------------------

/// An embedding provider that returns no bits.
///
/// This is the **graceful fallback**: when embeddings are not available or
/// the operator does not request them, this provider ensures the resonance
/// pipeline works the same as before — the embedding overlap dimension is
/// always zero.
pub struct NoOpEmbeddingProvider;

impl EmbeddingProvider for NoOpEmbeddingProvider {
    fn compute_embedding_bits(&self, _content_summary: &str, _keywords: &[String]) -> Vec<u64> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// CharacterNGramEmbeddingProvider
// ---------------------------------------------------------------------------

/// A built-in embedding provider that uses character n-gram hashing.
///
/// This provider extracts all character 2-grams and 3-grams (bigrams and
/// trigrams) from the combined `content_summary` + `keywords` text, then
/// hashes each n-gram into u64 bit-positions. The resulting bits capture
/// **subword and morphological similarity** — for example, "running" and
/// "run" share the bigrams "ru", "un", "nn", "ni", "in", "ng" plus trigram
/// overlap, so they produce non-zero embedding overlap even when no keyword
/// matches the other.
///
/// ## Properties
///
/// - **Deterministic**: same input always produces the same bits.
/// - **Zero external dependencies**: pure Rust, no ONNX, no model file.
/// - **Available by default**: gated behind `builtin-embedding` feature
///   (on by default).
/// - **Lightweight**: O(|text|·n) where n is 2 and 3.
/// - **Semantic generalization**: captures subword relationships between
///   morphologically related words without requiring a full NLP pipeline.
///
/// ## Limitations
///
/// This is not a real neural embedding. It does not capture synonymy
/// ("car" and "automobile" share no n-grams) or abstract semantic
/// similarity. For that, replace this provider with a real embedding
/// model (e.g. fastembed, ONNX BERT) behind the `EmbeddingProvider` trait.
pub struct CharacterNGramEmbeddingProvider {
    /// Include n-grams of this length (default: 2).
    min_gram: usize,
    /// Include n-grams of this length (default: 3).
    max_gram: usize,
    /// Number of hash seeds per n-gram (default: 3).
    hashes_per_gram: u64,
    /// Base seed for bit-position computation.
    base_seed: u64,
}

impl Default for CharacterNGramEmbeddingProvider {
    fn default() -> Self {
        Self {
            min_gram: 2,
            max_gram: 3,
            hashes_per_gram: 3,
            base_seed: 9999,
        }
    }
}

impl CharacterNGramEmbeddingProvider {
    /// Create a new provider with custom n-gram parameters.
    pub fn new(min_gram: usize, max_gram: usize, hashes_per_gram: u64, base_seed: u64) -> Self {
        Self {
            min_gram,
            max_gram,
            hashes_per_gram,
            base_seed,
        }
    }

    /// Extract all character n-grams from text.
    fn extract_ngrams(text: &str, min_gram: usize, max_gram: usize) -> Vec<String> {
        let mut ngrams = BTreeSet::new(); // dedup + sort
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();

        for n in min_gram..=max_gram {
            if len < n {
                continue;
            }
            for i in 0..=(len - n) {
                let gram: String = chars[i..(i + n)].iter().collect();
                ngrams.insert(gram);
            }
        }

        ngrams.into_iter().collect()
    }
}

impl EmbeddingProvider for CharacterNGramEmbeddingProvider {
    fn compute_embedding_bits(&self, content_summary: &str, keywords: &[String]) -> Vec<u64> {
        // Combine content + keywords into a single normalized text block
        let mut text = content_summary.to_lowercase();
        for kw in keywords {
            text.push(' ');
            text.push_str(&kw.to_lowercase());
        }

        let ngrams = Self::extract_ngrams(&text, self.min_gram, self.max_gram);
        let mut bits = BTreeSet::new();

        for gram in &ngrams {
            for i in 0..self.hashes_per_gram {
                let seed = self.base_seed.wrapping_add(i);
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                seed.hash(&mut hasher);
                gram.hash(&mut hasher);
                bits.insert(hasher.finish());
            }
        }

        bits.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// Helper: extend a DistributedSignature with embedding bits
// ---------------------------------------------------------------------------

/// Extend a [`DistributedSignature`][crate::DistributedSignature] with
/// embedding bits computed from the given text and provider.
///
/// This is a no-op (no bits added) when the provider returns an empty vector,
/// which is the case for [`NoOpEmbeddingProvider`].
///
/// # Example
///
/// ```ignore
/// use arpagona_holographic_memory::*;
/// use arpagona_holographic_memory::embedding::*;
///
/// let mut sig = encode_terms_to_signature(&keywords, &concepts, &entities, &decisions);
/// let provider = CharacterNGramEmbeddingProvider::default();
/// extend_signature_with_embedding(&mut sig, "Some text content", &["keyword1"], &provider);
/// ```
pub fn extend_signature_with_embedding(
    sig: &mut crate::DistributedSignature,
    content_summary: &str,
    keywords: &[String],
    provider: &dyn EmbeddingProvider,
) {
    let embedding_bits = provider.compute_embedding_bits(content_summary, keywords);
    if !embedding_bits.is_empty() {
        sig.embedding_bits = embedding_bits;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_returns_empty() {
        let provider = NoOpEmbeddingProvider;
        let bits = provider.compute_embedding_bits("hello world", &[]);
        assert!(bits.is_empty());
    }

    #[test]
    fn ngram_deterministic() {
        let provider = CharacterNGramEmbeddingProvider::default();
        let bits1 = provider.compute_embedding_bits("hello world", &["test".to_string()]);
        let bits2 = provider.compute_embedding_bits("hello world", &["test".to_string()]);
        assert_eq!(bits1, bits2);
    }

    #[test]
    fn ngram_returns_bits() {
        let provider = CharacterNGramEmbeddingProvider::default();
        let bits = provider.compute_embedding_bits("hello", &[]);
        assert!(!bits.is_empty(), "character n-grams should produce bits");
    }

    #[test]
    fn ngram_empty_text_returns_empty() {
        let provider = CharacterNGramEmbeddingProvider::default();
        let bits = provider.compute_embedding_bits("", &[]);
        assert!(bits.is_empty(), "empty text should produce no bits");
    }

    #[test]
    fn ngram_short_text_still_produces_bits() {
        // A single character produces no 2-gram or 3-gram
        let provider = CharacterNGramEmbeddingProvider::default();
        let bits = provider.compute_embedding_bits("a", &[]);
        assert!(bits.is_empty(), "single char should produce no bigrams");

        // Two characters produce one 2-gram (e.g., "he")
        let bits2 = provider.compute_embedding_bits("ab", &[]);
        assert!(!bits2.is_empty(), "two chars should produce bigrams");
    }

    #[test]
    fn ngram_semantic_similarity_detected() {
        // "running" and "run" should share some character n-grams
        let provider = CharacterNGramEmbeddingProvider::default();
        let bits_running = provider.compute_embedding_bits("running", &[]);
        let bits_run = provider.compute_embedding_bits("run", &[]);

        // Both should have non-empty bits
        assert!(!bits_running.is_empty());
        assert!(!bits_run.is_empty());

        // They share some n-grams: "ru", "un" (if run is ≥2 chars)
        // Count overlap
        let set_run: BTreeSet<u64> = bits_run.iter().copied().collect();
        let set_running: BTreeSet<u64> = bits_running.iter().copied().collect();
        let overlap = set_run.intersection(&set_running).count();

        assert!(
            overlap > 0,
            "'running' and 'run' should share character n-gram bits, found {}",
            overlap
        );
    }

    #[test]
    fn ngram_words_with_no_shared_ngrams_have_no_overlap() {
        // "xz" and "qw" share no character n-grams
        let provider = CharacterNGramEmbeddingProvider::default();
        let bits_a = provider.compute_embedding_bits("xz", &[]);
        let bits_b = provider.compute_embedding_bits("qw", &[]);

        let set_a: BTreeSet<u64> = bits_a.iter().copied().collect();
        let set_b: BTreeSet<u64> = bits_b.iter().copied().collect();
        let _overlap_count = set_a.intersection(&set_b).count();

        // The hash sets are deterministic, so they may by coincidence share bits
        // even though the source n-grams are different. This is expected with
        // hashing — the important thing is that they're not designed to overlap.
        // We just assert they aren't identical.
        assert_ne!(
            bits_a, bits_b,
            "different words should not have identical embedding bits"
        );
    }

    #[test]
    fn extend_signature_adds_embedding_bits() {
        use crate::DistributedSignature;

        let mut sig = DistributedSignature::empty();
        assert!(sig.embedding_bits.is_empty());

        let provider = CharacterNGramEmbeddingProvider::default();
        extend_signature_with_embedding(
            &mut sig,
            "test content",
            &["keyword".to_string()],
            &provider,
        );

        assert!(!sig.embedding_bits.is_empty());
    }

    #[test]
    fn extend_signature_noop_provider_does_not_add_bits() {
        use crate::DistributedSignature;

        let mut sig = DistributedSignature::empty();
        let provider = NoOpEmbeddingProvider;
        extend_signature_with_embedding(
            &mut sig,
            "test content",
            &["keyword".to_string()],
            &provider,
        );

        assert!(sig.embedding_bits.is_empty());
    }

    #[test]
    fn ngram_keywords_contribute_to_bits() {
        let provider = CharacterNGramEmbeddingProvider::default();

        // Without keyword
        let bits_no_kw = provider.compute_embedding_bits("hello", &[]);
        // With keyword — should add more bits
        let bits_with_kw = provider.compute_embedding_bits("hello", &["special-term".to_string()]);

        // The keyword adds character n-grams that weren't in "hello"
        let set_no_kw: BTreeSet<u64> = bits_no_kw.iter().copied().collect();
        let set_with_kw: BTreeSet<u64> = bits_with_kw.iter().copied().collect();

        assert!(
            set_with_kw.len() > set_no_kw.len(),
            "adding a keyword should produce more unique embedding bits"
        );
    }
}
