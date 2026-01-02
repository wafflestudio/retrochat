# GitHub Issue: Semantic Search with Local Embeddings

**Title:** `feat: Semantic Search with Local Embeddings (LanceDB + FastEmbed-rs)`

**Labels:** `enhancement`, `feature`

---

## Overview

Implement semantic (vector) search for summaries using locally-generated embeddings, enabling similarity-based retrieval beyond keyword matching. This complements the existing FTS5 full-text search with true semantic understanding.

## Technology Stack

### Vector Database: LanceDB
- **Crate:** [`lancedb`](https://crates.io/crates/lancedb) (v0.22+, SDK 1.0.0)
- Native Rust, embedded/serverless - no external database to manage
- Persistent storage with automatic versioning
- Supports vector similarity search + metadata filtering via SQL (DataFusion)
- Zero-copy Arrow-based storage

### Embedding Generation: FastEmbed-rs
- **Crate:** [`fastembed`](https://crates.io/crates/fastembed) (v4+)
- Pure Rust with ONNX runtime
- Pre-trained quantized models (BGE, MiniLM)
- Fast CPU inference, no GPU required
- Models auto-downloaded on first use (~17MB quantized)

### Recommended Model
- **Primary:** `BGESmallENV15Q` (384 dimensions, ~17MB)
- **Alternative:** `AllMiniLML6V2Q` (faster, slightly lower quality)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        retrochat-core                           │
├─────────────────────────────────────────────────────────────────┤
│  Services Layer                                                 │
│  ┌─────────────────────┐  ┌──────────────────────────────────┐ │
│  │SemanticSearchService│  │ EmbeddingService                 │ │
│  │ - search_summaries()│  │ - embed_text()                   │ │
│  │ - hybrid_search()   │  │ - embed_batch()                  │ │
│  │ - find_similar()    │  │ - model_info()                   │ │
│  └──────────┬──────────┘  └──────────────────┬───────────────┘ │
│             │                                 │                 │
│  ┌──────────▼─────────────────────────────────▼───────────────┐ │
│  │ VectorStore (Repository Layer)                             │ │
│  │ - LanceDB connection management                            │ │
│  │ - Table operations (turn_embeddings, session_embeddings)   │ │
│  │ - Vector search with metadata filtering                    │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  External Storage                                               │
│  ┌──────────────────┐  ┌────────────────────────────────────┐  │
│  │ SQLite (existing)│  │ LanceDB                            │  │
│  │ - turn_summaries │  │ - turn_embeddings table            │  │
│  │ - session_summaries│ │ - session_embeddings table        │  │
│  │ - messages       │  │ - Stored in ~/.retrochat/vectors/  │  │
│  └──────────────────┘  └────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## LanceDB Schema

### turn_embeddings
| Field | Type | Description |
|-------|------|-------------|
| id | String | Primary key (matches turn_summaries.id) |
| session_id | String | For filtering |
| turn_number | Int32 | For filtering |
| turn_type | String | task, question, error_fix, etc. |
| started_at | Timestamp | Turn start time (time-range queries) |
| ended_at | Timestamp | Turn end time |
| embedding | FixedSizeList<Float32>[384] | Vector column |
| text_hash | String | SHA256 for change detection |
| embedded_at | Timestamp | When embedding was generated |
| model_name | String | e.g., "BGESmallENV15" |

**Indexes:** IVF_PQ on embedding, scalar indexes on started_at, turn_type, session_id

### session_embeddings
| Field | Type | Description |
|-------|------|-------------|
| id | String | Primary key |
| session_id | String | Unique, for joining |
| outcome | String | completed, partial, abandoned, ongoing |
| created_at | Timestamp | Session creation time |
| updated_at | Timestamp | Last message time |
| provider | String | claude, gemini, chatgpt |
| project | String | Project name |
| embedding | FixedSizeList<Float32>[384] | Vector column |
| text_hash | String | SHA256 for change detection |
| embedded_at | Timestamp | When embedding was generated |
| model_name | String | Model used |

**Indexes:** IVF_PQ on embedding, scalar indexes on created_at, provider, outcome

## Metadata Filtering

LanceDB supports full SQL WHERE clause via DataFusion:

```rust
// Time range query
table.query().nearest_to(&query_vec)?
    .only_if("started_at >= timestamp '2025-01-01' AND started_at < timestamp '2025-02-01'")
    .limit(10)
    .execute().await?;

// Combined filters
table.query().nearest_to(&query_vec)?
    .only_if("provider = 'claude' AND turn_type IN ('task', 'error_fix')")
    .limit(20)
    .execute().await?;
```

## CLI Commands

```bash
# Semantic search
retrochat search --semantic "how to handle errors in async code"
retrochat search --hybrid "authentication" --semantic-weight 0.7

# With metadata filters
retrochat search --semantic "migrations" --from 2025-01-01 --to 2025-06-30
retrochat search --semantic "error handling" --provider claude --turn-type error_fix

# Index management
retrochat index status
retrochat index rebuild [--session SESSION_ID]
```

## Dependencies

```toml
[workspace.dependencies]
lancedb = "0.22"
arrow = { version = "53", default-features = false }
arrow-array = "53"
arrow-schema = "53"
fastembed = "4"
sha2 = "0.10"

[features]
semantic-search = ["lancedb", "arrow", "arrow-array", "arrow-schema", "fastembed"]
```

## Implementation Phases

### Phase 1: Foundation
- [ ] Add dependencies to workspace Cargo.toml
- [ ] Create `embedding/` module with EmbeddingService
- [ ] Create `vector_store/` module with VectorStore
- [ ] Add basic tests for embedding generation
- [ ] Add basic tests for vector storage

### Phase 2: Integration
- [ ] Create SemanticSearchService
- [ ] Integrate with turn/session summarization services
- [ ] Add embedding generation after summary creation
- [ ] Implement change detection (skip re-embedding unchanged summaries)

### Phase 3: Search API
- [ ] Implement semantic search endpoints
- [ ] Implement hybrid search (semantic + FTS)
- [ ] Add find-similar functionality
- [ ] Implement CLI commands for semantic search

### Phase 4: Index Management
- [ ] Implement index rebuild command
- [ ] Add index status/statistics
- [ ] Implement incremental indexing
- [ ] Add background indexing option

### Phase 5: Optimization
- [ ] Create IVF_PQ index after sufficient data
- [ ] Tune search parameters
- [ ] Add caching for frequent queries
- [ ] Benchmark and optimize batch operations

## Deployment

### Single Binary Support
- **LanceDB:** ✓ Fully embedded, no external server
- **FastEmbed-rs:** Runtime model download (default) or compile-time embedding

| Strategy | Binary Size | Network Required |
|----------|-------------|------------------|
| Runtime download (recommended) | ~10MB | First-run only |
| Embedded models | ~60MB+ | None |

## Success Metrics

1. Semantic search returns more relevant results than FTS alone for conceptual queries
2. Embedding generation < 100ms per summary
3. Search latency < 50ms for 10K summaries
4. Seamless integration with existing CLI/TUI

## References

- [LanceDB Rust SDK](https://docs.rs/lancedb/latest/lancedb/)
- [FastEmbed-rs](https://github.com/Anush008/fastembed-rs)
- [Plan Document](docs/plans/semantic-search-local-embeddings.md)
