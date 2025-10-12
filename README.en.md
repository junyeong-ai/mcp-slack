# Slack MCP Server

[![CI](https://github.com/junyeong-ai/mcp-slack/workflows/CI/badge.svg)](https://github.com/junyeong-ai/mcp-slack/actions)
[![Lint](https://github.com/junyeong-ai/mcp-slack/workflows/Lint/badge.svg)](https://github.com/junyeong-ai/mcp-slack/actions)
[![Rust](https://img.shields.io/badge/rust-1.90%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05%20%7C%202025--06--18-blue?style=flat-square)](https://modelcontextprotocol.io)
[![Tools](https://img.shields.io/badge/MCP%20tools-8-blue?style=flat-square)](#-8-powerful-tools)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/junyeong-ai/mcp-slack/releases)

> **🌐 [한국어](README.md)** | **English**

---

> **🤖 Reduce Your AI Agent's Slack Costs by 60%**
>
> Connect Claude and ChatGPT to Slack with:
> - 💰 **60% token cost reduction** (350 → 135 tokens/msg)
> - ⚡ **50x faster search** (FTS5 < 10ms)
> - 🎯 **Natural language Slack control** (channels, DMs, threads)

---

## 💰 Why Use This MCP Server?

### Solves the Slack Token Cost Problem for AI Agents

Standard Slack API responses are bloated with Block Kit structures, attachment metadata, and unnecessary data that cause AI Agent token costs to skyrocket.

**This MCP Server is optimized for AI Agents:**

**Token Savings (Estimated):**

| Metric | Standard Slack API | Slack MCP Server | 📈 Improvement |
|--------|-------------------|------------------|---------------|
| **Tokens/Message** | ~350 tokens | **~135 tokens** | 🔥 **60% ↓** |
| **Tokens/User** | ~50 tokens | **~20 tokens** | 🔥 **60% ↓** |
| **Search Speed** | API call (~500ms) | **Local FTS5 (<10ms)** | ⚡ **50x faster** |
| **Cost (1K msgs)** | $2.50 (Claude Sonnet) | **$1.00 (estimated)** | 💰 **$1.50 saved** |

> **📊 Calculation basis**: Removes Block Kit structures, attachment metadata, and empty values + auto-converts user names to dramatically reduce response size.
>
> <details>
> <summary><b>View Detailed Comparison</b></summary>
>
> **Standard Slack API Response (350 tokens estimated):**
> ```json
> {
>   "text": "Hello team!",
>   "user": "U1234567890",
>   "blocks": [{"type": "rich_text", "elements": [...]}],  // ← Unnecessary
>   "attachments": [{"fallback": "..."}],                  // ← Unnecessary
>   "metadata": {"event_type": "..."},                     // ← Unnecessary
>   "bot_id": "",                                          // ← Empty
>   "app_id": null,                                        // ← Null
>   // ... 20+ more fields
> }
> ```
>
> **Slack MCP Server Response (135 tokens estimated):**
> ```json
> {
>   "text": "Hello team!",
>   "user_id": "U1234567890",
>   "user_name": "John Smith",  // ← Name instead of ID (AI-friendly)
>   "ts": "1234567890.123456",
>   "timestamp": "2024-01-15T10:30:00Z"
>   // blocks, attachments, empty values all removed
> }
> ```
>
> **Result:** ~215 tokens saved (approximately 61% reduction)
>
> </details>
>
> Actual savings may vary based on message complexity.

**How is this possible?**
- ✅ User ID → Name auto-conversion (easier for AI to understand)
- ✅ Block Kit structure removal (complex UI data)
- ✅ Attachment metadata removal
- ✅ Thread info optimization (parent info once only)
- ✅ Empty string/null value removal
- ✅ SQLite FTS5 local cache minimizes API calls ([Learn more](#why-caching-is-essential))

---

## 💬 What Your AI Agent Can Do

### Real-World Examples

```
👤 User: "Find users named 'John' in Slack"
🤖 AI:   *Searches local FTS5 cache in milliseconds*
         "Found 2 users: John Smith (@john.smith), John Doe (@j.doe)"

👤 User: "Send 'Hello team!' to #general"
🤖 AI:   *Auto-converts channel ID and sends*
         "Message successfully sent to #general!"

👤 User: "Summarize the last 10 messages in #dev-team"
🤖 AI:   *Reads messages + auto-converts user names + summarizes*
         "Over the past 2 hours, 3 people discussed bug fixes:
         - John Smith: Proposed API response time improvements
         - Jane Doe: Completed database query optimization
         - Bob Wilson: Requested additional test cases"

👤 User: "Find messages containing 'deadline'"
🤖 AI:   *Searches entire workspace*
         "Found 15 messages across 3 channels..."
```

### 🛠️ 8 Powerful Tools

Tools available to AI Agents:

**📨 Messaging (4)**
- `send_message` - Send to channels, DMs, threads
- `get_channel_messages` - Read channel history (with pagination)
- `read_thread` - Read thread conversations (optimized format)
- `list_channel_members` - List channel members (with details)

**🔍 Search (3)**
- `search_users` - User search (FTS5 + fuzzy matching)
- `search_channels` - Channel search (all types)
- `search_messages` - Message search (workspace-wide)

**⚙️ System (1)**
- `refresh_cache` - Manual cache refresh

---

## 🎯 Real-World Use Cases

**Wondering how AI Agents are actually used?** Here are real-world work scenarios.

### Team Collaboration 🤝
```
"Summarize today's messages in #meeting in 3 lines"
→ Auto-generate meeting minutes

"Find recent discussions in #dev-team containing 'API'"
→ Track technical discussions

"Check @john's recent messages and tell me what project he's working on"
→ Understand team member workload
```

### Customer Support 💬
```
"Find all messages containing the word 'refund'"
→ Search customer issues

"Find unresolved conversations in #support channel"
→ Track pending issues

"Analyze what problem was mentioned most today"
→ Customer feedback analysis
```

### Automation 🤖
```
Every Monday morning: "Post last week's key topics summary to #weekly-report"
→ Automated weekly reports

On keyword detection: "DM @manager when the word 'urgent' appears"
→ Real-time alert automation

Project kickoff: "Create new channel, invite team members, and post kickoff message"
→ Workflow automation
```

---

## ⚡ Quick Start (1 minute)

```bash
# 1. Build
git clone https://github.com/junyeong-ai/mcp-slack && cd mcp-slack
cargo build --release

# 2. Generate Slack token (api.slack.com/apps)
# Add User Token Scopes and copy xoxp- token

# 3. Configure Claude Desktop
# ~/.config/Claude/claude_desktop_config.json or
# ~/Library/Application Support/Claude/claude_desktop_config.json (macOS)
{
  "mcpServers": {
    "slack": {
      "command": "/absolute/path/to/mcp-slack/target/release/mcp-slack",
      "env": { "SLACK_USER_TOKEN": "xoxp-your-token-here" }
    }
  }
}

# 4. Restart Claude Desktop → Done! 🎉
```

**💡 Tip:** User token (`xoxp-`) is recommended. It provides more features than bot tokens.

👉 **[Detailed Guide](#-get-started-in-3-steps)** | **[Troubleshooting](#-troubleshooting)**

---

## 🚀 Get Started in 3 Steps

### Prerequisites
- Rust 1.90+ (2024 edition)
- Slack workspace admin access
- Claude Desktop or MCP-compatible client

### 1️⃣ Install (2 min)

```bash
git clone https://github.com/junyeong-ai/mcp-slack
cd mcp-slack
cargo build --release

# Build complete! Binary location:
# target/release/mcp-slack
```

### 2️⃣ Generate Slack Token (2 min)

**Option A: User Token (Recommended) ⭐**

1. Visit **[api.slack.com/apps](https://api.slack.com/apps)** → **"Create New App"** → **"From scratch"**

2. Add **User Token Scopes**:
   ```
   channels:read     channels:history
   groups:read       groups:history
   im:read           im:history
   mpim:read         mpim:history
   users:read        users:read.email
   chat:write        search:read
   ```

3. **"Install to Workspace"** → Install to your account

4. Copy **User OAuth Token** (starts with `xoxp-`)

**Option B: Bot Token (Optional)**

1. Same app creation as above

2. Add **Bot Token Scopes**:
   ```
   channels:read     channels:history
   groups:read       groups:history
   im:read           im:history
   mpim:read         mpim:history
   users:read        users:read.email
   chat:write        chat:write.public
   ```

3. Click **"Install to Workspace"**

4. Copy **Bot User OAuth Token** (starts with `xoxb-`)

### 3️⃣ Configure Claude Desktop (1 min)

**Config file location:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

**Add configuration (user token recommended):**
```json
{
  "mcpServers": {
    "slack": {
      "command": "/absolute/path/to/mcp-slack/target/release/mcp-slack",
      "env": {
        "SLACK_USER_TOKEN": "xoxp-your-user-token-here",
        "LOG_LEVEL": "warn"
      }
    }
  }
}
```

**Or use bot token:**
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

**⚠️ Important**: Use **absolute path** for `command` (not `~/` but `/Users/username/...`)

**✅ Done!** Restart Claude Desktop and ask your AI Agent to perform Slack tasks.

---

## 🛠️ Tool Reference Guide

### 📨 Messaging Tools

#### `send_message` - Send Messages

Enables AI Agents to send messages to channels, DMs, and threads.

**Example prompts:**
```
"Send 'Meeting starting' to #general"
"DM @john saying 'Hello'"
"Reply to this thread with 'Acknowledged'"
```

**Parameters:**
| Parameter | Required | Description | Example |
|-----------|----------|-------------|---------|
| `channel` | ✅ | Channel name, ID, or username | `#general`, `C1234`, `@john` |
| `text` | ✅ | Message content (Slack markdown) | `"Hello *team*"` |
| `thread_ts` | ❌ | Thread timestamp | `"1234567890.123456"` |

---

#### `get_channel_messages` - Read Channel Messages

Retrieves channel message history. Enables AI Agents to understand and summarize past conversations.

**Example prompts:**
```
"Show the last 20 messages in #dev-team"
"Summarize today's messages in #general"
```

**Parameters:**
| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `channel` | ✅ | - | Channel name or ID |
| `limit` | ❌ | 100 | Max 1,000 messages |
| `cursor` | ❌ | - | Pagination cursor |

**Response format (token-optimized):**
```json
{
  "messages": [
    {
      "user_id": "U1234",
      "user_name": "John Smith",    // ← Auto-converted!
      "text": "Message content",
      "ts": "1234567890.123456",
      "timestamp": "2024-01-15T10:30:00Z"
    }
  ],
  "has_more": false
}
```

**Token savings:**
- ❌ Block Kit structure removed
- ❌ Attachment metadata removed
- ✅ User ID → Name auto-conversion
- ✅ Empty values removed

---

#### `read_thread` - Read Thread Conversations

Retrieves complete thread conversations. Parent message info provided once to save tokens.

**Example prompts:**
```
"Show all messages in this thread"
```

**Optimized response:**
```json
{
  "thread_info": {
    "parent_message": { /* Once only */ },
    "reply_count": 5,
    "channel": "C1234"
  },
  "messages": [
    { /* Reply 1 */ },
    { /* Reply 2 */ }
  ]
}
```

**Token savings:** 30% additional savings from parent info deduplication

---

#### `list_channel_members` - List Channel Members

Retrieves all members and their details for a channel.

**Example prompts:**
```
"Who's in the #dev-team channel?"
```

---

### 🔍 Search Tools

#### `search_users` - User Search

Finds users in milliseconds with **FTS5 full-text search**.

**Searches:**
- Username
- Display name
- Real name
- Email

**Example prompts:**
```
"Find users with 'john' in their email"
"Search for someone named 'Kim'"
```

**Parameters:**
| Parameter | Description | Default |
|-----------|-------------|---------|
| `query` | Search term | (required) |
| `limit` | Max results | 10 |

**Performance:**
- 🔍 FTS5 index: typically under 10ms
- 🧠 Fuzzy matching: typo-tolerant
- 💾 Local cache: 0 API calls

---

#### `search_channels` - Channel Search

Searches all channel types (public/private/DM/group DM).

**Example prompts:**
```
"Find all channels containing 'dev'"
```

---

#### `search_messages` - Message Search

Searches messages across the entire workspace.

**Example prompts:**
```
"Find messages containing 'deadline'"
"Search #dev-team for 'bug'"
```

**⚠️ Requirements:** `SLACK_USER_TOKEN` (user token) + `search:read` scope

---

### ⚙️ System Tools

#### `refresh_cache` - Refresh Cache

Manually updates SQLite cache.

**Example prompts:**
```
"Refresh the Slack cache"
```

**Parameters:**
- `scope`: `"users"`, `"channels"`, or `"all"` (default)

**Auto-refresh:** Cache auto-refreshes on server start if stale (default TTL: 24 hours).

---

## ⚙️ Environment Variables

> **💡 Important**: At least one token is required. **User token (`xoxp-`) is recommended**.

### Required Settings

| Variable | Description | Example |
|----------|-------------|---------|
| **`SLACK_USER_TOKEN`** ⭐ | User token (recommended) | `xoxp-...` |
| **`SLACK_BOT_TOKEN`** | Bot token (alternative) | `xoxb-...` |

**Token Comparison:**

| Feature | User Token ⭐ | Bot Token |
|---------|--------------|-----------|
| Channel Access | ✅ Automatic | ⚠️ Invitation required |
| Message Search | ✅ Available | ❌ Unavailable |
| Sender | You | Bot account |

**Generate User Token:**
1. [api.slack.com/apps](https://api.slack.com/apps) → "Create New App" → "From scratch"
2. "OAuth & Permissions" → Add **User Token Scopes**:
   ```
   channels:read channels:history groups:read groups:history
   im:read im:history mpim:read mpim:history
   users:read users:read.email chat:write search:read
   ```
3. "Install to Workspace" → Copy token

---

### Recommended Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `LOG_LEVEL` | `warn` | Log level (`error`, `warn`, `info`, `debug`) |
| `DATA_PATH` | `~/.mcp-slack` | SQLite cache location |

---

<details>
<summary><b>🔧 Advanced Configuration (config.toml)</b></summary>

For finer control, use a config file.

**File location:** `./config.toml` or `~/.mcp-slack/config.toml`

```toml
[slack]
bot_token = "xoxb-..."
user_token = "xoxp-..."

[cache]
data_path = "~/.mcp-slack"
ttl_users_hours = 24        # User cache TTL
ttl_channels_hours = 24     # Channel cache TTL
ttl_members_hours = 12      # Member list TTL
compression = "snappy"      # Compression method

[retry]
max_attempts = 3            # API retry attempts
initial_delay_ms = 1000     # First retry delay
max_delay_ms = 60000        # Max delay time
exponential_base = 2.0      # Exponential backoff multiplier

[connection]
timeout_seconds = 30               # HTTP timeout
max_idle_per_host = 10             # Connection pool size
pool_idle_timeout_seconds = 90     # Idle connection timeout
```

**Configuration Effects:**

| Setting | Default | When to Change | Effect |
|---------|---------|----------------|--------|
| `ttl_users_hours` | 24 | Frequent team changes → 12 | Fresher cache |
| `ttl_channels_hours` | 24 | Many channel creations → 12 | Latest sync |
| `max_attempts` | 3 | Unstable network → 5 | More retries |
| `timeout_seconds` | 30 | Slow network → 60 | Timeout relief |

</details>

---

## 🏗️ Tech Stack & Architecture

### System Architecture

```mermaid
graph LR
    A[Claude Desktop<br/>AI Agent] -->|MCP stdio| B[Slack MCP Server<br/>Rust/Tokio]
    B -->|HTTPS API| C[Slack Workspace]
    B -->|Read/Write| D[(SQLite Cache<br/>WAL + FTS5)]
    D -.->|< 10ms| B
    C -.->|~500ms| B

    subgraph Cache
        D
        E[User FTS5]
        F[Channel FTS5]
        G[Distributed Lock]
        H[Metadata]
    end
```

<details>
<summary>View ASCII Diagram</summary>

```
┌──────────────┐         ┌──────────────────┐         ┌─────────────┐
│   Claude     │   MCP   │   Slack MCP      │  HTTPS  │    Slack    │
│   Desktop    │◄───────►│     Server       │◄───────►│  Workspace  │
│ (AI Agent)   │  stdio  │   (Rust/Tokio)   │         │             │
└──────────────┘         └─────────┬────────┘         └─────────────┘
                                   │
                                   ▼
                          ┌─────────────────┐
                          │   SQLite Cache  │
                          │   (WAL + FTS5)  │
                          │                 │
                          │ • User FTS5     │
                          │ • Channel FTS5  │
                          │ • Distributed   │
                          │   locking       │
                          │ • Metadata      │
                          └─────────────────┘
```

</details>

### Why Caching is Essential?

Local caching is critical to overcome Slack API limitations:

**📉 Slack API Limitations:**
- **No Channel Search**: `conversations.list` API doesn't support search by channel name
- **Low Rate Limit**: Tier 2 (20 calls/min) - bottleneck for large workspaces
- **Repeated Query Inefficiency**: Querying user/channel info via API every time causes slowdowns

**✅ Caching Solution:**
- 🚀 **Full Load on Startup**: Cache all users and channels at once
- 🔍 **FTS5 Full-Text Search**: Local channel/user search in milliseconds
- ⚡ **Instant Response**: Direct cache queries without API calls
- 🔄 **Auto-Refresh**: TTL-based (24 hours) automatic updates

**Performance Comparison:**
| Operation | Slack API | Cache (FTS5) | Improvement |
|-----------|-----------|--------------|-------------|
| User Search | ~500ms + rate limit | **<10ms** | **50x+ faster** |
| Channel Search | ❌ Unavailable | **<10ms** | **Enabled** |
| Consecutive Queries | Rate limit restricted | **Unlimited** | **No constraints** |

---

### Core Technologies

**AI Agent Optimization:**
- **SQLite FTS5**: Full-text search engine for millisecond queries
- **WAL Mode**: Write-Ahead Logging for concurrent reads
- **Snappy Compression**: Minimizes cache storage
- **Governor Rate Limiting**: Token bucket for automatic API limit handling
- **Full Data Caching**: Load all user/channel info on server startup

**Token Optimization:**
- **User Name Enrichment**: ID → Name auto-conversion
- **Block Kit Removal**: Eliminates unnecessary UI structures
- **Null/Empty Filtering**: Auto-removes empty values
- **Thread Optimization**: Deduplicates parent info

**Performance:**
- **Rust 2024 Edition**: Memory safety + high performance
- **Tokio Async Runtime**: Async I/O for concurrent operations
- **r2d2 Connection Pool**: HTTP connection reuse
- **Exponential Backoff**: Automatic retry

### Tech Stack

| Category | Technology | Version | Purpose |
|----------|------------|---------|---------|
| **Language** | Rust | 2024 Edition | Systems programming |
| **Runtime** | Tokio | 1.47 | Async processing |
| **Database** | SQLite | 3.x | Embedded cache |
| **Search** | FTS5 | - | Full-text search |
| **HTTP** | reqwest | 0.12 | HTTP client |
| **Rate Limiting** | governor | 0.8 | Token bucket |
| **Serialization** | serde | 1.0 | JSON processing |

---

## 🔧 Troubleshooting

### Cache Not Refreshing

**Symptom:** New team members or channels not showing

**Solution:**
```bash
# 1. Delete cache
rm ~/.mcp-slack/cache.db

# 2. Restart Claude Desktop
# Cache will auto-recreate
```

**Using AI Agent:**
> "Refresh the Slack cache"

---

### "Unauthorized" Error

**Symptom:** `Error: Unauthorized - check Slack token`

**Checklist:**
- [ ] Check token format: User token (`xoxp-`) or Bot token (`xoxb-`)
- [ ] All required scopes added
- [ ] Reinstalled to workspace after scope changes

**Test token:**
```bash
# Test user token
curl -H "Authorization: Bearer xoxp-YOUR-TOKEN" \
  https://slack.com/api/auth.test

# Test bot token
curl -H "Authorization: Bearer xoxb-YOUR-TOKEN" \
  https://slack.com/api/auth.test
```

---

### Message Search Not Working

**Symptom:** `search_messages` returns empty results

**Cause:** Missing user token or insufficient permissions

**Solution:**
1. Set `SLACK_USER_TOKEN` (starts with `xoxp-`)
2. Add `search:read` scope
3. Reinstall to workspace

> 💡 Bot tokens (`xoxb-`) cannot search messages

---

### Debug Logging

**Claude Desktop config:**
```json
{
  "mcpServers": {
    "slack": {
      "command": "/path/to/mcp-slack",
      "env": {
        "SLACK_USER_TOKEN": "xoxp-...",  // or SLACK_BOT_TOKEN
        "LOG_LEVEL": "debug"    // ← Change
      }
    }
  }
}
```

**From terminal:**
```bash
RUST_LOG=debug cargo run
RUST_LOG=mcp_slack::cache=debug cargo run  # Module-specific
```

---

### Inspect Cache Data

```bash
sqlite3 ~/.mcp-slack/cache.db

# Useful queries
SELECT COUNT(*) FROM users;          # User count
SELECT COUNT(*) FROM channels;       # Channel count
SELECT * FROM metadata;              # Last sync times

# Cache freshness
SELECT
    key,
    datetime(CAST(value AS INTEGER), 'unixepoch') as last_sync,
    (unixepoch() - CAST(value AS INTEGER)) / 3600 as hours_ago
FROM metadata
WHERE key LIKE 'last_%_sync';
```

---

## 🚀 Development

### Building

```bash
git clone https://github.com/junyeong-ai/mcp-slack
cd mcp-slack

cargo build                # Development build
cargo build --release      # Optimized build
cargo test                 # Run tests
cargo clippy              # Lint
```

### Project Structure

```
src/
├── mcp/                 # MCP Protocol
│   ├── server.rs       # JSON-RPC stdio
│   ├── handlers.rs     # Tool routing
│   └── types.rs        # MCP types
│
├── slack/              # Slack Client
│   ├── client.rs       # Unified facade
│   ├── core.rs         # HTTP + Rate Limiting
│   ├── users.rs        # User operations
│   ├── channels.rs     # Channel operations
│   └── messages.rs     # Message operations
│
├── cache/              # SQLite Cache
│   ├── sqlite_cache.rs # Main implementation
│   ├── schema.rs       # FTS5 schema
│   ├── users.rs        # User caching
│   ├── channels.rs     # Channel caching
│   └── locks.rs        # Distributed locking
│
└── tools/              # MCP Tools (8)
    ├── search.rs       # Search tools (3)
    ├── messages.rs     # Messaging (4)
    ├── cache.rs        # Cache (1)
    └── message_utils.rs # Token optimization
```

**Developer docs:** [CLAUDE.md](CLAUDE.md)

---

## 📄 License

MIT License - [LICENSE](LICENSE)

---

## 💬 Support

- **GitHub Issues**: [Report issues](https://github.com/junyeong-ai/mcp-slack/issues)
- **Developer docs**: [CLAUDE.md](CLAUDE.md)
- **MCP Protocol**: [modelcontextprotocol.io](https://modelcontextprotocol.io)

---

<div align="center">

**🌐 [한국어](README.md)** | **English**

**Version 0.1.0** • Built with Rust 2024 Edition

Made with ❤️ for AI Agents

</div>
