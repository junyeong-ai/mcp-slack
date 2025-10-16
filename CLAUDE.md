# Slack MCP Server - Development Guide

AI assistant documentation for the Slack MCP Server project.

## Quick Start

**What**: Rust-based MCP server for Slack integration with SQLite caching, FTS5 search, and distributed locking.
**Stack**: Rust 2024 (requires 1.90+), Tokio async, SQLite with WAL, r2d2 pooling
**Tests**: `cargo test` (147 tests)
**Build**: `cargo build --release`

### Project Structure
```
src/
├── main.rs                  # Entry point: Tokio runtime, config, server init
├── config.rs                # Configuration with defaults
├── error.rs                 # McpError enum for MCP protocol errors
├── utils.rs                 # Channel resolution, parameter validation
├── mcp/                     # MCP protocol layer
│   ├── server.rs           # JSON-RPC stdio server
│   ├── handlers.rs         # Tool registration and routing
│   └── types.rs            # Protocol types
├── slack/                   # Slack API client
│   ├── client.rs           # Unified facade (messages, users, channels)
│   ├── core.rs             # HTTP client with rate limiting
│   ├── users.rs            # fetch_all_users
│   ├── channels.rs         # fetch_all_channels
│   ├── messages.rs         # send_message, get_channel_messages, get_thread_messages
│   ├── api_config.rs       # Per-method rate limit configs
│   └── types.rs            # Slack data models
├── cache/                   # SQLite cache with FTS5
│   ├── mod.rs              # Module exports, CacheRefreshType
│   ├── sqlite_cache.rs     # Main implementation, Pool management
│   ├── schema.rs           # Table/index/trigger definitions
│   ├── error.rs            # CacheError enum with typed errors
│   ├── users.rs            # save_users(), get_users(), search_users()
│   ├── channels.rs         # save_channels(), get_channels(), search_channels()
│   ├── locks.rs            # Distributed locking: acquire_lock(), with_lock()
│   └── helpers.rs          # process_fts_query(), is_cache_stale(), get_counts()
└── tools/                   # MCP tools (8 total)
    ├── search.rs           # SearchUsersTool, SearchChannelsTool, SearchMessagesTool
    ├── messages.rs         # SendMessageTool, GetChannelMessagesTool, ReadThreadTool, ListChannelMembersTool
    ├── cache.rs            # RefreshCacheTool
    ├── message_utils.rs    # format_message(), format_thread_messages()
    └── response.rs         # ToolResponse, IntoToolResponse
```

## Architecture

### Core Design Principles

1. **Cache-First**: All user/channel data cached in SQLite with FTS5 for fast search
2. **Atomic Updates**: Distributed locks + transactions for zero-downtime cache refresh
3. **Type Safety**: Custom error enums (CacheError, McpError) for precise error handling
4. **Performance**: Async I/O, connection pooling, no unnecessary clones or allocations
5. **Token Efficiency**: Minimal JSON payloads (no blocks/attachments in responses)

### Data Flow

```
MCP Client → JSON-RPC stdio → RequestHandler → Tool
                                                  ↓
                                          SqliteCache (FTS5 search)
                                                  ↓
                                          SlackClient (if cache miss/write)
```

### Cache Strategy

**Storage**: `~/.mcp-slack/cache.db` (SQLite WAL mode)
**Tables**: users, channels, locks, metadata, users_fts, channels_fts
**TTL**: 24h for users/channels, 12h for members
**Refresh**: Atomic swap pattern with distributed locks

**Atomic Swap Process**:
1. Acquire lock with retries + exponential backoff
2. Create temp table, insert new data from Slack API
3. Transaction: DELETE old → INSERT from temp → UPDATE metadata
4. Release lock

**Why**: Zero downtime, no partial reads, safe for multi-instance deployment

## Key Patterns

### Error Handling

Two error types for different layers:

**CacheError** (`src/cache/error.rs`) - Internal cache operations:
```rust
pub enum CacheError {
    ConnectionPoolError(r2d2::Error),
    DatabaseError(rusqlite::Error),
    SerializationError(serde_json::Error),
    LockAcquisitionFailed { key: String, attempts: usize },
    SystemTimeError(std::time::SystemTimeError),
    InvalidQuery(String),
    InvalidInput(String),
}
pub type CacheResult<T> = Result<T, CacheError>;
```

**McpError** (`src/error.rs`) - MCP protocol/API errors:
```rust
pub enum McpError {
    NotFound(String),
    InvalidParameter(String),
    Internal(String),
    Unauthorized(String),
    RateLimited(String),
    SlackApi(String),
}
pub type McpResult<T> = Result<T, McpError>;
```

**Conversion**: CacheError → McpError via `.mcp_context()` trait extension

### Async Patterns

**Rule**: Only use `async` for functions that actually await
- Cache read operations (get_users, search_users): **NOT async** (no I/O, just SQLite query)
- Cache write operations (save_users): **async** (uses distributed lock with retries/sleep)
- Slack API calls: **async** (network I/O)

### Connection Management

**Pool**: `Pool<SqliteConnectionManager>` (NOT Arc-wrapped - Pool itself is Clone)
```rust
pub struct SqliteCache {
    pub(super) pool: Pool<SqliteConnectionManager>,  // Clone creates new reference
    pub(super) instance_id: String,                   // For distributed lock ownership
}
```

**Usage**: `let conn = self.pool.get()?` - blocks until connection available

### FTS5 Search

**Query Sanitization** (`src/cache/helpers.rs`):
```rust
pub fn process_fts_query(&self, query: &str) -> String {
    // Escapes special chars, wraps in quotes for phrase search
    // Empty/wildcard-only → returns empty string → fallback to LIKE
}
```

**Search Flow** (2-phase strategy):
1. Phase 1: LIKE substring match with exact match priority (0-3)
2. Phase 2: FTS5 fuzzy match (only if Phase 1 returns no results)
3. Return sorted results

### Distributed Locking

**Purpose**: Prevent concurrent cache updates across multiple instances
**Implementation**: SQLite table with instance_id + expiry timestamp
**Pattern**:
```rust
self.with_lock("key", || {
    // Critical section - guaranteed single writer
    // Automatic lock release even on error
    Ok(result)
}).await
```

**Stale Lock Detection**: Auto-cleanup locks older than 2x timeout

## Module Guide

### src/cache/

**sqlite_cache.rs**: Main cache struct with Pool
- `new(path)` - Initialize DB, create pool (max 10 connections), set up WAL
- Pool shared across clones via interior Arc (r2d2 design)

**error.rs**: Typed cache errors
- Automatic From implementations for common error types
- No anyhow dependency - pure typed errors

**users.rs / channels.rs**: Entity operations
- `save_*()` - Atomic swap with distributed lock (async)
- `get_*()` - Simple SELECT (NOT async)
- `search_*()` - 2-phase: LIKE substring → FTS5 fuzzy (NOT async)

**locks.rs**: Distributed locking
- `acquire_lock()` - 3 retries with exponential backoff
- `release_lock()` - DELETE by key + instance_id
- `with_lock()` - RAII pattern, always releases

**helpers.rs**: Utilities
- `process_fts_query()` - Sanitize FTS5 input
- `is_cache_stale()` - Check last_sync timestamps vs TTL
- `get_counts()` - Quick stats

**schema.rs**: SQL definitions
- Generated columns for JSON extraction
- FTS5 virtual tables with triggers
- Indexes for common queries

### src/slack/

**client.rs**: Unified interface
```rust
pub struct SlackClient {
    pub messages: SlackMessageClient,
    pub users: SlackUserClient,
    pub channels: SlackChannelClient,
}
```

**core.rs**: HTTP with rate limiting
- `governor` crate for token bucket
- Per-method rate limits from api_config.rs
- Automatic retry with exponential backoff

**users.rs / channels.rs**: API methods
- Pagination handling (cursor-based)
- Batch requests for efficiency

**messages.rs**: Message operations
- `send_message()` - chat.postMessage
- `get_channel_messages()` - conversations.history with pagination
- `get_thread_messages()` - conversations.replies

### src/tools/

**Tool Pattern**:
```rust
#[async_trait]
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, params: Value) -> McpResult<Value>;
}
```

**message_utils.rs**: Token-efficient formatting
- `format_message()` - Excludes blocks/attachments, resolves user names
- `format_thread_messages()` - Parent info only once
- `remove_empty_strings()` - Minimize JSON size

**response.rs**: Standardized responses
- `ToolResponse` struct with optional pagination metadata
- `IntoToolResponse` trait for easy conversion

### src/mcp/

**handlers.rs**: Tool registration
- HashMap of tool name → Box<dyn Tool>
- Cache initialization check → background refresh if stale
- Request routing by tool name

**server.rs**: JSON-RPC stdio
- Read/write MCP protocol messages
- Error serialization

## Development

### Testing
```bash
cargo test                              # All 147 tests
cargo test cache::users::tests          # Specific module
cargo test -- --nocapture               # Show println! output
RUST_LOG=debug cargo test              # With logging
```

### Linting
```bash
cargo fmt                               # Format code
cargo clippy --all-targets -- -D warnings  # Lint (treat warnings as errors)
cargo check                             # Fast compilation check
```

### Debugging Cache
```bash
sqlite3 ~/.mcp-slack/cache.db ".tables"
sqlite3 ~/.mcp-slack/cache.db "SELECT COUNT(*) FROM users;"
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM locks;"
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM metadata;"

# Test FTS5
sqlite3 ~/.mcp-slack/cache.db "
  SELECT u.name, u.email
  FROM users u
  JOIN users_fts f ON u.rowid = f.rowid
  WHERE users_fts MATCH 'john'
  LIMIT 5;
"
```

### Logging
```bash
RUST_LOG=mcp_slack=debug cargo run              # All modules
RUST_LOG=mcp_slack::cache=trace cargo run       # Cache only
RUST_LOG=mcp_slack::slack::core=debug cargo run # HTTP client
```

**Log Points**:
- Cache initialization and refresh
- Lock acquisition/release with instance_id
- API rate limiting (429 responses)
- FTS5 fallback triggers
- Transaction timing

### Common Tasks

**Add New Tool**:
1. Implement Tool trait in `src/tools/`
2. Register in `src/mcp/handlers.rs` RequestHandler::new()
3. Add tests
4. Document in README.md

**Add Cache Field**:
1. Update SlackUser/SlackChannel in `src/slack/types.rs`
2. Add generated column in `src/cache/schema.rs`
3. Update FTS5 virtual table if searchable
4. Increment SCHEMA_VERSION

**Add Slack API Method**:
1. Add method to appropriate client in `src/slack/`
2. Add rate limit config in `src/slack/api_config.rs`
3. Add response type in `src/slack/types.rs`

## Configuration

### Environment Variables
```bash
export SLACK_BOT_TOKEN="xoxb-..."      # Required
export SLACK_USER_TOKEN="xoxp-..."    # Optional, for message search
export DATA_PATH="~/.mcp-slack"        # Optional, default shown
export LOG_LEVEL="info"                # Optional: error/warn/info/debug/trace
```

### config.toml (optional)
```toml
[slack]
bot_token = "xoxb-..."
user_token = "xoxp-..."

[cache]
ttl_users_hours = 24
ttl_channels_hours = 24
data_path = "~/.mcp-slack"

[retry]
max_attempts = 3
initial_delay_ms = 1000
max_delay_ms = 60000

[connection]
timeout_seconds = 30
max_idle_per_host = 10
```

## Reference

### Key Dependencies
- **Tokio 1.47**: Async runtime
- **rusqlite 0.32**: SQLite with FTS5, bundled
- **r2d2 0.8**: Generic connection pool
- **r2d2_sqlite 0.25**: SQLite adapter for r2d2
- **reqwest 0.12**: HTTP client with rustls
- **governor 0.8**: Rate limiting
- **serde_json 1.0**: JSON serialization
- **tracing 0.1**: Structured logging

### MCP Tools (8)

| Tool | Description | Uses Cache | Uses API |
|------|-------------|-----------|----------|
| search_users | Search workspace users | ✓ FTS5 | - |
| search_channels | Search channels | ✓ FTS5 | - |
| search_messages | Search message content | - | ✓ (user token) |
| send_message | Send message to channel/DM | - | ✓ |
| get_channel_messages | Retrieve channel history | - | ✓ |
| read_thread | Get thread replies | - | ✓ |
| list_channel_members | List channel members | - | ✓ |
| refresh_cache | Force cache refresh | ✓ | ✓ |

### SQLite Schema Highlights

**users table**:
- JSON storage with generated columns (name, email, display_name, real_name, is_bot)
- FTS5 virtual table for full-text search
- Triggers keep FTS5 in sync

**channels table**:
- Similar structure with name, topic, purpose
- Flags: is_private, is_im, is_mpim, is_archived, is_channel
- FTS5 for channel search

**locks table**:
- Distributed locking: key, instance_id, acquired_at, expires_at
- Auto-cleanup of expired locks

**metadata table**:
- Key-value store: last_user_sync, last_channel_sync, schema_version

### Performance Characteristics

**Cache Read Operations**: O(log n) with SQLite B-tree indexes
**FTS5 Search**: O(k log n) where k = result count
**Cache Write**: O(n) - atomic swap of entire table
**Lock Contention**: Max 3 retries with exponential backoff (500ms → 1s → 1s)
**Token Usage**: ~135 tokens/message, ~20 tokens/user

### Common Issues

**"Database is locked"**: Rare with WAL mode. Check for stale locks.
**FTS5 syntax error**: Special chars not sanitized. Use `process_fts_query()`.
**User names as IDs**: Cache not populated. Run refresh_cache tool.
**Lock acquisition fails**: High concurrency. Increase MAX_RETRIES or reduce LOCK_TIMEOUT.

---

**Version**: 0.1.0 (Rust 2024 edition, requires 1.90+)
**Tests**: 147 passing
**Documentation**: See README.md for usage, DEVELOPMENT.md for contributing
