# Compressed Convolutional Memory Retrieval

**Status:** Alpha experimental crate

**Crate:** `crates/compressed-cognitive-attention` (`arpagona-compressed-cognitive-attention`)

## Purpose

This crate implements an experimental memory-retrieval mechanism inspired by Compressed Convolutional Attention. It answers:

> *What memories are relevant when each memory is interpreted with its local temporal neighborhood?*

The key idea is that a memory event should not be scored only as an isolated embedding vector. Its neighboring events — what happened right before and right after — may change its meaning, importance, or usefulness for the current query.

## Pipeline

```text
Ordered memory events (embedding vectors)
  → Deterministic projection (embedding_dim → latent_dim)
  → Local temporal convolution (window-based smoothing)
  → Cosine scoring against query
  → Top-k retrieval with readback explanation
```

## Safety invariants

- **No LLM calls** — all logic is deterministic arithmetic
- **No GPU dependency** — pure f64 operations
- **No persistent mutation** — all functions are pure
- **No authorization semantics** — every `RetrievalResponse` carries `non_authorizing: true`
- **No I/O** — the crate does not read/write files or call APIs

## Design

### Deterministic Projection

The projection matrix is generated using a fixed-seed LCG (Linear Congruential Generator) with parameters from Numerical Recipes. The same `(embedding_dim, latent_dim, seed)` always produces the same matrix. This ensures reproducibility without requiring the `rand` crate.

Each matrix entry is scaled to the range `[-1.0, 1.0]`.

### Projection (embedding → latent)

For each memory event:

```text
latent[j] = Σ_i embedding[i] × matrix[i][j]
```

The latent vector is then L2-normalized for proper cosine scoring. A zero embedding produces a zero latent vector (normalization is skipped).

### Convolution

For each event at position `i`, the output latent vector is the average of the latent vectors for positions `[max(0, i - half), min(len - 1, i + half)]` where `half = (window_size - 1) / 2`.

Edge behaviour: partial windows at the boundaries are automatically re-normalized so that output vectors are not attenuated.

### Scoring

Cosine similarity between the query latent vector and each convolved memory latent vector. Returns `[-1.0, 1.0]` where `1.0` = identical direction and `0.0` = orthogonal or zero vector.

### Retrieval

The full `retrieve(query, events, config)` function runs the entire pipeline and returns a `RetrievalResponse` with:

- sorted results by score descending
- top-k limit
- human-readable explanation for audit/readback
- `non_authorizing: true` invariant

## Public API

### Types

| Type | Description |
|------|-------------|
| `MemoryEvent` | A memory event with embedding vector, optional timestamp, optional label |
| `Config` | Configuration: embedding_dim, latent_dim, window_size, top_k, seed |
| `ProjectionMatrix` | Deterministic projection matrix (serializable) |
| `RetrievalResult` | Single result with id, score, rank, latent vector |
| `RetrievalResponse` | Full response with results, config, explanation, `non_authorizing` flag |

### Functions

| Function | Description |
|----------|-------------|
| `generate_projection_matrix(embedding_dim, latent_dim, seed)` | Deterministic matrix generation |
| `project(events, matrix)` | Project all events to latent space |
| `convolve(events, window_size)` | Apply local temporal convolution |
| `cosine_similarity(a, b)` | Cosine similarity between two vectors |
| `retrieve(query, events, config)` | Full retrieval pipeline |

## Test coverage

**50 tests** covering:

- Deterministic LCG: same seed → same output, different seeds → different output
- Projection matrix: dimensions, determinism, value range, 1x1 case
- Projection: dimension mismatch handling, determinism, L2 normalization, zero embedding
- Convolution: empty input, window=1 (no-op), window=3 smoothing, edge cases, large window
- Cosine similarity: identical, orthogonal, opposite, partial match, zero vector, dimension mismatch, empty
- Config validation: all error conditions + valid config
- Full retrieval pipeline: empty events, basic retrieval, top-k, config validation rejection, query dim mismatch, event dim mismatch, determinism, timestamp sorting, temporal effect (convolution pulls neighbors together), explanation format, single event, identical query/event
- Non-authorizing invariant: every response is `non_authorizing: true`
- Serialization round-trips: MemoryEvent, Config, RetrievalResponse

## Non-goals

This crate deliberately does **not** implement:

- LLM integration or model calls
- Graph Memory or persistent storage
- Holographic Memory integration
- Authorization or action approval
- GPU acceleration
- Neural network training
- Real-time streaming
- Distributed computation

## Integration notes

The crate is a pure Rust library with no external dependencies beyond `serde` (for serialization) and `chrono` (for timestamps on memory events). It is ready to be imported by other ARPAGONA crates for experimental recall enrichment, but has **no integration hooks yet** — those belong in a future Cycle Trace / Runtime milestone.

## Relationship to other memory layers

| Layer | Role | Status |
|-------|------|--------|
| Reservoir Echo | Short-term volatile cognitive continuity | Alpha |
| Graph Memory | Structured durable memory with facts, entities, relations | Experimental |
| Holographic Memory | Pattern resonance via distributed signatures | Alpha V0 |
| **Compressed Cognitive Attention** | **Temporally enriched recall** | **This crate (alpha)** |

## Files

| File | Purpose |
|------|---------|
| `crates/compressed-cognitive-attention/Cargo.toml` | Crate manifest |
| `crates/compressed-cognitive-attention/src/lib.rs` | All types, logic, and tests |
| `docs/compressed-cognitive-attention.md` | This document |
