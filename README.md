# Slack MCP Server

<!-- Core Project Info -->
[![Rust](https://img.shields.io/badge/rust-1.90%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/yourusername/mcp-slack/releases)

<!-- Features -->
[![Tools](https://img.shields.io/badge/MCP%20tools-8-blue?style=flat-square)](#-available-tools)
[![MCP](https://img.shields.io/badge/MCP-2025--06--18-blue?style=flat-square)](https://modelcontextprotocol.io)

<!-- Tech Stack -->
[![Tokio](https://img.shields.io/badge/tokio-1.47-blue?style=flat-square&logo=tokio)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/SQLite-FTS5-blue?style=flat-square&logo=sqlite)](https://www.sqlite.org)

> **Production-ready Model Context Protocol (MCP) server that enables AI assistants to interact with Slack workspaces through intelligent caching and full-text search.**

Connect Claude Desktop or any MCP-compatible AI assistant to your Slack workspace with SQLite-based caching, FTS5 full-text search, and comprehensive workspace operations.

---

## 📖 Table of Contents

- [What is this?](#-what-is-this)
- [Why Slack MCP Server?](#-why-slack-mcp-server)
- [Key Features](#-key-features)
- [Quick Start](#-quick-start)
- [Available Tools](#-available-tools)
- [Usage Examples](#-usage-examples)
- [Configuration](#-configuration)
- [Architecture](#-architecture)
- [Performance](#-performance)
- [Troubleshooting](#-troubleshooting)
- [Development](#-development)
- [FAQ](#-faq)
- [Contributing](#-contributing)

---

## 🎯 What is this?

**Slack MCP Server** is a high-performance Rust implementation of the Model Context Protocol that bridges AI assistants (like Claude) with Slack. It provides:

- **Direct Slack Integration**: Send messages, read threads, search conversations
- **Intelligent Caching**: SQLite-based cache with FTS5 full-text search
- **User Name Resolution**: Automatically enriches messages with user names
- **Production Ready**: Distributed locking, rate limiting, automatic cache refresh

### What is MCP?

The [Model Context Protocol (MCP)](https://modelcontextprotocol.io) is an open standard that enables AI assistants to securely interact with external data sources and tools. This server implements MCP to give AI assistants native Slack capabilities.

---

## 💡 Why Slack MCP Server?

### The Problem
AI assistants can't directly access your Slack workspace for:
- Reading team conversations and context
- Sending messages or updates
- Searching for users and channels
- Following discussion threads

### The Solution
This MCP server acts as a secure bridge:

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│             │  MCP    │   Slack MCP      │  API    │             │
│   Claude    │◄───────►│     Server       │◄───────►│    Slack    │
│  Desktop    │         │  (This Project)  │         │  Workspace  │
│             │         │                  │         │             │
└─────────────┘         └──────────────────┘         └─────────────┘
                              │
                              ▼
                        ┌──────────┐
                        │  SQLite  │
                        │  Cache   │
                        └──────────┘
```

### Why This Implementation?

| Feature | Description |
|---------|-------------|
| **Language** | Rust (memory-safe, concurrent) |
| **Caching** | SQLite WAL mode with FTS5 full-text search |
| **Search** | FTS5 indexed search with fuzzy fallback |
| **Concurrency** | Distributed locking, connection pooling (r2d2) |
| **User Experience** | Auto-enriched user names in messages |
| **Rate Limiting** | Token bucket with exponential backoff |

---

## ✨ Key Features

### 🚀 Performance
- **SQLite Caching**: WAL mode for concurrent access
- **FTS5 Full-Text Search**: Indexed search for users and channels
- **Smart Caching**: Auto-refresh on startup, TTL-based updates
- **Connection Pooling**: r2d2 with configurable pool size

### 🔍 Search Capabilities
- **User Search**: Name, email, display name (FTS5 + fuzzy fallback)
- **Channel Search**: Public, private, DMs, multi-person DMs
- **Message Search**: Full workspace search with user name enrichment

### 💬 Messaging
- **Send Messages**: Channels, DMs, threads
- **Read Conversations**: Channel history, threaded discussions
- **User Resolution**: Messages automatically include user names
- **Thread Optimization**: Parent info provided once, not per message

### 🛡️ Production Ready
- **Rate Limiting**: Configurable req/min with exponential backoff
- **Distributed Locking**: SQLite-based with timeout
- **Error Handling**: Comprehensive error types with context
- **Concurrent Safe**: WAL mode for multi-instance deployment

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.90+ (2024 edition)
- Slack workspace with admin access
- Claude Desktop or MCP-compatible client

### 1. Install

```bash
# Clone repository
git clone https://github.com/yourusername/mcp-slack
cd mcp-slack

# Build release binary
cargo build --release

# Binary location: target/release/mcp-slack
```

### 2. Create Slack App

1. Visit [api.slack.com/apps](https://api.slack.com/apps)
2. Click **"Create New App"** → **"From scratch"**
3. Name your app (e.g., "MCP Assistant") and select workspace
4. Navigate to **"OAuth & Permissions"**
5. Add **Bot Token Scopes** (see [Required Scopes](#required-scopes))
6. Click **"Install to Workspace"**
7. Copy **Bot User OAuth Token** (starts with `xoxb-`)

<details>
<summary>📸 Visual Setup Guide (Click to expand)</summary>

**Step 1**: Create App → From Scratch
![Create App](docs/images/create-app.png)

**Step 2**: Add OAuth Scopes
![OAuth Scopes](docs/images/oauth-scopes.png)

**Step 3**: Install & Get Token
![Install App](docs/images/install-app.png)

</details>

### 3. Configure Claude Desktop

Edit configuration file:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "slack": {
      "command": "/absolute/path/to/mcp-slack/target/release/mcp-slack",
      "env": {
        "SLACK_BOT_TOKEN": "xoxb-your-bot-token-here",
        "LOG_LEVEL": "warn"
      }
    }
  }
}
```

### 4. Verify Installation

Restart Claude Desktop and try:

> "Search for users named John in Slack"

> "Send a message to #general: Hello team!"

> "Show me recent messages from #engineering"

---

## 🛠️ Available Tools

### Core Messaging (4 tools)

#### `send_message`
Send messages to channels, users, or threads.

**Parameters:**
- `channel` (required): Channel name (#general), ID (C1234), or username (@user)
- `text` (required): Message text (supports Slack markdown)
- `thread_ts` (optional): Thread timestamp to reply to

**Example:**
```json
{
  "channel": "#general",
  "text": "Hello team! Check out this *important* update."
}
```

---

#### `get_channel_messages`
Retrieve message history from a channel.

**Parameters:**
- `channel` (required): Channel name or ID
- `limit` (optional): Max messages (default: 100, max: 1000)
- `cursor` (optional): Pagination cursor

**Returns:** Messages with `user_id` and `user_name` fields enriched from cache

**Example:**
```json
{
  "channel": "#engineering",
  "limit": 50
}
```

---

#### `read_thread`
Get all messages in a thread conversation.

**Parameters:**
- `channel` (required): Channel containing the thread
- `thread_ts` (required): Thread parent timestamp

**Returns:** Optimized format with parent info once + message array

**Example:**
```json
{
  "channel": "C1234567",
  "thread_ts": "1234567890.123456"
}
```

**Response Format:**
```json
{
  "thread_info": {
    "parent_ts": "1234567890.123456",
    "parent_text": "Original message",
    "parent_user_id": "U123ABC",
    "parent_user_name": "Alice Johnson"
  },
  "messages": [
    {
      "ts": "1234567890.123457",
      "text": "Reply message",
      "user_id": "U456DEF",
      "user_name": "Bob Smith"
    }
  ]
}
```

---

#### `list_channel_members`
Get all members of a channel with user details.

**Parameters:**
- `channel` (required): Channel name or ID

**Returns:** Array of users with names, emails, and status

---

### Search & Discovery (3 tools)

#### `search_users`
Find users by name, email, or display name using FTS5 full-text search.

**Parameters:**
- `query` (required): Search term
- `limit` (optional): Max results (default: 10)

**Search Features:**
- Full-text search with FTS5 (exact + partial matching)
- Automatic fuzzy fallback for no results
- Searches: username, display name, real name, email
- Excludes bots by default

**Example:**
```json
{
  "query": "john",
  "limit": 5
}
```

---

#### `search_channels`
Find channels by name (public, private, DMs, multi-person DMs).

**Parameters:**
- `query` (required): Channel name search term
- `limit` (optional): Max results (default: 10)

**Features:**
- FTS5 full-text search on channel names
- Returns all channel types user has access to
- Instant results from cache

---

#### `search_messages`
Search all workspace messages (requires user token).

**Parameters:**
- `query` (required): Search query (supports Slack search syntax)
- `channel` (optional): Limit to specific channel
- `from_user` (optional): Filter by user
- `limit` (optional): Max results (default: 10)

**Features:**
- Full workspace search using Slack API
- Automatic user name enrichment
- Supports Slack search operators (`in:#channel`, `from:@user`, etc.)

---

### System (1 tool)

#### `refresh_cache`
Manually refresh the SQLite cache.

**Parameters:**
- `scope` (optional): What to refresh
  - `"users"` - Only user data
  - `"channels"` - Only channel data
  - `"all"` - Everything (default)

**When to use:**
- After adding new team members
- When channel list changes
- To force cache update (bypasses TTL)

---

## 📚 Usage Examples

### Example 1: Daily Standup Assistant

**User:** "Get recent messages from #engineering and summarize them"

**Behind the scenes:**
1. MCP calls `search_channels` with query "engineering"
2. Gets channel ID from cache
3. MCP calls `get_channel_messages` with channel ID
4. Returns messages with user names enriched
5. Claude summarizes the content

---

### Example 2: Team Member Lookup

**User:** "Find Sarah's email address"

**Behind the scenes:**
1. MCP calls `search_users` with query "sarah"
2. FTS5 searches name, display_name, real_name fields
3. Returns user object with email from cache

---

### Example 3: Send Update to Multiple Channels

**User:** "Send 'Deployment complete' to #engineering and #ops"

**Behind the scenes:**
1. MCP calls `search_channels` for "engineering"
2. MCP calls `search_channels` for "ops"
3. MCP calls `send_message` twice with channel IDs
4. Both messages sent with rate limiting

---

### Example 4: Thread Conversation Analysis

**User:** "Show me the discussion in that thread about the API redesign"

**Behind the scenes:**
1. MCP calls `search_messages` with query "API redesign"
2. Gets message with thread_ts
3. MCP calls `read_thread` with channel + thread_ts
4. Returns optimized thread format
5. Claude analyzes the discussion

---

## ⚙️ Configuration

### Environment Variables

```bash
# Required (at least one)
SLACK_BOT_TOKEN=xoxb-...      # Bot token from Slack app
SLACK_USER_TOKEN=xoxp-...     # User token (for message search)

# Optional
DATA_PATH=~/.mcp-slack         # SQLite database location
LOG_LEVEL=warn                 # error | warn | info | debug | trace
RUST_LOG=mcp_slack=debug      # Module-specific logging
```

### Configuration File (Optional)

Create `config.toml` in project root or `~/.mcp-slack/config.toml`:

```toml
[slack]
bot_token = "xoxb-..."         # Can use env vars instead
user_token = "xoxp-..."

[cache]
ttl_users_hours = 24           # User cache TTL (default: 24)
ttl_channels_hours = 24        # Channel cache TTL (default: 24)
ttl_members_hours = 48         # Member list TTL (default: 48)
data_path = "~/.mcp-slack"     # Database location

[retry]
max_attempts = 3               # API retry attempts
initial_delay_ms = 1000        # Initial backoff delay
max_delay_ms = 32000           # Max backoff delay
exponential_base = 2.0         # Backoff multiplier

[rate_limit]
requests_per_minute = 20       # Slack API rate limit

[connection]
timeout_seconds = 30           # Request timeout
max_connections = 10           # Connection pool size
```

### Required Scopes

#### Bot Token Scopes (Required)

Add these scopes in Slack App settings → OAuth & Permissions → Bot Token Scopes:

**Channels & Conversations:**
```
channels:read       - List public channels
channels:history    - Read public channel messages
groups:read         - List private channels
groups:history      - Read private channel messages
im:read            - List direct messages
im:history         - Read DM messages
mpim:read          - List multi-person DMs
mpim:history       - Read multi-person DM messages
```

**Users:**
```
users:read         - List workspace users
users:read.email   - Read user email addresses
```

**Messaging:**
```
chat:write         - Send messages as bot
chat:write.public  - Send to channels bot isn't in
reactions:write    - Add emoji reactions
```

#### User Token Scopes (Optional - For Enhanced Features)

For message search, you need a user token with:
```
search:read        - Search workspace messages
```

<details>
<summary>How to get a User Token</summary>

1. In your Slack App settings, go to **OAuth & Permissions**
2. Under **User Token Scopes**, add `search:read`
3. Reinstall the app to your workspace
4. Copy the **User OAuth Token** (starts with `xoxp-`)
5. Set as `SLACK_USER_TOKEN` environment variable

</details>

---

## 🏗️ Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        AI Assistant                          │
│                      (Claude Desktop)                        │
└────────────────────────────┬────────────────────────────────┘
                             │ MCP Protocol (JSON-RPC)
                             │ stdio transport
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                     MCP Server (Rust)                        │
│  ┌────────────┐  ┌────────────┐  ┌─────────────────────┐   │
│  │    MCP     │  │   Tools    │  │    Slack Client     │   │
│  │  Protocol  │─▶│  Handler   │─▶│  (Rate Limited)     │   │
│  │   Layer    │  │            │  │                     │   │
│  └────────────┘  └────────────┘  └──────────┬──────────┘   │
│                                              │               │
│                                              │               │
│  ┌─────────────────────────────────────────┐│               │
│  │        SQLite Cache (WAL Mode)          ││               │
│  │  ┌──────────┐  ┌──────────┐  ┌────────┐││               │
│  │  │  Users   │  │ Channels │  │ Locks  │││               │
│  │  │  + FTS5  │  │  + FTS5  │  │        │││               │
│  │  └──────────┘  └──────────┘  └────────┘││               │
│  └─────────────────────────────────────────┘│               │
│                                              │               │
└──────────────────────────────────────────────┼───────────────┘
                                               │
                                               │ HTTPS
                                               ▼
                                     ┌──────────────────┐
                                     │   Slack API      │
                                     │  (api.slack.com) │
                                     └──────────────────┘
```

### Component Details

#### 1. MCP Protocol Layer (`src/mcp/`)
- **server.rs**: JSON-RPC server over stdio
- **handlers.rs**: Request routing, auto-cache initialization
- **types.rs**: MCP protocol types (Tool, Property, etc.)

#### 2. Tools Layer (`src/tools/`)
- **search.rs**: User/channel/message search tools
- **messages.rs**: Send, read, thread operations
- **cache.rs**: Manual cache refresh
- **message_utils.rs**: User name enrichment utilities
- **response.rs**: Standardized tool responses

#### 3. Slack Client (`src/slack/`)
- **client.rs**: Unified facade for all Slack operations
- **core.rs**: HTTP client with rate limiting
- **users.rs**: User-related API calls
- **channels.rs**: Channel operations
- **messages.rs**: Message operations
- **api_config.rs**: API method configs (GET/POST/form-encoded)
- **types.rs**: Slack data models

#### 4. Cache System (`src/cache/`)
- **sqlite_cache.rs**: Main implementation
- **schema.rs**: Table definitions, FTS5 indexes
- **users.rs**: User caching logic
- **channels.rs**: Channel caching logic
- **locks.rs**: Distributed locking mechanism
- **helpers.rs**: Query sanitization, atomic swaps

#### 5. Configuration (`src/config.rs`)
- Default values for TTLs, retry logic
- Environment variable loading
- Config file parsing

### Data Flow Examples

#### User Search Flow
```
1. User asks: "Find john@company.com"
2. MCP → search_users tool
3. Tool → SQLite FTS5 query on users_fts table
4. Cache hit → fast response
5. Return: { id: "U123", name: "John Doe", email: "john@company.com" }
```

#### Send Message Flow
```
1. User: "Send message to #engineering"
2. MCP → send_message tool
3. Tool → search_channels from cache (instant)
4. Get channel ID: C1234567
5. Tool → Slack API chat.postMessage
6. Rate limiter: check token bucket
7. HTTP POST with exponential backoff
8. Return: Success + message timestamp
```

#### Cache Refresh Flow
```
1. On startup: Check cache staleness
2. If stale or empty:
   a. Acquire distributed lock (SQLite)
   b. Create temp tables
   c. Fetch from Slack API (async)
   d. Begin transaction
   e. Atomic swap (DELETE + INSERT)
   f. Update metadata timestamps
   g. Commit transaction
   h. Release lock
3. Background task completes
```

---

## 📊 Performance

### Architecture Characteristics

**Caching Strategy:**
- SQLite WAL mode for concurrent reads
- FTS5 full-text search indexes
- TTL-based cache refresh (24h default)
- Atomic updates via transactions

**Concurrency:**
- r2d2 connection pooling
- Distributed locking mechanism
- Tokio async runtime

**Rate Limiting:**
- Token bucket implementation
- Exponential backoff on failures
- Configurable requests per minute

### Technical Stack

- **Language**: Rust 2024 edition (requires 1.90+)
- **Runtime**: Tokio 1.47 (async I/O)
- **Database**: SQLite with FTS5 (rusqlite 0.32)
- **HTTP Client**: reqwest 0.12 with rustls
- **Rate Limiting**: governor 0.8
- **Connection Pool**: r2d2 0.8

### Resource Considerations

**Storage:**
- Cache database size scales with workspace size
- FTS5 indexes add ~20-30% overhead
- WAL mode uses temporary files during writes

**Memory:**
- Baseline: Rust binary + SQLite overhead
- Scales with connection pool size
- Cache data loaded on demand

**Network:**
- Cache-first strategy minimizes API calls
- Rate limiting prevents API throttling
- Automatic retry with backoff

---

## 🐛 Troubleshooting

### Common Issues

#### 1. "database is locked" Error

**Symptom:**
```
Error: database is locked
```

**Cause:** Multiple processes trying to write simultaneously (rare with WAL mode)

**Solution:**
```bash
# WAL mode handles this automatically, but if persists:
# 1. Check for stale locks
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM locks;"

# 2. Remove stale locks (older than 30s)
sqlite3 ~/.mcp-slack/cache.db "DELETE FROM locks WHERE created_at < unixepoch() - 30;"

# 3. Restart server
```

---

#### 2. Cache Not Refreshing

**Symptom:** Old data showing up, new users/channels missing

**Solution:**
```bash
# Force refresh by deleting cache
rm ~/.mcp-slack/cache.db

# Restart Claude Desktop or MCP server
# Cache will auto-initialize on startup

# Or use refresh_cache tool:
# In Claude: "Refresh the Slack cache"
```

---

#### 3. "Unauthorized" or "Invalid Token"

**Symptom:**
```
Error: Unauthorized - check Slack token
```

**Solutions:**
1. Verify token starts with `xoxb-` (bot) or `xoxp-` (user)
2. Check token in config file or env var matches Slack app
3. Ensure all required scopes are added
4. Reinstall Slack app if scopes were added after install

```bash
# Test token manually:
curl -H "Authorization: Bearer xoxb-YOUR-TOKEN" \
  https://slack.com/api/auth.test
```

---

#### 4. No Search Results

**Symptom:** Search returns empty even though data exists

**Solutions:**

**For user search:**
```bash
# Check FTS5 index
sqlite3 ~/.mcp-slack/cache.db "SELECT COUNT(*) FROM users_fts;"

# Rebuild FTS5 index if zero:
sqlite3 ~/.mcp-slack/cache.db "INSERT INTO users_fts(users_fts) VALUES('rebuild');"
```

**For message search:**
- Ensure `SLACK_USER_TOKEN` is set (bot token can't search messages)
- Verify user token has `search:read` scope

---

#### 5. Slow Performance

**Symptom:** Queries taking longer than expected

**Diagnostics:**
```bash
# Enable debug logging
LOG_LEVEL=debug cargo run

# Check cache size
sqlite3 ~/.mcp-slack/cache.db "
  SELECT
    (SELECT COUNT(*) FROM users) as users,
    (SELECT COUNT(*) FROM channels) as channels,
    (SELECT COUNT(*) FROM users_fts) as users_fts_rows;
"

# Check lock contention
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM locks;"
```

**Solutions:**
- Increase connection pool size in config
- Reduce cache TTL to avoid stale data
- Run `VACUUM` on database if size is large

---

### Debug Mode

Enable detailed logging:

```bash
# Full debug output
RUST_LOG=debug cargo run

# Module-specific
RUST_LOG=mcp_slack::cache=debug,mcp_slack::slack=info cargo run

# Or in config.toml
LOG_LEVEL=debug
```

**Log Levels:**
- `error`: Only critical errors
- `warn`: Warnings + errors
- `info`: Key operations + warnings
- `debug`: Detailed flow + info
- `trace`: Everything (very verbose)

---

### Database Inspection

```bash
# Open SQLite shell
sqlite3 ~/.mcp-slack/cache.db

# Useful commands:
.tables                              # List all tables
.schema users                        # Show table structure
SELECT COUNT(*) FROM users;          # Count users
SELECT COUNT(*) FROM channels;       # Count channels
SELECT * FROM metadata;              # Check last update times
SELECT * FROM locks;                 # Check active locks
.quit
```

---

## 🔧 Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/yourusername/mcp-slack
cd mcp-slack

# Development build (fast compile, slow runtime)
cargo build

# Release build (slow compile, fast runtime)
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Project Structure

```
src/
├── main.rs                 # Entry point, async runtime setup
├── lib.rs                  # Library exports
├── config.rs               # Configuration management
├── error.rs                # Custom error types
├── utils.rs                # Shared utilities
│
├── mcp/                    # MCP Protocol Layer
│   ├── mod.rs
│   ├── server.rs          # JSON-RPC stdio server
│   ├── handlers.rs        # Tool routing, auto-cache init
│   └── types.rs           # MCP type definitions
│
├── slack/                  # Slack API Client
│   ├── mod.rs
│   ├── client.rs          # Unified client facade
│   ├── core.rs            # HTTP client + rate limiting
│   ├── users.rs           # User operations
│   ├── channels.rs        # Channel operations
│   ├── messages.rs        # Message operations
│   ├── api_config.rs      # API method configurations
│   └── types.rs           # Slack data models
│
├── cache/                  # SQLite Cache System
│   ├── mod.rs
│   ├── sqlite_cache.rs    # Main cache implementation
│   ├── schema.rs          # Table schemas, FTS5
│   ├── users.rs           # User caching logic
│   ├── channels.rs        # Channel caching logic
│   ├── locks.rs           # Distributed locking
│   └── helpers.rs         # Utilities, sanitization
│
└── tools/                  # MCP Tool Implementations
    ├── mod.rs
    ├── search.rs          # User/channel/message search
    ├── messages.rs        # Send, read, threads
    ├── cache.rs           # Manual refresh
    ├── message_utils.rs   # Name enrichment
    └── response.rs        # Response formatting
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint with clippy (strict)
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test

# Run specific test
cargo test test_name

# Check compilation without building
cargo check

# Security audit
cargo audit
```

### Adding New Tools

1. **Create tool struct** in `src/tools/your_tool.rs`:

```rust
use async_trait::async_trait;
use serde_json::Value;
use crate::error::McpResult;
use crate::tools::Tool;

pub struct YourTool {
    // dependencies
}

#[async_trait]
impl Tool for YourTool {
    fn description(&self) -> &str {
        "Description for AI to understand"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        // Implementation
        Ok(json!({ "result": "success" }))
    }
}
```

2. **Register in `src/mcp/handlers.rs`**:

```rust
register_tool!(tools, "your_tool", YourTool::new(deps));
```

3. **Add to `src/tools/mod.rs`**:

```rust
pub mod your_tool;
```

---

## ❓ FAQ

### General Questions

**Q: What is MCP?**
A: Model Context Protocol - an open standard for AI assistants to interact with external tools and data sources. Learn more at [modelcontextprotocol.io](https://modelcontextprotocol.io)

**Q: Does this work with other AI assistants besides Claude?**
A: Yes! Any MCP-compatible client can use this server. Currently supported: Claude Desktop, Continue.dev (VS Code), and other MCP clients.

**Q: Is my Slack data stored externally?**
A: No. All data stays local on your machine in `~/.mcp-slack/cache.db`. The server only acts as a bridge between the AI and Slack API.

---

### Security & Privacy

**Q: What permissions does the Slack app need?**
A: Only the minimum required scopes for each feature. Bot token for reading/writing, optional user token only for message search. See [Required Scopes](#required-scopes).

**Q: Can I audit what data is sent to Slack?**
A: Yes. Run with `RUST_LOG=debug` to see all API calls. The code is open source for full transparency.

**Q: Is the cache encrypted?**
A: The SQLite database is not encrypted by default. For sensitive environments, use filesystem encryption or implement SQLite encryption.

---

### Performance

**Q: How much data does it cache?**
A: Depends on workspace size. A typical workspace with 5000 users might use ~15MB for the database.

**Q: Does it work offline?**
A: Cached data (users, channels) works offline. Live operations (send message, search messages) require internet.

**Q: How often does the cache refresh?**
A: Automatically on startup if stale (default TTL: 24h). Manual refresh with `refresh_cache` tool or by deleting cache.db.

---

### Troubleshooting

**Q: Why are message search results empty?**
A: Message search requires a **user token** (`xoxp-`) with `search:read` scope. Bot tokens can't search messages.

**Q: Getting "database is locked" errors?**
A: WAL mode should prevent this. If persistent, check for stale locks and ensure only one MCP server instance is running.

**Q: New team members not showing up?**
A: Run cache refresh: Tell Claude "Refresh the Slack cache" or manually delete `~/.mcp-slack/cache.db` and restart.

---

### Development

**Q: How do I add a new tool?**
A: See [Adding New Tools](#adding-new-tools) section.

**Q: Can I contribute?**
A: Yes! See [Contributing](#-contributing) section below.

**Q: How do I debug issues?**
A: Enable debug logging: `RUST_LOG=debug cargo run` or `LOG_LEVEL=debug` in config.

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### Contribution Process

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Make** your changes
4. **Test** thoroughly (`cargo test && cargo clippy`)
5. **Format** code (`cargo fmt`)
6. **Commit** with clear messages
7. **Push** to your fork
8. **Open** a Pull Request

### Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/mcp-slack
cd mcp-slack

# Create feature branch
git checkout -b feature/my-feature

# Make changes, then test
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Commit and push
git add .
git commit -m "feat: add amazing feature"
git push origin feature/my-feature
```

### Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): brief description

Detailed explanation of changes (optional)

- Bullet point 1
- Bullet point 2
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(cache): add compression support for large workspaces
fix(search): resolve FTS5 query sanitization issue
docs(readme): update performance benchmarks
refactor(slack): consolidate API client error handling
```

### Code Guidelines

- **Error Handling**: Use `?` operator and `McpResult<T>`
- **Async**: All I/O operations must be async
- **Documentation**: Public APIs need doc comments
- **Testing**: Add tests for new features
- **Performance**: Consider cache-first strategies

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test cache::tests

# Run with output
cargo test -- --nocapture

# Test coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Feature Requests & Bug Reports

Use GitHub Issues with templates:

**Bug Report:**
```markdown
**Describe the bug**
Clear description of the issue

**To Reproduce**
Steps to reproduce:
1. ...
2. ...

**Expected behavior**
What should happen

**Environment:**
- OS: [e.g. macOS 14.0]
- Rust version: [e.g. 1.90]
- MCP Server version: [e.g. 0.1.0]

**Logs**
Attach logs with `RUST_LOG=debug`
```

**Feature Request:**
```markdown
**Feature Description**
What you want to achieve

**Use Case**
Why this feature is useful

**Proposed Solution**
How you envision it working
```

---

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

### MIT License Summary

- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Private use
- ❌ Liability
- ❌ Warranty

---

## 🙏 Acknowledgments

This project stands on the shoulders of giants:

- **[Rust Language](https://www.rust-lang.org/)** - Systems programming language prioritizing safety and performance
- **[Tokio](https://tokio.rs/)** - Asynchronous runtime for Rust
- **[SQLite](https://www.sqlite.org/)** - Embedded database engine with FTS5
- **[MCP Protocol](https://modelcontextprotocol.io/)** - Model Context Protocol standard
- **[Anthropic](https://www.anthropic.com/)** - Claude and MCP development
- **[Slack API](https://api.slack.com/)** - Collaboration platform APIs

### Contributors

Thank you to all contributors who have helped improve this project!

[See full contributor list →](https://github.com/yourusername/mcp-slack/graphs/contributors)

---

## 📞 Support

- **Documentation**: This README + [CLAUDE.md](CLAUDE.md) for developers
- **Issues**: [GitHub Issues](https://github.com/yourusername/mcp-slack/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/mcp-slack/discussions)
- **Security**: See [SECURITY.md](SECURITY.md) for reporting vulnerabilities

---

<div align="center">

**Made with ❤️ by the MCP community**

[⭐ Star this repo](https://github.com/yourusername/mcp-slack) • [🐛 Report Bug](https://github.com/yourusername/mcp-slack/issues) • [💡 Request Feature](https://github.com/yourusername/mcp-slack/issues)

**Version 0.1.0** • Built with Rust 2024 Edition

</div>
