# Slack MCP Server

[![Rust](https://img.shields.io/badge/rust-1.90%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/yourusername/mcp-slack/releases)
[![Tools](https://img.shields.io/badge/MCP%20tools-8-blue?style=flat-square)](#available-tools)
[![MCP](https://img.shields.io/badge/MCP-2025--06--18-blue?style=flat-square)](https://modelcontextprotocol.io)

> **Production-ready Model Context Protocol (MCP) server for Slack integration with SQLite caching and FTS5 full-text search.**

Rust-based MCP server that enables AI assistants to interact with Slack workspaces through 8 MCP tools, featuring intelligent caching, user name enrichment, and token-efficient message formatting.

---

## Table of Contents

- [What is this?](#what-is-this)
- [Key Features](#key-features)
- [Quick Start](#quick-start)
- [Available Tools](#available-tools)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [License](#license)

---

## What is this?

**Slack MCP Server** connects AI assistants (like Claude Desktop) to Slack workspaces through the Model Context Protocol (MCP). It provides:

- **8 MCP Tools**: Send messages, read channels, search users, manage cache
- **SQLite Caching**: FTS5 full-text search with WAL mode for concurrency
- **User Name Enrichment**: Automatically resolves user IDs to names
- **Token Efficiency**: Optimized response format (~135 tokens per message)

### What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io) is an open standard enabling AI assistants to securely interact with external tools and data sources.

---

## Key Features

### Performance
- **SQLite WAL mode** for concurrent read access
- **FTS5 full-text search** indexes for users and channels
- **Automatic cache refresh** on startup if stale
- **r2d2 connection pooling** with configurable idle connections
- **Snappy compression** for efficient storage

### Search Capabilities
- **User search** by name, email, display name (FTS5 + fuzzy fallback)
- **Channel search** across all types (public/private/DM/multi-DM)
- **Message search** with Slack API (requires user token)

### Messaging
- **Send to** channels, DMs, and threads
- **Read** channel history and threaded conversations
- **User name resolution** for all messages
- **Token-efficient format** (excludes Block Kit structures and attachments)

### Production Ready
- **Governor** token bucket rate limiting with exponential backoff
- **Distributed locking** with timeout and stale lock detection
- **Comprehensive error handling** with typed errors
- **Async/await** throughout with Tokio runtime

---

## Quick Start

### Prerequisites
- Rust 1.90+ (2024 edition)
- Slack workspace with admin access
- Claude Desktop or MCP-compatible client

### 1. Install

```bash
git clone https://github.com/yourusername/mcp-slack
cd mcp-slack
cargo build --release
# Binary: target/release/mcp-slack
```

### 2. Create Slack App

1. Visit [api.slack.com/apps](https://api.slack.com/apps) → **Create New App** → **From scratch**
2. Add **Bot Token Scopes** (see [Required Scopes](#required-scopes)):
   - `channels:read`, `channels:history`
   - `groups:read`, `groups:history`
   - `im:read`, `im:history`
   - `mpim:read`, `mpim:history`
   - `users:read`, `users:read.email`
   - `chat:write`, `chat:write.public`
3. Install app to workspace
4. Copy **Bot User OAuth Token** (`xoxb-...`)

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

---

## Available Tools

### Messaging Tools (4)

#### `send_message`
Send messages to channels, users, or threads.

**Parameters:**
- `channel` (required): Channel name (#general), ID (C1234), or username (@user)
- `text` (required): Message text (supports Slack markdown)
- `thread_ts` (optional): Thread timestamp for replies

#### `get_channel_messages`
Retrieve channel message history with pagination.

**Parameters:**
- `channel` (required): Channel name or ID
- `limit` (optional): Messages to return (default: 100, max: 1000)
- `cursor` (optional): Pagination cursor

**Returns:** Messages with user_id and user_name. Block Kit structures and attachments excluded.

#### `read_thread`
Get complete thread conversation.

**Parameters:**
- `channel` (required): Channel ID
- `thread_ts` (required): Thread parent timestamp

**Returns:** Optimized format with parent info once, followed by replies array.

#### `list_channel_members`
Get channel members with user details.

**Parameters:**
- `channel` (required): Channel name or ID

---

### Search Tools (3)

#### `search_users`
Find users with FTS5 full-text search and fuzzy fallback.

**Parameters:**
- `query` (required): Search term
- `limit` (optional): Max results (default: 10)

**Searches:** username, display name, real name, email

#### `search_channels`
Find channels by name (all types).

**Parameters:**
- `query` (required): Channel name search
- `limit` (optional): Max results (default: 10)

#### `search_messages`
Search workspace messages (requires user token).

**Parameters:**
- `query` (required): Search query (Slack search syntax)
- `channel` (optional): Limit to specific channel
- `from_user` (optional): Filter by user
- `limit` (optional): Max results (default: 10)

**Note:** Requires `SLACK_USER_TOKEN` with `search:read` scope.

---

### System Tools (1)

#### `refresh_cache`
Manually refresh SQLite cache.

**Parameters:**
- `scope` (optional): `"users"`, `"channels"`, or `"all"` (default)

---

## Configuration

### Environment Variables

```bash
# Required (at least one)
SLACK_BOT_TOKEN=xoxb-...      # Bot token
SLACK_USER_TOKEN=xoxp-...     # User token (for message search)

# Optional
DATA_PATH=~/.mcp-slack         # Database location
LOG_LEVEL=warn                 # error | warn | info | debug | trace
RUST_LOG=mcp_slack=debug      # Module-specific logging
```

### Configuration File (Optional)

Create `config.toml` in project root or `~/.mcp-slack/config.toml`:

```toml
[slack]
bot_token = "xoxb-..."
user_token = "xoxp-..."

[cache]
data_path = "~/.mcp-slack"
ttl_users_hours = 24           # User cache TTL
ttl_channels_hours = 24        # Channel cache TTL
ttl_members_hours = 12         # Member list TTL
compression = "snappy"         # SQLite compression

[retry]
max_attempts = 3
initial_delay_ms = 1000
max_delay_ms = 60000           # 60 seconds max backoff
exponential_base = 2.0

[connection]
timeout_seconds = 30
max_idle_per_host = 10         # HTTP connection pool size
pool_idle_timeout_seconds = 90
```

### Required Scopes

#### Bot Token Scopes

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
```

#### User Token Scopes (Optional)

For `search_messages` tool:
```
search:read        - Search workspace messages
```

---

## Architecture

### System Overview

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│   Claude    │  MCP    │   Slack MCP      │  HTTPS  │    Slack    │
│  Desktop    │◄───────►│     Server       │◄───────►│  Workspace  │
│             │ stdio   │   (Rust/Tokio)   │         │             │
└─────────────┘         └────────┬─────────┘         └─────────────┘
                                 │
                                 ▼
                          ┌────────────┐
                          │   SQLite   │
                          │   Cache    │
                          │ (WAL+FTS5) │
                          └────────────┘
```

### Component Structure

```
src/
├── mcp/                    # MCP Protocol Layer
│   ├── server.rs          # JSON-RPC stdio server
│   ├── handlers.rs        # Tool routing
│   └── types.rs           # MCP types
│
├── slack/                  # Slack API Client
│   ├── client.rs          # Unified facade
│   ├── core.rs            # HTTP + rate limiting
│   ├── users.rs           # User operations
│   ├── channels.rs        # Channel operations
│   ├── messages.rs        # Message operations
│   └── types.rs           # Slack data models
│
├── cache/                  # SQLite Cache
│   ├── sqlite_cache.rs    # Main implementation
│   ├── schema.rs          # Tables + FTS5
│   ├── users.rs           # User caching
│   ├── channels.rs        # Channel caching
│   ├── locks.rs           # Distributed locking
│   └── helpers.rs         # Utilities
│
└── tools/                  # MCP Tools (8 total)
    ├── search.rs          # 3 search tools
    ├── messages.rs        # 4 message tools
    ├── cache.rs           # 1 cache tool
    └── message_utils.rs   # Formatting utilities
```

### Technical Stack

- **Language**: Rust 2024 edition (1.90+)
- **Runtime**: Tokio 1.47
- **Database**: SQLite with FTS5 (rusqlite 0.32, r2d2 0.8)
- **HTTP**: reqwest 0.12 with rustls
- **Rate Limiting**: governor 0.8
- **Serialization**: serde 1.0, serde_json 1.0

---

## Troubleshooting

### Cache Not Refreshing

**Symptom:** Old data showing up

**Solution:**
```bash
# Delete cache database
rm ~/.mcp-slack/cache.db

# Restart Claude Desktop
# Cache auto-initializes on startup
```

### "Unauthorized" or "Invalid Token"

**Symptom:** `Error: Unauthorized - check Slack token`

**Solutions:**
1. Verify token starts with `xoxb-` (bot) or `xoxp-` (user)
2. Check all required scopes are added
3. Reinstall Slack app if scopes changed
4. Test token:
```bash
curl -H "Authorization: Bearer xoxb-YOUR-TOKEN" \
  https://slack.com/api/auth.test
```

### Message Search Returns Empty

**Symptom:** `search_messages` finds nothing

**Solution:**
- Ensure `SLACK_USER_TOKEN` is set
- Verify token has `search:read` scope
- Bot tokens cannot search messages

### "database is locked"

**Symptom:** SQLite lock errors

**Solution:**
```bash
# Check for stale locks
sqlite3 ~/.mcp-slack/cache.db "SELECT * FROM locks;"

# Remove stale locks (older than 30s)
sqlite3 ~/.mcp-slack/cache.db \
  "DELETE FROM locks WHERE created_at < unixepoch() - 30;"
```

### Debug Logging

Enable detailed logs:

```bash
# Full debug output
RUST_LOG=debug cargo run

# Module-specific
RUST_LOG=mcp_slack::cache=debug,mcp_slack::slack=info cargo run
```

### Database Inspection

```bash
sqlite3 ~/.mcp-slack/cache.db

# Useful queries:
.tables                              # List tables
SELECT COUNT(*) FROM users;          # Count cached users
SELECT COUNT(*) FROM channels;       # Count channels
SELECT * FROM metadata;              # Check sync times
SELECT * FROM locks;                 # Check active locks
```

---

## Development

### Building from Source

```bash
git clone https://github.com/yourusername/mcp-slack
cd mcp-slack

# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Code quality
cargo fmt
cargo clippy --all-targets -- -D warnings
```

### Project Structure

- **`src/mcp/`** - MCP protocol implementation
- **`src/slack/`** - Slack API client with rate limiting
- **`src/cache/`** - SQLite caching with FTS5
- **`src/tools/`** - MCP tool implementations
- **`src/config.rs`** - Configuration management
- **`src/error.rs`** - Error types

See [CLAUDE.md](CLAUDE.md) for detailed developer documentation.

---

## License

MIT License - see [LICENSE](LICENSE) file.

---

## Support

- **Documentation**: [CLAUDE.md](CLAUDE.md) for developers
- **Issues**: [GitHub Issues](https://github.com/yourusername/mcp-slack/issues)

---

**Version 0.1.0** • Built with Rust 2024 Edition • [MCP Protocol](https://modelcontextprotocol.io)
