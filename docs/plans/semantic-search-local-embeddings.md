# Semantic Search with Local Embeddings - Implementation Plan

## Overview

Implement semantic (vector) search for summaries using locally-generated embeddings, enabling similarity-based retrieval beyond keyword matching. This complements the existing FTS5 full-text search with true semantic understanding.

## Technology Choices

### Vector Database: LanceDB

**Selected:** [LanceDB](https://github.com/lancedb/lancedb) (crate: `lancedb`)

**Rationale:**
- Native Rust SDK (now at 1.0.0 as of Dec 2025)
- Embedded/serverless - no external database to manage
- Persistent storage with automatic versioning
- Supports vector similarity search + metadata filtering
- Uses Arrow for efficient columnar storage
- Zero-copy operations
- Supports hybrid search (vector + full-text)

**Alternatives Considered:**
- Qdrant (requires separate server process)
- SQLite with vector extensions (less mature)
- In-memory only (loses persistence)

### Embedding Generation: FastEmbed-rs

**Selected:** [FastEmbed-rs](https://github.com/Anush008/fastembed-rs) (crate: `fastembed`)

**Rationale:**
- Pure Rust with ONNX runtime (via `ort`)
- Pre-trained quantized models available (BGE, MiniLM, etc.)
- Fast CPU inference (no GPU required)
- Supports text embeddings, sparse embeddings, and reranking
- Models downloaded automatically on first use
- Memory efficient with quantized variants

**Alternatives Considered:**
- Candle (`candle_embed`) - More flexible but requires more setup
- Remote API embeddings - Adds latency and API costs
- sentence-transformers via PyO3 - Introduces Python dependency

### Recommended Embedding Model

**Primary:** `BGESmallENV15` (or quantized `BGESmallENV15Q`)
- 384 dimensions
- Excellent quality/speed tradeoff
- ~33MB model size (quantized: ~17MB)

**Alternative:** `AllMiniLML6V2` (quantized: `AllMiniLML6V2Q`)
- 384 dimensions
- Faster but slightly lower quality
- Good for resource-constrained environments

## Architecture Design

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        retrochat-core                                │
├─────────────────────────────────────────────────────────────────────┤
│  Services Layer                                                      │
│  ┌─────────────────────┐  ┌────────────────────────────────────────┐│
│  │ SemanticSearchService│  │ EmbeddingService                      ││
│  │ - search_summaries() │  │ - embed_text()                        ││
│  │ - hybrid_search()    │  │ - embed_batch()                       ││
│  │ - find_similar()     │  │ - model_info()                        ││
│  └──────────┬──────────┘  └──────────────────┬─────────────────────┘│
│             │                                 │                      │
│  ┌──────────▼──────────────────────────────────▼─────────────────────┤
│  │ VectorStore (Repository Layer)                                   ││
│  │ - LanceDB connection management                                  ││
│  │ - Table operations (turn_embeddings, session_embeddings)         ││
│  │ - Vector search with metadata filtering                          ││
│  └──────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  External Storage                                                    │
│  ┌──────────────────┐  ┌──────────────────────────────────────────┐ │
│  │ SQLite (existing)│  │ LanceDB                                  │ │
│  │ - turn_summaries │  │ - turn_embeddings table                  │ │
│  │ - session_summaries│ │ - session_embeddings table              │ │
│  │ - messages        │  │ - Stored in ~/.retrochat/vectors/       │ │
│  └──────────────────┘  └──────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. Summary Created/Updated
   turn_summaries table → EmbeddingService.embed_text() → VectorStore.upsert()
                                    ↓
                          FastEmbed model inference
                                    ↓
                          384-dim float vector
                                    ↓
                          LanceDB turn_embeddings table

2. Semantic Search Query
   User query → EmbeddingService.embed_text() → VectorStore.search()
                         ↓                              ↓
                   Query vector              LanceDB ANN search
                                                       ↓
                                            Top-K results with scores
                                                       ↓
                                            Join with SQLite summaries
```

## Database Schema

### LanceDB Tables

#### turn_embeddings
```
Schema:
- id: String (primary key, matches turn_summaries.id)
- session_id: String (for filtering)
- turn_number: Int32 (for filtering)
- turn_type: String (nullable, for filtering)
- embedding: FixedSizeList<Float32>[384] (vector column)
- text_hash: String (SHA256 of embedded text for change detection)
- embedded_at: Timestamp
- model_name: String (e.g., "BGESmallENV15")

Index: IVF_PQ on embedding column (created after sufficient data)
```

#### session_embeddings
```
Schema:
- id: String (primary key, matches session_summaries.id)
- session_id: String (unique, for joining)
- outcome: String (nullable, for filtering)
- embedding: FixedSizeList<Float32>[384] (vector column)
- text_hash: String
- embedded_at: Timestamp
- model_name: String

Index: IVF_PQ on embedding column
```

### Text to Embed

For optimal semantic search, combine relevant fields:

**Turn Summary Embedding Input:**
```
{user_intent}

{assistant_action}

{summary}

Topics: {key_topics.join(", ")}
```

**Session Summary Embedding Input:**
```
{title}

{summary}

Goal: {primary_goal}

Decisions: {key_decisions.join(", ")}
Technologies: {technologies_used.join(", ")}
```

## Module Structure

### New Files

```
crates/retrochat-core/src/
├── embedding/
│   ├── mod.rs              # Module exports
│   ├── service.rs          # EmbeddingService - text → vector
│   ├── config.rs           # Model configuration, cache paths
│   └── models.rs           # Supported model definitions
├── vector_store/
│   ├── mod.rs              # Module exports
│   ├── store.rs            # VectorStore - LanceDB operations
│   ├── schemas.rs          # LanceDB table schemas
│   └── search.rs           # Search query builders
└── services/
    └── semantic_search.rs  # SemanticSearchService - high-level API
```

### Updated Files

```
crates/retrochat-core/src/
├── lib.rs                  # Export new modules
├── services/
│   ├── mod.rs              # Export SemanticSearchService
│   ├── turn_summarization.rs    # Trigger embedding on summary create
│   └── session_summarization.rs # Trigger embedding on summary create
└── database/
    └── mod.rs              # Keep existing, add vector_store reference
```

## API Design

### EmbeddingService

```rust
pub struct EmbeddingService {
    model: TextEmbedding,
    config: EmbeddingConfig,
}

impl EmbeddingService {
    /// Create service with specified model
    pub async fn new(config: EmbeddingConfig) -> Result<Self>;

    /// Embed a single text
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed multiple texts (more efficient for batches)
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Get model info (name, dimensions)
    pub fn model_info(&self) -> ModelInfo;
}

pub struct EmbeddingConfig {
    pub model: EmbeddingModel,        // BGESmallENV15, AllMiniLML6V2, etc.
    pub cache_dir: Option<PathBuf>,   // Model cache location
    pub use_quantized: bool,          // Use quantized model variant
}
```

### VectorStore

```rust
pub struct VectorStore {
    db: Database,
    turn_table: Table,
    session_table: Table,
}

impl VectorStore {
    /// Open or create vector store
    pub async fn open(path: &Path) -> Result<Self>;

    /// Upsert turn embedding
    pub async fn upsert_turn_embedding(&self, embedding: TurnEmbedding) -> Result<()>;

    /// Upsert session embedding
    pub async fn upsert_session_embedding(&self, embedding: SessionEmbedding) -> Result<()>;

    /// Search turn embeddings
    pub async fn search_turns(
        &self,
        query_vector: &[f32],
        limit: usize,
        filter: Option<TurnFilter>,
    ) -> Result<Vec<TurnSearchResult>>;

    /// Search session embeddings
    pub async fn search_sessions(
        &self,
        query_vector: &[f32],
        limit: usize,
        filter: Option<SessionFilter>,
    ) -> Result<Vec<SessionSearchResult>>;

    /// Delete embeddings for a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()>;

    /// Get embedding status
    pub async fn get_stats(&self) -> Result<VectorStoreStats>;
}

pub struct TurnFilter {
    pub session_id: Option<String>,
    pub turn_types: Option<Vec<TurnType>>,
}

pub struct SessionFilter {
    pub outcomes: Option<Vec<SessionOutcome>>,
}

pub struct TurnSearchResult {
    pub id: String,
    pub session_id: String,
    pub turn_number: i32,
    pub score: f32,  // Cosine similarity (0.0 - 1.0)
}
```

### SemanticSearchService

```rust
pub struct SemanticSearchService {
    embedding_service: EmbeddingService,
    vector_store: VectorStore,
    summary_repo: TurnSummaryRepo,
    session_summary_repo: SessionSummaryRepo,
}

impl SemanticSearchService {
    /// Search turn summaries by semantic similarity
    pub async fn search_turns(
        &self,
        query: &str,
        limit: usize,
        filter: Option<TurnFilter>,
    ) -> Result<Vec<TurnSearchResult>>;

    /// Search session summaries by semantic similarity
    pub async fn search_sessions(
        &self,
        query: &str,
        limit: usize,
        filter: Option<SessionFilter>,
    ) -> Result<Vec<SessionSearchResult>>;

    /// Hybrid search combining semantic + FTS
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        semantic_weight: f32,  // 0.0-1.0, remainder goes to FTS
    ) -> Result<Vec<HybridSearchResult>>;

    /// Find similar turns to a given turn
    pub async fn find_similar_turns(
        &self,
        turn_id: &str,
        limit: usize,
    ) -> Result<Vec<TurnSearchResult>>;

    /// Index a turn summary (called after summarization)
    pub async fn index_turn(&self, turn: &TurnSummary) -> Result<()>;

    /// Index a session summary
    pub async fn index_session(&self, session: &SessionSummary) -> Result<()>;

    /// Rebuild entire index
    pub async fn rebuild_index(&self, progress: impl Fn(usize, usize)) -> Result<()>;
}
```

## Dependencies

### Cargo.toml Additions (workspace)

```toml
[workspace.dependencies]
# Vector database
lancedb = "0.22"
arrow = { version = "53", default-features = false }
arrow-array = "53"
arrow-schema = "53"

# Embedding
fastembed = "4"

# Hashing for change detection
sha2 = "0.10"
```

### retrochat-core/Cargo.toml

```toml
[dependencies]
lancedb = { workspace = true, optional = true }
arrow = { workspace = true, optional = true }
arrow-array = { workspace = true, optional = true }
arrow-schema = { workspace = true, optional = true }
fastembed = { workspace = true, optional = true }
sha2 = { workspace = true }

[features]
default = ["reqwest"]
semantic-search = ["lancedb", "arrow", "arrow-array", "arrow-schema", "fastembed"]
```

## CLI Integration

### New Commands

```bash
# Semantic search
cargo cli -- search --semantic "how to handle errors in async code"
cargo cli -- search --hybrid "authentication implementation" --semantic-weight 0.7

# Index management
cargo cli -- index status           # Show indexing status
cargo cli -- index rebuild          # Rebuild entire vector index
cargo cli -- index rebuild --session SESSION_ID  # Rebuild specific session
```

### Updated Commands

```bash
# Enhanced search with semantic option
cargo cli -- search "query" [--semantic] [--hybrid] [--limit N]
```

## Implementation Phases

### Phase 1: Foundation (Core Infrastructure)
1. Add dependencies to workspace Cargo.toml
2. Create `embedding/` module with EmbeddingService
3. Create `vector_store/` module with VectorStore
4. Add basic tests for embedding generation
5. Add basic tests for vector storage

### Phase 2: Integration (Connect to Existing Code)
6. Create SemanticSearchService
7. Integrate with turn/session summarization services
8. Add embedding generation after summary creation
9. Implement change detection (skip re-embedding unchanged summaries)

### Phase 3: Search API (User-Facing Features)
10. Implement semantic search endpoints
11. Implement hybrid search (semantic + FTS)
12. Add find-similar functionality
13. Implement CLI commands for semantic search

### Phase 4: Index Management
14. Implement index rebuild command
15. Add index status/statistics
16. Implement incremental indexing
17. Add background indexing option

### Phase 5: Optimization (Performance Tuning)
18. Create IVF_PQ index after sufficient data
19. Tune search parameters
20. Add caching for frequent queries
21. Benchmark and optimize batch operations

## Configuration

### Environment Variables

```bash
# Model selection
RETROCHAT_EMBEDDING_MODEL=BGESmallENV15  # or AllMiniLML6V2
RETROCHAT_EMBEDDING_QUANTIZED=true       # Use quantized model

# Storage paths
RETROCHAT_VECTOR_STORE_PATH=~/.retrochat/vectors/

# Search defaults
RETROCHAT_SEMANTIC_SEARCH_LIMIT=10
RETROCHAT_HYBRID_SEARCH_WEIGHT=0.7       # Semantic weight in hybrid search
```

### Config File Support

```toml
# ~/.retrochat/config.toml
[semantic_search]
enabled = true
model = "BGESmallENV15"
quantized = true
auto_index = true  # Index summaries automatically

[hybrid_search]
default_weight = 0.7  # Semantic weight
```

## Migration Strategy

### Existing Data

For users with existing summaries:

1. **On first run with semantic-search enabled:**
   - Detect unindexed summaries
   - Prompt user: "Found N summaries without embeddings. Index now? (Y/n)"
   - Or auto-index in background

2. **Incremental indexing:**
   - Track `text_hash` to detect changed summaries
   - Only re-embed when summary content changes

### Model Changes

If user changes embedding model:
- Detect model mismatch in stored embeddings
- Warn user and offer full reindex
- Keep model name in embedding metadata

## Testing Strategy

### Unit Tests
- EmbeddingService: Text embedding generation
- VectorStore: CRUD operations, search queries
- Hash calculation: Change detection

### Integration Tests
- End-to-end semantic search flow
- Hybrid search ranking
- Index rebuild process

### Performance Tests
- Embedding throughput (texts/second)
- Search latency at various index sizes
- Memory usage during batch operations

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Large model download on first run | Show progress, cache in standard location |
| Slow embedding on CPU | Use quantized models, batch processing |
| LanceDB version compatibility | Pin to stable version, test upgrades |
| Memory pressure from large indexes | Configure vector store cache limits |
| Model accuracy for code-heavy content | Test with representative queries, consider code-specific models |

## Success Metrics

1. **Search Quality**: Semantic search returns more relevant results than FTS alone for conceptual queries
2. **Performance**: Embedding generation < 100ms per summary, Search < 50ms for 10K summaries
3. **User Experience**: Seamless integration with existing CLI/TUI
4. **Maintainability**: Clean separation of concerns, well-tested code

## Future Enhancements

1. **Multi-modal embeddings**: Embed code snippets separately
2. **Sparse + Dense hybrid**: Use SPLADE for keyword-aware vectors
3. **Query expansion**: Use LLM to expand search queries
4. **Clustering**: Automatic topic clustering of summaries
5. **Recommendations**: "Sessions similar to this one"

## References

- [LanceDB Rust SDK](https://docs.rs/lancedb/latest/lancedb/)
- [LanceDB GitHub](https://github.com/lancedb/lancedb)
- [FastEmbed-rs GitHub](https://github.com/Anush008/fastembed-rs)
- [FastEmbed-rs Documentation](https://docs.rs/fastembed)
- [Candle ML Framework](https://github.com/huggingface/candle)
- [BGE Embedding Models](https://huggingface.co/BAAI/bge-small-en-v1.5)
