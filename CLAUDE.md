# Claude Assistant Instructions for Slack MCP Server

Technical documentation for AI assistants working on the Slack MCP Server project.

## Project Overview

Rust-based MCP server for Slack integration using SQLite caching with FTS5 full-text search, distributed locking, and token-efficient message formatting. Built with Rust 2024 edition (requires 1.90+).

## Core Architecture

### Storage Layer - SQLite
- **Database**: `cache.db` at `~/.mcp-slack/`
- **Mode**: WAL (Write-Ahead Logging)
- **Connection Pool**: r2d2 (HTTP client pool, not database connections)
- **Tables**: users, channels, locks, metadata, users_fts, channels_fts

### Caching Strategy
- Auto-initialization on startup if empty/stale
- TTL-based: 24h users/channels, 12h members
- Atomic updates via transactions
- Distributed locking with timeout
- Snappy compression

## Component Architecture

### 1. MCP Protocol Layer (`src/mcp/`)
- **server.rs**: JSON-RPC stdio server
- **handlers.rs**: Tool registration and routing (8 tools)
- **types.rs**: MCP protocol types

### 2. Slack Client (`src/slack/`)
- **client.rs**: Unified facade
- **core.rs**: HTTP client with governor rate limiting
- **users.rs**: `fetch_all_users`
- **channels.rs**: `fetch_all_channels`
- **messages.rs**: `send_message`, `get_channel_messages`, `get_thread_messages`
- **api_config.rs**: API method configs
- **types.rs**: Slack data models

### 3. Cache System (`src/cache/`)
Modular structure:
- **sqlite_cache.rs**: Main implementation, connection pool management
- **schema.rs**: Table definitions, FTS5, triggers
- **users.rs**: `save_users()`, `search_users()`, `get_user()`
- **channels.rs**: `save_channels()`, `search_channels()`, `get_channel()`
- **locks.rs**: `acquire_lock()`, `release_lock()`
- **helpers.rs**: `process_fts_query()`, `atomic_swap()`

### 4. Tools (`src/tools/`)
- **search.rs**: SearchUsersTool, SearchChannelsTool, SearchMessagesTool (3)
- **messages.rs**: SendMessageTool, GetChannelMessagesTool, ReadThreadTool, ListChannelMembersTool (4)
- **cache.rs**: RefreshCacheTool (1)
- **message_utils.rs**: `format_message()`, `format_thread_messages()`, `resolve_channel_identifier()`
- **response.rs**: ToolResponse, IntoToolResponse

**Total: 8 MCP tools**

### 5. Configuration (`src/config.rs`)
Default constants:
```rust
DEFAULT_TTL_USERS_HOURS: 24
DEFAULT_TTL_CHANNELS_HOURS: 24
DEFAULT_TTL_MEMBERS_HOURS: 12
DEFAULT_COMPRESSION: "snappy"
DEFAULT_MAX_ATTEMPTS: 3
DEFAULT_INITIAL_DELAY_MS: 1000
DEFAULT_MAX_DELAY_MS: 60000
DEFAULT_EXPONENTIAL_BASE: 2.0
DEFAULT_TIMEOUT_SECONDS: 30
DEFAULT_MAX_IDLE_PER_HOST: 10
DEFAULT_POOL_IDLE_TIMEOUT_SECONDS: 90
```

### 6. Error Handling (`src/error.rs`)
`McpError` enum variants:
- `NotFound`, `InvalidParameter`, `Internal`, `Unauthorized`, `RateLimited`, `SlackApi`
- Type alias: `McpResult<T>`

### 7. Main Entry Point (`src/main.rs`)
- Tokio runtime setup
- Config loading (env + file)
- Logging initialization
- SlackClient and SqliteCache creation
- MCP server startup

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

-- FTS5 virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS users_fts USING fts5(
    id UNINDEXED,
    name, display_name, real_name, email,
    content=users,
    content_rowid=rowid,
    tokenize='porter unicode61'
);

-- Triggers keep FTS5 synchronized
CREATE TRIGGER users_ai AFTER INSERT ON users BEGIN
    INSERT INTO users_fts(rowid, id, name, display_name, real_name, email)
    VALUES (new.rowid, new.id, new.name, new.display_name, new.real_name, new.email);
END;
```

Similar structure for channels table with is_private, is_im, is_mpim flags.

### Cache Update Process (Atomic Swap)

1. Acquire distributed lock
2. Create temporary table
3. Insert new data from Slack API
4. Begin transaction
5. DELETE all from main table
6. INSERT from temp to main
7. Update metadata timestamps
8. Commit transaction
9. Drop temp table
10. Release lock

Ensures zero downtime and atomicity.

### Message Formatting

```rust
// src/tools/message_utils.rs
pub async fn format_message(
    msg: SlackMessage,
    cache: &Arc<SqliteCache>,
    include_thread_info: bool
) -> Value {
    // 1. Extract user_id
    // 2. Look up in cache
    // 3. Get display_name, fallback to real_name, then username
    // 4. Return JSON with user_id AND user_name
    // 5. Blocks and attachments excluded for token efficiency
}

pub async fn format_thread_messages(
    messages: Vec<SlackMessage>,
    cache: &Arc<SqliteCache>
) -> Value {
    // Returns: { "thread_info": {...}, "messages": [...] }
    // Parent info provided once, not repeated
}
```

### Rate Limiting

Governor crate with token bucket:
- Configured per Slack API method
- Exponential backoff on 429 responses
- Respects Retry-After header
- Max 3 retries

### FTS5 Search

```rust
// src/cache/users.rs
pub async fn search_users(&self, query: &str, limit: usize) -> McpResult<Vec<User>> {
    // 1. Sanitize query with process_fts_query()
    // 2. Try FTS5: WHERE rowid IN (SELECT rowid FROM users_fts WHERE users_fts MATCH ?)
    // 3. If empty, fallback to fuzzy-matcher
    // 4. Return results
}
```

## Configuration

### Environment Variables
- `SLACK_BOT_TOKEN` - Bot OAuth token (xoxb-)
- `SLACK_USER_TOKEN` - User OAuth token (xoxp-) for message search
- `DATA_PATH` - Database location (default: ~/.mcp-slack)
- `LOG_LEVEL` - error/warn/info/debug/trace
- `RUST_LOG` - Module-specific (e.g., `mcp_slack::cache=debug`)

### Config File (config.toml)
```toml
[slack]
bot_token = "xoxb-..."
user_token = "xoxp-..."

[cache]
ttl_users_hours = 24
ttl_channels_hours = 24
ttl_members_hours = 12
data_path = "~/.mcp-slack"
compression = "snappy"

[retry]
max_attempts = 3
initial_delay_ms = 1000
max_delay_ms = 60000
exponential_base = 2.0

[connection]
timeout_seconds = 30
max_idle_per_host = 10
pool_idle_timeout_seconds = 90
```

## Development Guidelines

### Code Patterns
1. **Async Everything**: All I/O with async/await + Tokio
2. **Arc for Sharing**: `Arc<T>` for shared ownership
3. **Error Propagation**: Use `?` and `McpResult<T>`
4. **Connection Pooling**: r2d2 for HTTP client
5. **Atomic Operations**: All cache writes in transactions
6. **Modular Design**: Cache split into logical modules

### File Organization
```
src/
├── main.rs              # Entry point
├── lib.rs               # Exports
├── config.rs            # Configuration
├── error.rs             # Error types
├── utils.rs             # Utilities
├── mcp/                 # MCP layer
│   ├── server.rs
│   ├── handlers.rs
│   └── types.rs
├── slack/               # Slack client
│   ├── client.rs
│   ├── core.rs
│   ├── users.rs
│   ├── channels.rs
│   ├── messages.rs
│   ├── api_config.rs
│   └── types.rs
├── cache/               # SQLite cache
│   ├── sqlite_cache.rs
│   ├── schema.rs
│   ├── users.rs
│   ├── channels.rs
│   ├── locks.rs
│   └── helpers.rs
└── tools/               # MCP tools
    ├── search.rs
    ├── messages.rs
    ├── cache.rs
    ├── message_utils.rs
    └── response.rs
```

### Testing Commands
```bash
cargo test                           # All tests
cargo test cache::users::tests       # Specific module
cargo check                          # Check compilation
cargo fmt                            # Format
cargo clippy --all-targets -- -D warnings  # Lint
cargo build --release                # Optimized build
RUST_LOG=debug cargo run             # Debug logging
```

### Common Issues & Solutions

#### 1. "table has 3 columns but 2 values"
**Cause:** Schema mismatch
**Fix:** Ensure temp tables match main table schema with all columns

#### 2. "database is locked"
**Cause:** WAL mode handles this; rare with proper locking
**Fix:** Check for stale locks, ensure single writer

#### 3. FTS5 syntax errors
**Cause:** Special characters in queries
**Fix:** `process_fts_query()` in helpers.rs sanitizes

#### 4. Cache not refreshing
**Cause:** TTL logic or empty detection
**Fix:** Auto-refresh on startup; verify `is_cache_stale()`

#### 5. User names showing as IDs
**Cause:** Cache miss
**Fix:** Ensure cache populated; `format_message()` handles gracefully

## Technical Stack

### Core Dependencies
- **Rust 2024**: edition, requires 1.90+
- **Tokio 1.47**: async runtime
- **rusqlite 0.32**: SQLite with FTS5
- **r2d2 0.8 + r2d2_sqlite 0.25**: connection pooling
- **reqwest 0.12**: HTTP with rustls
- **governor 0.8**: rate limiting
- **serde 1.0 + serde_json 1.0**: serialization
- **fuzzy-matcher 0.3**: fuzzy search
- **tracing 0.1 + tracing-subscriber 0.3**: logging

### Architecture Characteristics

**Caching:**
- SQLite WAL mode
- FTS5 indexes
- TTL-based refresh
- Atomic transactions

**Concurrency:**
- Tokio async runtime
- r2d2 pooling
- Distributed locking
- Non-blocking I/O

**Reliability:**
- Exponential backoff
- Automatic retry
- Token bucket rate limiting
- Comprehensive errors

## API Coverage

### Slack APIs Implemented
- `users.list` - All workspace users
- `conversations.list` - All channel types
- `conversations.history` - Channel messages
- `conversations.replies` - Thread messages
- `conversations.members` - Channel members
- `chat.postMessage` - Send messages
- `search.messages` - Workspace search (user token only)

### Tool-to-API Mapping

| MCP Tool | Slack API | Token |
|----------|-----------|-------|
| search_users | Cache FTS5 | Bot/User |
| search_channels | Cache FTS5 | Bot/User |
| search_messages | search.messages | **User** |
| send_message | chat.postMessage | Bot/User |
| get_channel_messages | conversations.history | Bot/User |
| read_thread | conversations.replies | Bot/User |
| list_channel_members | conversations.members | Bot/User |
| refresh_cache | users.list, conversations.list | Bot/User |

## Response Payload Design

Token-efficient format:
- Message `text` field included
- Blocks and attachments excluded
- Empty strings omitted
- Null values omitted (`skip_serializing_if`)
- Boolean flags only when true

**Implementation:**
- `format_message()` and `format_thread_messages()` exclude blocks/attachments
- `skip_serializing_if = "Option::is_none"` on Optional fields
- `remove_empty_strings()` helper filters empties

**Included Data:**
- Message text
- User ID + user name
- Timestamps (Slack ts + ISO 8601)
- Thread structure
- Pagination metadata

**Token Usage:** ~135 tokens/message, ~20 tokens/user

## Best Practices

### When Writing Code

1. **Cache First**: Check cache before API calls
2. **Batch Operations**: Use transactions for multiple updates
3. **Lock Responsibly**: Minimal critical sections
4. **Log with Context**: Include operation details
   ```rust
   tracing::debug!(user_id = %user.id, action = "cache_save", "Saving user");
   ```
5. **Test Fallbacks**: FTS5 to fuzzy fallback
6. **Handle Empty Strings**: Check empty, not just None
   ```rust
   let name = user.display_name
       .filter(|s| !s.is_empty())
       .or(user.real_name.filter(|s| !s.is_empty()))
       .unwrap_or(&user.name);
   ```
7. **Thread Format**: Parent info only once

### When Debugging

1. **Enable Logging**:
   ```bash
   RUST_LOG=mcp_slack=debug cargo run
   ```

2. **Check Cache**:
   ```bash
   sqlite3 ~/.mcp-slack/cache.db "
     SELECT
       (SELECT COUNT(*) FROM users) as users,
       (SELECT COUNT(*) FROM channels) as channels,
       (SELECT value FROM metadata WHERE key='last_users_sync') as last_sync
   "
   ```

3. **Monitor Locks**:
   ```bash
   sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM locks;"
   ```

4. **Trace API Calls**: Look for rate limit warnings

5. **Profile Performance**: Use tracing span timings
   ```rust
   let _span = tracing::debug_span!("operation").entered();
   // operation
   drop(_span);  // Logs duration
   ```

## Monitoring & Debugging

### Key Log Points
- Cache initialization status
- Lock acquisition/release with holder_id
- API rate limit hits (429)
- FTS5 fallback triggers
- Transaction timing
- User name resolution cache misses

### Debug Helpers

```bash
sqlite3 ~/.mcp-slack/cache.db

# Common queries:
.tables
.schema users
SELECT COUNT(*) FROM users;
SELECT COUNT(*) FROM channels;
SELECT * FROM metadata;
SELECT * FROM locks;
SELECT COUNT(*) FROM users_fts;

# Test FTS5:
SELECT id, name, email FROM users
WHERE rowid IN (SELECT rowid FROM users_fts WHERE users_fts MATCH 'john')
LIMIT 5;

# Cache freshness:
SELECT
    key,
    datetime(CAST(value AS INTEGER), 'unixepoch') as last_sync,
    (unixepoch() - CAST(value AS INTEGER)) / 3600 as hours_ago
FROM metadata
WHERE key LIKE 'last_%_sync';
```

## Security Considerations

### Token Safety
- Never log full tokens (mask: `xoxb-***`)
- Store in environment variables
- Config files with restricted permissions (0600)

### Database Security
- Cache contains emails and profile data
- Restrict database file permissions
- Consider SQLite encryption for sensitive environments

### API Permissions
- Request minimal scopes
- Prefer bot tokens over user tokens
- Regularly audit usage

## Quick Reference

### Environment Setup
```bash
# Required
export SLACK_BOT_TOKEN="xoxb-..."

# Optional
export SLACK_USER_TOKEN="xoxp-..."
export DATA_PATH="~/.mcp-slack"
export LOG_LEVEL="info"
```

### Common Operations
```bash
# Build and run
cargo build --release
./target/release/mcp-slack

# Force cache refresh
rm ~/.mcp-slack/cache.db
./target/release/mcp-slack

# Debug mode
RUST_LOG=debug ./target/release/mcp-slack

# Check cache
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM metadata;"
```

### Tool Registration Pattern
```rust
// src/mcp/handlers.rs
impl RequestHandler {
    pub async fn new(
        cache: Arc<SqliteCache>,
        slack_client: Arc<SlackClient>,
        config: Config,
    ) -> anyhow::Result<Self> {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();

        register_tool!(tools, "tool_name", ToolStruct::new(dependencies));

        if cache.is_empty().await? {
            tokio::spawn(async move { /* fetch and cache */ });
        }

        Ok(Self { tools })
    }
}
```

### Error Handling Template
```rust
use crate::error::{McpError, McpResult};

pub async fn operation() -> McpResult<Value> {
    let data = fetch_data().await
        .map_err(|e| McpError::Internal(format!("Fetch failed: {}", e)))?;

    if data.is_empty() {
        return Err(McpError::NotFound("No data found".to_string()));
    }

    Ok(json!({ "result": data }))
}
```

---

*Version: 0.1.0 (Rust 2024 edition, requires 1.90+)*
