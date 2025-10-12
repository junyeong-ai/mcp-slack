# Claude Assistant Instructions for Slack MCP Server

This document provides comprehensive technical documentation for AI assistants working on the Slack MCP Server project.

## Project Overview

Production-ready Rust-based MCP (Model Context Protocol) server for Slack integration using SQLite for caching with FTS5 full-text search, distributed locking, and intelligent message formatting. Built with Rust 2024 edition (requires 1.90+), emphasizing safety, concurrency, and maintainability.

## Core Architecture

### Storage Layer - SQLite
- **Database**: Single `cache.db` file at `~/.mcp-slack/`
- **Mode**: WAL (Write-Ahead Logging) for concurrent access
- **Connection Pooling**: r2d2 (configurable, default 10 max connections)
- **Tables**:
  - `users` - User data with generated columns for indexing
  - `channels` - Channel data with type flags (public/private/DM/multi-DM)
  - `locks` - Distributed locking mechanism
  - `metadata` - Sync timestamps and cache metadata
  - `users_fts` - FTS5 virtual table for user search
  - `channels_fts` - FTS5 virtual table for channel search

### Caching Strategy
- **Auto-initialization**: On startup if cache is empty or stale
- **TTL-based refresh**: Configurable per resource type (default: 24h users/channels, 48h members)
- **Atomic updates**: Using temporary tables and transactions
- **Lock management**: Timeout with stale lock detection
- **Background refresh**: Spawned as async task, doesn't block startup

## Component Architecture

### 1. MCP Protocol Layer (`src/mcp/`)
- **server.rs**: Stdio-based JSON-RPC server
- **handlers.rs**: Request routing with auto-cache initialization
  - Registers 8 tools at startup
  - Spawns background cache refresh if empty/stale
  - Converts tool results to MCP format
- **types.rs**: MCP protocol type definitions (Tool, Property, ToolContent, etc.)

### 2. Slack Client (`src/slack/`)
- **client.rs**: Unified client facade aggregating sub-clients
- **core.rs**: Core HTTP API client with rate limiting
  - Token bucket implementation (governor crate)
  - Exponential backoff with jitter
  - Automatic retry on 429 responses
  - Respects Retry-After header
  - Default: 20 req/min (configurable)
- **users.rs**: User operations client (`fetch_all_users`)
- **channels.rs**: Channel operations client (`fetch_all_channels`)
- **messages.rs**: Message operations client
  - `send_message`
  - `get_channel_messages` (with pagination)
  - `get_thread_messages`
- **api_config.rs**: API method configurations (GET/POST/Form-encoded)
- **types.rs**: Slack data models (User, Channel, Message, etc.)

### 3. Cache System (`src/cache/`)

**Modular structure:**

- **sqlite_cache.rs**: Main cache implementation
  - Public `SqliteCache` struct
  - Connection pool management
  - Delegates to specialized modules

- **schema.rs**: Table definitions and initialization
  - CREATE TABLE statements
  - Generated columns for JSON extraction
  - FTS5 virtual table creation
  - Triggers for FTS5 sync

- **users.rs**: User caching operations
  - `save_users()` - Atomic user cache update
  - `search_users()` - FTS5 search with fuzzy fallback
  - `get_user()` - Single user lookup

- **channels.rs**: Channel caching operations
  - `save_channels()` - Atomic channel cache update
  - `search_channels()` - FTS5 search
  - `get_channel()` - Single channel lookup

- **locks.rs**: Distributed locking mechanism
  - `acquire_lock()` - Acquire with timeout
  - `release_lock()` - Release lock
  - Automatic stale lock cleanup
  - Exponential backoff on contention

- **helpers.rs**: Utility functions
  - `process_fts_query()` - FTS5 query sanitization
  - `atomic_swap()` - Temp table pattern for updates
  - Connection pool helpers

- **mod.rs**: Public types and enums
  - `RefreshScope` enum (Users, Channels, All)
  - Cache trait definition

### 4. Tools (`src/tools/`)

**Currently implemented tools:**

- **search.rs**: Search tools (3 tools)
  - `SearchUsersTool` - FTS5 user search with fuzzy fallback
  - `SearchChannelsTool` - FTS5 channel search
  - `SearchMessagesTool` - Slack API message search (requires user token)

- **messages.rs**: Message operations (4 tools)
  - `SendMessageTool` - Send to channels/DMs/threads
  - `GetChannelMessagesTool` - Read channel history with pagination
  - `ReadThreadTool` - Get thread with optimized format
  - `ListChannelMembersTool` - Get channel members with user details

- **cache.rs**: Cache management (1 tool)
  - `RefreshCacheTool` - Manual cache refresh with scope control

- **message_utils.rs**: Common formatting utilities
  - `format_message()` - Enrich message with user name
  - `format_thread_messages()` - Optimized thread format (parent info once)
  - `resolve_channel_identifier()` - Channel name/ID resolution
  - Handles empty display_name by falling back to real_name then username

- **response.rs**: Tool response formatting
  - `ToolResponse` trait
  - `IntoToolResponse` trait
  - Standardized error/success responses

**Total: 8 MCP tools implemented**

### 5. Configuration (`src/config.rs`)
- Default values for TTLs, retry logic, rate limits
- Environment variable loading (dotenv support)
- Config file parsing (TOML format)
- Validation and error handling

### 6. Error Handling (`src/error.rs`)
- Custom `McpError` enum with variants:
  - `NotFound` - Resource not found
  - `InvalidParameter` - Bad input
  - `Internal` - Server errors
  - `Unauthorized` - Auth failures
  - `RateLimited` - API rate limit hit
  - `SlackApi` - Slack API errors
- `McpResult<T>` type alias
- All errors propagate with context via `?` operator
- Implements `From` traits for error conversion

### 7. Main Entry Point (`src/main.rs`)
- Tokio async runtime setup
- Config loading (env + file)
- Logging initialization (tracing-subscriber)
- SlackClient creation
- SqliteCache initialization
- MCP server startup
- Graceful shutdown handling

## Key Implementation Details

### SQLite Schema

```sql
-- Users table with generated columns
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    data JSON NOT NULL,
    name TEXT GENERATED ALWAYS AS (json_extract(data, '$.name')) STORED,
    display_name TEXT GENERATED ALWAYS AS (json_extract(data, '$.profile.display_name')) STORED,
    real_name TEXT GENERATED ALWAYS AS (json_extract(data, '$.profile.real_name')) STORED,
    email TEXT GENERATED ALWAYS AS (json_extract(data, '$.profile.email')) STORED,
    is_bot INTEGER GENERATED ALWAYS AS (json_extract(data, '$.is_bot')) STORED,
    updated_at INTEGER DEFAULT (unixepoch())
);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS users_fts USING fts5(
    id UNINDEXED,
    name,
    display_name,
    real_name,
    email,
    content=users,
    content_rowid=rowid,
    tokenize='porter unicode61'
);

-- Triggers to keep FTS5 in sync
CREATE TRIGGER IF NOT EXISTS users_ai AFTER INSERT ON users BEGIN
    INSERT INTO users_fts(rowid, id, name, display_name, real_name, email)
    VALUES (new.rowid, new.id, new.name, new.display_name, new.real_name, new.email);
END;

CREATE TRIGGER IF NOT EXISTS users_ad AFTER DELETE ON users BEGIN
    DELETE FROM users_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS users_au AFTER UPDATE ON users BEGIN
    UPDATE users_fts SET
        id = new.id,
        name = new.name,
        display_name = new.display_name,
        real_name = new.real_name,
        email = new.email
    WHERE rowid = new.rowid;
END;

-- Channels table
CREATE TABLE IF NOT EXISTS channels (
    id TEXT PRIMARY KEY,
    data JSON NOT NULL,
    name TEXT GENERATED ALWAYS AS (json_extract(data, '$.name')) STORED,
    is_private INTEGER GENERATED ALWAYS AS (json_extract(data, '$.is_private')) STORED,
    is_im INTEGER GENERATED ALWAYS AS (json_extract(data, '$.is_im')) STORED,
    is_mpim INTEGER GENERATED ALWAYS AS (json_extract(data, '$.is_mpim')) STORED,
    updated_at INTEGER DEFAULT (unixepoch())
);

-- Locks table for distributed locking
CREATE TABLE IF NOT EXISTS locks (
    resource_name TEXT PRIMARY KEY,
    holder_id TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Metadata for tracking cache freshness
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER DEFAULT (unixepoch())
);
```

### Cache Update Process (Atomic Swap Pattern)

1. **Acquire distributed lock** with exponential backoff
2. **Create temporary table** matching main table schema
3. **Insert new data** into temp table (from Slack API)
4. **Begin transaction**
5. **DELETE all** from main table
6. **INSERT** from temp table to main table
7. **Update metadata** timestamps
8. **Commit transaction** (atomic - all or nothing)
9. **Drop temporary table**
10. **Release lock**

This pattern ensures:
- Zero downtime (WAL mode allows reads during update)
- Atomicity (transaction ensures consistency)
- No partial updates
- Concurrent read safety

### Message Formatting

Messages are enriched with user names from cache:

```rust
// In message_utils.rs
pub async fn format_message(
    msg: SlackMessage,
    cache: &Arc<SqliteCache>,
    include_thread_info: bool
) -> Value {
    // 1. Extract user_id from message
    // 2. Look up user in cache
    // 3. Get display_name, fallback to real_name, then username
    // 4. Return JSON with user_id AND user_name fields
    // 5. Optionally include thread_ts if message is in thread
}

// Thread optimization - parent info only once
pub async fn format_thread_messages(
    messages: Vec<SlackMessage>,
    cache: &Arc<SqliteCache>
) -> Value {
    // 1. Extract parent message (first in list)
    // 2. Format parent with user name
    // 3. Return structure:
    //    {
    //      "thread_info": { parent details with user_name },
    //      "messages": [ replies without duplicate parent ]
    //    }
    // Benefits: Reduces redundancy, easier for AI to parse
}
```

### Rate Limiting

Token bucket implementation in `core.rs`:

```rust
// Governor crate with Tokio integration
// Default: 20 requests per minute (configurable)
// On 429 response:
//   1. Read Retry-After header (if present)
//   2. Exponential backoff with jitter
//   3. Retry with same request
// Max retries: 3 (configurable)
```

### FTS5 Search

```rust
// In users.rs
pub async fn search_users(&self, query: &str, limit: usize) -> McpResult<Vec<User>> {
    // 1. Sanitize query (remove special FTS5 chars)
    let safe_query = process_fts_query(query);

    // 2. Try FTS5 search first
    let sql = "SELECT data FROM users
               WHERE rowid IN (
                   SELECT rowid FROM users_fts
                   WHERE users_fts MATCH ?
               )
               AND is_bot = 0
               LIMIT ?";

    // 3. If FTS5 fails or returns empty, fallback to fuzzy
    if results.is_empty() {
        // Use fuzzy-matcher crate for substring matching
        // Searches name, display_name, real_name, email
    }

    // 4. Return results
}
```

### Error Handling Pattern

```rust
// In handlers.rs
pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, McpError> {
    // 1. Get tool from registry
    let tool = self.tools.get(name)
        .ok_or_else(|| McpError::NotFound(format!("Tool not found: {}", name)))?;

    // 2. Execute tool (may return McpError)
    let result = tool.execute(arguments).await?;

    // 3. Convert to MCP format
    let content = if let Some(text) = result.as_str() {
        vec![ToolContent::Text { text: text.to_string() }]
    } else {
        vec![ToolContent::Text { text: serde_json::to_string_pretty(&result)? }]
    };

    Ok(CallToolResult { content })
}
```

## Configuration

### Default Values (in `config.rs`)

```rust
// Cache TTLs
const DEFAULT_TTL_USERS_HOURS: u64 = 24;
const DEFAULT_TTL_CHANNELS_HOURS: u64 = 24;
const DEFAULT_TTL_MEMBERS_HOURS: u64 = 48;

// Retry configuration
const DEFAULT_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;
const DEFAULT_MAX_DELAY_MS: u64 = 32000;
const DEFAULT_EXPONENTIAL_BASE: f64 = 2.0;

// Connection
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

// Rate limiting
const DEFAULT_RATE_LIMIT_PER_MINUTE: u32 = 20;
```

### Environment Variables

- `SLACK_BOT_TOKEN` - Bot user OAuth token (xoxb-)
- `SLACK_USER_TOKEN` - User OAuth token (xoxp-) for message search
- `DATA_PATH` - Database location (default: ~/.mcp-slack)
- `LOG_LEVEL` - Logging level (error/warn/info/debug/trace)
- `RUST_LOG` - Module-specific logging (e.g., `mcp_slack::cache=debug`)

### Config File Format (config.toml)

```toml
[slack]
bot_token = "xoxb-..."
user_token = "xoxp-..."  # Optional, for message search

[cache]
ttl_users_hours = 24
ttl_channels_hours = 24
ttl_members_hours = 48
data_path = "~/.mcp-slack"

[retry]
max_attempts = 3
initial_delay_ms = 1000
max_delay_ms = 32000
exponential_base = 2.0

[rate_limit]
requests_per_minute = 20

[connection]
timeout_seconds = 30
max_connections = 10
```

## Development Guidelines

### Code Patterns

1. **Async Everything**: All I/O operations use async/await with Tokio
2. **Arc for Sharing**: Use `Arc<T>` for shared ownership across async tasks
3. **Error Propagation**: Use `?` operator and `McpResult<T>` type alias
4. **Connection Pooling**: r2d2 pool shared via Arc
5. **Atomic Operations**: All cache updates in transactions
6. **Modular Design**: Cache split into logical modules (users, channels, locks, etc.)

### File Organization

```
src/
├── main.rs              # Entry point, runtime setup
├── lib.rs               # Library exports
├── config.rs            # Config with defaults and env loading
├── error.rs             # Custom error types
├── utils.rs             # Shared utilities
│
├── mcp/                 # MCP Protocol Layer
│   ├── mod.rs
│   ├── server.rs       # JSON-RPC stdio server
│   ├── handlers.rs     # Tool registry and routing
│   └── types.rs        # MCP type definitions
│
├── slack/               # Slack API Client
│   ├── mod.rs
│   ├── client.rs       # Unified facade
│   ├── core.rs         # HTTP + rate limiting
│   ├── users.rs        # User operations
│   ├── channels.rs     # Channel operations
│   ├── messages.rs     # Message operations
│   ├── api_config.rs   # API method configs
│   └── types.rs        # Slack data models
│
├── cache/               # SQLite Cache (Modular)
│   ├── mod.rs          # Types and traits
│   ├── sqlite_cache.rs # Main implementation
│   ├── schema.rs       # Table definitions
│   ├── users.rs        # User cache operations
│   ├── channels.rs     # Channel cache operations
│   ├── locks.rs        # Distributed locking
│   └── helpers.rs      # Utilities
│
└── tools/               # MCP Tools (8 total)
    ├── mod.rs
    ├── search.rs       # 3 search tools
    ├── messages.rs     # 4 message tools
    ├── cache.rs        # 1 cache tool
    ├── message_utils.rs # Formatting utilities
    └── response.rs     # Response types
```

### Testing Commands

```bash
# Run all tests
cargo test

# Run specific test module
cargo test cache::users::tests

# Check compilation
cargo check

# Format code
cargo fmt

# Lint with clippy (strict mode)
cargo clippy --all-targets -- -D warnings

# Build optimized release
cargo build --release

# Run with debug logging
RUST_LOG=debug cargo run

# Run specific module with debug
RUST_LOG=mcp_slack::cache=debug,mcp_slack::slack=info cargo run
```

### Common Issues & Solutions

#### 1. "table has 3 columns but 2 values"
**Cause:** Schema mismatch between temp and main tables
**Fix:** Ensure temp tables include all columns with defaults, use same schema

#### 2. "database is locked"
**Cause:** Long-running transaction or missing WAL mode
**Fix:** WAL mode handles most cases; check for stale locks, ensure single writer

#### 3. FTS5 syntax errors
**Cause:** Special characters in search queries (quotes, operators)
**Fix:** `process_fts_query()` in helpers.rs sanitizes input

#### 4. Cache not refreshing
**Cause:** Empty cache or expired TTL not detected
**Fix:** Auto-refresh on startup handles this; check is_cache_stale() logic

#### 5. User names showing as IDs
**Cause:** Cache miss during message formatting
**Fix:** Ensure cache is populated; format_message() handles gracefully

## Technical Stack

### Core Dependencies

- **Language**: Rust 2024 edition (requires 1.90+)
- **Runtime**: Tokio 1.47 (async I/O)
- **Database**: SQLite with FTS5 (rusqlite 0.32)
- **HTTP Client**: reqwest 0.12 with rustls
- **Rate Limiting**: governor 0.8
- **Connection Pool**: r2d2 0.8 + r2d2_sqlite 0.25
- **Serialization**: serde 1.0 + serde_json 1.0
- **Error Handling**: anyhow 1.0 + thiserror 2.0
- **Fuzzy Matching**: fuzzy-matcher 0.3
- **Logging**: tracing 0.1 + tracing-subscriber 0.3

### Architecture Characteristics

**Caching:**
- SQLite WAL mode for concurrent reads
- FTS5 full-text search indexes
- TTL-based refresh strategy
- Atomic updates via transactions

**Concurrency:**
- Tokio async runtime
- r2d2 connection pooling
- Distributed locking mechanism
- Non-blocking I/O

**Reliability:**
- Exponential backoff on failures
- Automatic retry with jitter
- Rate limiting with token bucket
- Comprehensive error handling

## API Coverage

### Implemented Slack APIs

**Users:**
- `users.list` - Fetch all workspace users (paginated)

**Conversations:**
- `conversations.list` - Fetch all channels (all types)
- `conversations.history` - Get channel messages (paginated)
- `conversations.replies` - Get thread messages
- `conversations.members` - Get channel members

**Chat:**
- `chat.postMessage` - Send messages to channels/DMs/threads

**Search:**
- `search.messages` - Full workspace message search (requires user token)

### Tool-to-API Mapping

| MCP Tool | Slack API(s) Used | Token Required |
|----------|-------------------|----------------|
| `search_users` | Cache (SQLite FTS5) | Bot or User |
| `search_channels` | Cache (SQLite FTS5) | Bot or User |
| `search_messages` | search.messages | **User token** |
| `send_message` | chat.postMessage | Bot or User |
| `get_channel_messages` | conversations.history | Bot or User |
| `read_thread` | conversations.replies | Bot or User |
| `list_channel_members` | conversations.members | Bot or User |
| `refresh_cache` | users.list, conversations.list | Bot or User |

**Note:** Message search specifically requires a user token (`xoxp-`) with `search:read` scope. Bot tokens cannot search messages.

## Best Practices

### When Writing Code

1. **Cache First**: Always check cache before API calls
   - Search users/channels from cache (instant)
   - Only call Slack API when necessary (send message, live search)

2. **Batch Operations**: Use transactions for multiple updates
   - All cache writes in transactions
   - Atomic swap pattern for consistency

3. **Lock Responsibly**: Keep critical sections minimal
   - Acquire lock → quick operation → release immediately
   - Use try_lock with timeout, not infinite wait

4. **Log with Context**: Include operation details in log messages
   ```rust
   tracing::debug!(
       user_id = %user.id,
       action = "cache_save",
       "Saving user to cache"
   );
   ```

5. **Test Fallbacks**: Ensure FTS5 fallback to fuzzy works
   - Test with special characters
   - Test with partial matches
   - Test with empty results

6. **Handle Empty Strings**: Check for empty, not just None
   ```rust
   let name = user.display_name
       .filter(|s| !s.is_empty())
       .or(user.real_name.filter(|s| !s.is_empty()))
       .unwrap_or(&user.name);
   ```

7. **Thread Format**: Return parent info only once
   - Optimized format reduces token usage for AI
   - Easier for AI to parse thread structure

### When Debugging

1. **Enable Structured Logging**:
   ```bash
   RUST_LOG=mcp_slack=debug cargo run
   ```

2. **Check Cache State**:
   ```bash
   sqlite3 ~/.mcp-slack/cache.db "
     SELECT
       (SELECT COUNT(*) FROM users) as users,
       (SELECT COUNT(*) FROM channels) as channels,
       (SELECT COUNT(*) FROM users_fts) as fts_rows,
       (SELECT value FROM metadata WHERE key='last_users_sync') as last_sync
   "
   ```

3. **Monitor Lock Contention**:
   ```bash
   sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM locks;"
   ```

4. **Trace API Calls**: Look for rate limit warnings in logs

5. **Profile Performance**: Use `tracing` span timings
   ```rust
   let _span = tracing::debug_span!("operation_name").entered();
   // operation
   drop(_span); // Logs duration
   ```

## Monitoring & Debugging

### Key Log Points

- **Cache initialization**: On startup, logs cache status
- **Lock acquisition/release**: Debug logs with holder_id
- **API rate limit hits**: Warn when 429 received
- **FTS5 fallback triggers**: Info when switching to fuzzy
- **Transaction timing**: Debug spans around DB transactions
- **User name resolution**: Debug when cache misses occur

### Debug Helpers

```bash
# Inspect database
sqlite3 ~/.mcp-slack/cache.db

# Common queries:
.tables                                    # List tables
.schema users                              # Show table structure
SELECT COUNT(*) FROM users;                # Count cached users
SELECT COUNT(*) FROM channels;             # Count cached channels
SELECT * FROM metadata;                    # Check last sync times
SELECT * FROM locks;                       # Check active locks
SELECT COUNT(*) FROM users_fts;            # Verify FTS5 index

# Test FTS5 search:
SELECT id, name, email FROM users
WHERE rowid IN (
    SELECT rowid FROM users_fts WHERE users_fts MATCH 'john'
) LIMIT 5;

# Check cache freshness:
SELECT
    key,
    datetime(CAST(value AS INTEGER), 'unixepoch') as last_sync,
    (unixepoch() - CAST(value AS INTEGER)) / 3600 as hours_ago
FROM metadata
WHERE key LIKE 'last_%_sync';
```

### Performance Profiling

```bash
# Build with profiling
cargo build --release

# Run with perf (Linux)
perf record --call-graph dwarf ./target/release/mcp-slack
perf report

# Use Instruments (macOS)
instruments -t "Time Profiler" ./target/release/mcp-slack

# Memory profiling with valgrind
valgrind --tool=massif ./target/release/mcp-slack
```

## Future Considerations

### Potential Enhancements

- [ ] **Incremental cache updates**: Use Slack Events API for real-time updates
- [ ] **Compression**: SQLite page compression for large workspaces
- [ ] **Query caching**: LRU cache for frequently accessed data
- [ ] **Metrics export**: Prometheus metrics for monitoring
- [ ] **Streaming responses**: For large result sets
- [ ] **Connection pool tuning**: Dynamic pool sizing based on load
- [ ] **Advanced search**: Semantic search with embeddings
- [ ] **Multi-workspace**: Support multiple Slack workspaces
- [ ] **Encryption**: SQLite encryption for sensitive data
- [ ] **Backup/restore**: Automated cache backup mechanism

### Known Limitations

1. **Message search requires user token**: Bot tokens can't access search.messages API
2. **No real-time updates**: Cache is TTL-based, not event-driven
3. **FTS5 query syntax**: Limited compared to Elasticsearch
4. **Single database file**: Not distributed (but WAL enables concurrent reads)
5. **Rate limiting**: Default 20 req/min (can be increased but Slack may throttle)

## Security Considerations

### Token Safety
- Never log full tokens (mask in logs: `xoxb-***`)
- Store tokens in environment variables, not in code
- Use config files with restricted permissions (0600)

### Database Security
- Cache contains user emails and profile data
- Database file should have restricted permissions
- Consider SQLite encryption for sensitive environments

### API Permissions
- Request minimal scopes needed
- Bot tokens preferred over user tokens when possible
- Regularly audit token usage

---

## Quick Reference

### Environment Setup

```bash
# Required
export SLACK_BOT_TOKEN="xoxb-..."

# Optional
export SLACK_USER_TOKEN="xoxp-..."  # For message search
export DATA_PATH="~/.mcp-slack"
export LOG_LEVEL="info"
```

### Common Operations

```bash
# Build and run
cargo build --release
./target/release/mcp-slack

# Force cache refresh (delete and restart)
rm ~/.mcp-slack/cache.db
./target/release/mcp-slack

# Debug mode
RUST_LOG=debug ./target/release/mcp-slack

# Check cache status
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM metadata;"
```

### Tool Registration Pattern

```rust
// In src/mcp/handlers.rs
impl RequestHandler {
    pub async fn new(
        cache: Arc<SqliteCache>,
        slack_client: Arc<SlackClient>,
        config: Config,
    ) -> anyhow::Result<Self> {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();

        // Register tools with macro
        register_tool!(tools, "tool_name", ToolStruct::new(dependencies));

        // Background cache initialization if empty
        if cache.is_empty().await? {
            tokio::spawn(async move {
                // Fetch and cache
            });
        }

        Ok(Self { tools })
    }
}
```

### Error Handling Template

```rust
use crate::error::{McpError, McpResult};

pub async fn operation() -> McpResult<Value> {
    // Use ? for error propagation
    let data = fetch_data().await
        .map_err(|e| McpError::Internal(format!("Fetch failed: {}", e)))?;

    // Validate
    if data.is_empty() {
        return Err(McpError::NotFound("No data found".to_string()));
    }

    // Return success
    Ok(json!({ "result": data }))
}
```

---

*Last updated: Reflects current implementation with modular cache architecture and 8 MCP tools*

*Version: 0.1.0 (Rust 2024 edition, requires 1.90+)*
