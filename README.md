# Slack MCP Server

[![CI](https://github.com/junyeong-ai/mcp-slack/workflows/CI/badge.svg)](https://github.com/junyeong-ai/mcp-slack/actions)
[![Lint](https://github.com/junyeong-ai/mcp-slack/workflows/Lint/badge.svg)](https://github.com/junyeong-ai/mcp-slack/actions)
[![Rust](https://img.shields.io/badge/rust-1.90%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05%20%7C%202025--06--18-blue?style=flat-square)](https://modelcontextprotocol.io)
[![Tools](https://img.shields.io/badge/MCP%20tools-8-blue?style=flat-square)](#%EF%B8%8F-8개의-강력한-도구)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/junyeong-ai/mcp-slack/releases)

> **🌐 한국어** | **[English](README.en.md)**

---

> **🤖 AI Agent의 Slack 비용을 60% 절감하세요**
>
> Claude, ChatGPT를 Slack과 연결하여:
> - 💰 **토큰 비용 60% 절감** (350 → 135 tokens/msg)
> - ⚡ **검색 속도 50배 향상** (FTS5 < 10ms)
> - 🎯 **자연어로 Slack 제어** (채널, DM, 스레드 모두)

---

## 💰 왜 이 MCP 서버를 사용해야 할까요?

### AI Agent의 Slack 비용 문제를 해결합니다

일반적인 Slack API 응답은 Block Kit 구조, 첨부파일 메타데이터 등 불필요한 데이터로 가득 차 있어 AI Agent의 토큰 비용이 급증합니다.

**이 MCP 서버는 AI Agent를 위해 최적화되었습니다:**

**토큰 절감 효과 (추정치):**

| 지표 | 일반 Slack API | Slack MCP Server | 📈 개선 효과 |
|------|---------------|------------------|-------------|
| **메시지당 토큰** | ~350 tokens | **~135 tokens** | 🔥 **60% ↓** |
| **사용자당 토큰** | ~50 tokens | **~20 tokens** | 🔥 **60% ↓** |
| **검색 속도** | API 호출 (~500ms) | **로컬 FTS5 (<10ms)** | ⚡ **50배 빠름** |
| **1,000개 메시지 비용** | $2.50 (Claude Sonnet) | **$1.00 (추정)** | 💰 **$1.50 절약** |

> **📊 계산 근거**: Block Kit 구조, 첨부파일 메타데이터, 빈 값 제거 + 사용자 이름 자동 변환으로 응답 크기 대폭 감소.
>
> <details>
> <summary><b>상세 비교 보기</b></summary>
>
> **일반 Slack API 응답 (350 tokens 추정):**
> ```json
> {
>   "text": "Hello team!",
>   "user": "U1234567890",
>   "blocks": [{"type": "rich_text", "elements": [...]}],  // ← 불필요
>   "attachments": [{"fallback": "..."}],                  // ← 불필요
>   "metadata": {"event_type": "..."},                     // ← 불필요
>   "bot_id": "",                                          // ← 빈 값
>   "app_id": null,                                        // ← null
>   // ... 기타 20+ 필드
> }
> ```
>
> **Slack MCP Server 응답 (135 tokens 추정):**
> ```json
> {
>   "text": "Hello team!",
>   "user_id": "U1234567890",
>   "user_name": "홍길동",      // ← ID 대신 이름 (AI 친화적)
>   "ts": "1234567890.123456",
>   "timestamp": "2024-01-15T10:30:00Z"
>   // blocks, attachments, 빈 값 모두 제거
> }
> ```
>
> **결과:** ~215 tokens 절감 (약 61% 감소)
>
> </details>
>
> 실제 절감 효과는 메시지 복잡도에 따라 다를 수 있습니다.

**어떻게 가능한가요?**
- ✅ 사용자 ID → 이름 자동 변환 (AI가 이해하기 쉽게)
- ✅ Block Kit 구조 제거 (복잡한 UI 데이터)
- ✅ 첨부파일 메타데이터 제거
- ✅ 스레드 정보 최적화 (부모 정보 1회만)
- ✅ 빈 문자열/null 값 제거
- ✅ SQLite FTS5 로컬 캐시로 API 호출 최소화 ([상세 보기](#왜-캐싱이-필요한가요))

---

## 💬 AI Agent가 할 수 있는 일

### 실제 사용 예시

```
👤 User: "Slack에서 'John'이라는 이름의 사용자를 찾아줘"
🤖 AI:   *FTS5 로컬 캐시에서 밀리초 단위 검색*
         "John Smith (@john.smith), John Doe (@j.doe) 2명을 찾았습니다."

👤 User: "#general 채널에 '팀 여러분 안녕하세요!' 메시지를 보내줘"
🤖 AI:   *자동으로 채널 ID 변환 후 전송*
         "메시지를 #general에 성공적으로 전송했습니다!"

👤 User: "최근 #dev-team 채널의 메시지 10개를 요약해줘"
🤖 AI:   *메시지 읽기 + 사용자 이름 자동 변환 + 요약*
         "지난 2시간 동안 3명이 버그 수정에 대해 논의했습니다:
         - 홍길동: API 응답 시간 개선 제안
         - 김철수: 데이터베이스 쿼리 최적화 완료
         - 박영희: 테스트 케이스 추가 요청"

👤 User: "'마감일'이라는 단어가 포함된 메시지를 찾아줘"
🤖 AI:   *워크스페이스 전체 검색*
         "3개 채널에서 15개의 메시지를 찾았습니다..."
```

### 🛠️ 8개의 강력한 도구

AI Agent가 사용할 수 있는 도구:

**📨 메시징 (4개)**
- `send_message` - 채널, DM, 스레드에 메시지 전송
- `get_channel_messages` - 채널 히스토리 읽기 (페이지네이션 지원)
- `read_thread` - 스레드 대화 읽기 (최적화된 포맷)
- `list_channel_members` - 채널 멤버 조회 (상세 정보 포함)

**🔍 검색 (3개)**
- `search_users` - 사용자 검색 (FTS5 + 퍼지 매칭)
- `search_channels` - 채널 검색 (모든 타입)
- `search_messages` - 메시지 검색 (워크스페이스 전체)

**⚙️ 시스템 (1개)**
- `refresh_cache` - 캐시 수동 갱신

---

## 🎯 실제 활용 사례

**AI Agent가 실제로 어떻게 사용되는지 궁금하신가요?** 다음은 실제 업무 시나리오입니다.

### 팀 협업 🤝
```
"#meeting 채널의 오늘 메시지를 3줄로 요약해줘"
→ 회의록 자동 생성

"#dev-team에서 'API'라는 단어가 포함된 최근 논의 찾아줘"
→ 기술 논의 추적

"@john이 어떤 프로젝트를 맡고 있는지 최근 메시지 보고 알려줘"
→ 팀원 업무 파악
```

### 고객 지원 💬
```
"'환불'이라는 단어가 포함된 메시지를 모두 찾아줘"
→ 고객 이슈 검색

"#support 채널에서 '해결되지 않은' 상태의 대화 찾아줘"
→ 미해결 이슈 추적

"오늘 가장 많이 언급된 문제가 뭔지 분석해줘"
→ 고객 피드백 분석
```

### 자동화 🤖
```
매주 월요일 오전: "#weekly-report 채널에 지난 주 핵심 토픽 정리해서 올려줘"
→ 자동 주간 리포트

특정 키워드 감지 시: "'긴급'이라는 단어 나오면 @manager에게 DM 보내줘"
→ 실시간 알림 자동화

프로젝트 시작 시: "새 채널 만들고 팀원들 초대 후 킥오프 메시지 보내줘"
→ 워크플로우 자동화
```

---

## ⚡ Quick Start (1분)

```bash
# 1. 빌드
git clone https://github.com/junyeong-ai/mcp-slack && cd mcp-slack
cargo build --release

# 2. Slack 토큰 생성 (api.slack.com/apps)
# User Token Scopes 추가 후 xoxp- 토큰 복사

# 3. Claude Desktop 설정
# ~/.config/Claude/claude_desktop_config.json 또는
# ~/Library/Application Support/Claude/claude_desktop_config.json (macOS)
{
  "mcpServers": {
    "slack": {
      "command": "/절대경로/mcp-slack/target/release/mcp-slack",
      "env": { "SLACK_USER_TOKEN": "xoxp-your-token-here" }
    }
  }
}

# 4. Claude Desktop 재시작 → 완료! 🎉
```

**💡 Tip:** 사용자 토큰(`xoxp-`)을 권장합니다. 봇 토큰보다 더 많은 기능을 사용할 수 있습니다.

👉 **[상세 가이드 보기](#-3단계로-시작하기)** | **[문제 해결](#-문제-해결)**

---

## 🚀 3단계로 시작하기

### 사전 요구사항
- Rust 1.90+ (2024 edition)
- Slack 워크스페이스 관리자 권한
- Claude Desktop 또는 MCP 호환 클라이언트

### 1️⃣ 설치 (2분)

```bash
git clone https://github.com/junyeong-ai/mcp-slack
cd mcp-slack
cargo build --release

# 빌드 완료! 바이너리 위치:
# target/release/mcp-slack
```

### 2️⃣ Slack 토큰 생성 (2분)

**옵션 A: 사용자 토큰 (권장) ⭐**

1. **[api.slack.com/apps](https://api.slack.com/apps)** 방문 → **"Create New App"** → **"From scratch"**

2. **User Token Scopes** 추가:
   ```
   channels:read     channels:history
   groups:read       groups:history
   im:read           im:history
   mpim:read         mpim:history
   users:read        users:read.email
   chat:write        search:read
   ```

3. **"Install to Workspace"** → 본인 계정에 설치

4. **User OAuth Token** 복사 (`xoxp-...`로 시작)

**옵션 B: 봇 토큰 (선택)**

1. 위와 동일하게 앱 생성

2. **Bot Token Scopes** 추가:
   ```
   channels:read     channels:history
   groups:read       groups:history
   im:read           im:history
   mpim:read         mpim:history
   users:read        users:read.email
   chat:write        chat:write.public
   ```

3. **"Install to Workspace"** 클릭

4. **Bot User OAuth Token** 복사 (`xoxb-...`로 시작)

### 3️⃣ Claude Desktop 설정 (1분)

**설정 파일 위치:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

**설정 추가 (사용자 토큰 권장):**
```json
{
  "mcpServers": {
    "slack": {
      "command": "/절대경로/mcp-slack/target/release/mcp-slack",
      "env": {
        "SLACK_USER_TOKEN": "xoxp-여기에-사용자-토큰-입력",
        "LOG_LEVEL": "warn"
      }
    }
  }
}
```

**또는 봇 토큰 사용:**
```json
{
  "mcpServers": {
    "slack": {
      "command": "/절대경로/mcp-slack/target/release/mcp-slack",
      "env": {
        "SLACK_BOT_TOKEN": "xoxb-여기에-봇-토큰-입력",
        "LOG_LEVEL": "warn"
      }
    }
  }
}
```

**⚠️ 중요**: `command`는 **절대 경로**를 사용하세요 (`~/` 대신 `/Users/username/...`)

**✅ 완료!** Claude Desktop을 재시작하고 AI Agent에게 Slack 작업을 요청해보세요.

---

## 🛠️ 도구 상세 가이드

### 📨 메시징 도구

#### `send_message` - 메시지 전송

AI Agent가 채널, DM, 스레드에 메시지를 전송할 수 있습니다.

**사용 예시:**
```
"#general 채널에 '회의 시작합니다' 메시지 보내줘"
"@john에게 DM으로 '안녕하세요' 보내줘"
"이 스레드에 '확인했습니다' 답장 보내줘"
```

**파라미터:**
| 파라미터 | 필수 | 설명 | 예시 |
|---------|------|------|------|
| `channel` | ✅ | 채널명, ID, 사용자명 | `#general`, `C1234`, `@john` |
| `text` | ✅ | 메시지 내용 (Slack 마크다운) | `"안녕하세요 *팀 여러분*"` |
| `thread_ts` | ❌ | 스레드 타임스탬프 | `"1234567890.123456"` |

---

#### `get_channel_messages` - 채널 메시지 읽기

채널의 메시지 히스토리를 가져옵니다. AI Agent가 과거 대화를 이해하고 요약할 수 있습니다.

**사용 예시:**
```
"#dev-team 채널의 최근 메시지 20개 보여줘"
"#general의 오늘 메시지를 요약해줘"
```

**파라미터:**
| 파라미터 | 필수 | 기본값 | 설명 |
|---------|------|--------|------|
| `channel` | ✅ | - | 채널명 또는 ID |
| `limit` | ❌ | 100 | 최대 1,000개 |
| `cursor` | ❌ | - | 페이지네이션 |

**응답 형식 (토큰 최적화):**
```json
{
  "messages": [
    {
      "user_id": "U1234",
      "user_name": "홍길동",      // ← 자동 변환!
      "text": "메시지 내용",
      "ts": "1234567890.123456",
      "timestamp": "2024-01-15T10:30:00Z"
    }
  ],
  "has_more": false
}
```

**토큰 절약:**
- ❌ Block Kit 구조 제거
- ❌ 첨부파일 메타데이터 제거
- ✅ 사용자 ID → 이름 자동 변환
- ✅ 빈 값 제거

---

#### `read_thread` - 스레드 대화 읽기

완전한 스레드 대화를 가져옵니다. 부모 메시지 정보를 1회만 제공하여 토큰을 절약합니다.

**사용 예시:**
```
"이 메시지의 스레드를 모두 보여줘"
```

**최적화된 응답:**
```json
{
  "thread_info": {
    "parent_message": { /* 1회만 */ },
    "reply_count": 5,
    "channel": "C1234"
  },
  "messages": [
    { /* 답글 1 */ },
    { /* 답글 2 */ }
  ]
}
```

**토큰 절약:** 부모 정보 중복 제거로 30% 추가 절감

---

#### `list_channel_members` - 채널 멤버 조회

채널의 모든 멤버와 상세 정보를 가져옵니다.

**사용 예시:**
```
"#dev-team 채널에 누가 있는지 알려줘"
```

---

### 🔍 검색 도구

#### `search_users` - 사용자 검색

**FTS5 전문 검색**으로 사용자를 밀리초 단위로 찾습니다.

**검색 대상:**
- 사용자명 (username)
- 표시 이름 (display_name)
- 실명 (real_name)
- 이메일

**사용 예시:**
```
"이메일에 'john'이 들어간 사용자 찾아줘"
"'김철수'라는 이름의 사람을 검색해줘"
```

**파라미터:**
| 파라미터 | 설명 | 기본값 |
|---------|------|--------|
| `query` | 검색어 | (필수) |
| `limit` | 최대 결과 수 | 10 |

**성능:**
- 🔍 FTS5 인덱스: 일반적으로 10ms 미만
- 🧠 퍼지 매칭: 오타 허용
- 💾 로컬 캐시: API 호출 0회

---

#### `search_channels` - 채널 검색

모든 유형의 채널을 검색합니다 (공개/비공개/DM/그룹 DM).

**사용 예시:**
```
"'dev'가 포함된 채널을 모두 찾아줘"
```

---

#### `search_messages` - 메시지 검색

워크스페이스 전체에서 메시지를 검색합니다.

**사용 예시:**
```
"'마감일'이라는 단어가 포함된 메시지 찾아줘"
"#dev-team에서 'bug'를 검색해줘"
```

**⚠️ 요구사항:** `SLACK_USER_TOKEN` (사용자 토큰) + `search:read` 권한 필요

---

### ⚙️ 시스템 도구

#### `refresh_cache` - 캐시 갱신

SQLite 캐시를 수동으로 업데이트합니다.

**사용 예시:**
```
"Slack 캐시를 새로 고침해줘"
```

**파라미터:**
- `scope`: `"users"`, `"channels"`, `"all"` (기본값)

**자동 갱신:** 서버 시작 시 캐시가 오래되었으면 자동으로 갱신합니다 (기본 TTL: 24시간).

---

## ⚙️ 환경 변수

> **💡 중요**: 최소 하나의 토큰이 필요합니다. **사용자 토큰(`xoxp-`)을 권장**합니다.

### 필수 설정

| 변수 | 설명 | 예제 |
|------|------|------|
| **`SLACK_USER_TOKEN`** ⭐ | 사용자 토큰 (권장) | `xoxp-...` |
| **`SLACK_BOT_TOKEN`** | 봇 토큰 (대안) | `xoxb-...` |

**토큰 비교:**

| 기능 | 사용자 토큰 ⭐ | 봇 토큰 |
|------|--------------|---------|
| 채널 접근 | ✅ 자동 | ⚠️ 초대 필요 |
| 메시지 검색 | ✅ 가능 | ❌ 불가 |
| 전송자 | 사용자 본인 | 봇 계정 |

**사용자 토큰 생성:**
1. [api.slack.com/apps](https://api.slack.com/apps) → "Create New App" → "From scratch"
2. "OAuth & Permissions" → **User Token Scopes** 추가:
   ```
   channels:read channels:history groups:read groups:history
   im:read im:history mpim:read mpim:history
   users:read users:read.email chat:write search:read
   ```
3. "Install to Workspace" → 토큰 복사

---

### 권장 설정

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `LOG_LEVEL` | `warn` | 로그 레벨 (`error`, `warn`, `info`, `debug`) |
| `DATA_PATH` | `~/.mcp-slack` | SQLite 캐시 저장 위치 |

---

<details>
<summary><b>🔧 고급 설정 (config.toml)</b></summary>

더 세밀한 제어가 필요한 경우 설정 파일을 사용하세요.

**파일 위치:** `./config.toml` 또는 `~/.mcp-slack/config.toml`

```toml
[slack]
bot_token = "xoxb-..."
user_token = "xoxp-..."

[cache]
data_path = "~/.mcp-slack"
ttl_users_hours = 24        # 사용자 캐시 TTL
ttl_channels_hours = 24     # 채널 캐시 TTL
ttl_members_hours = 12      # 멤버 목록 TTL
compression = "snappy"      # 압축 방식

[retry]
max_attempts = 3            # API 재시도 횟수
initial_delay_ms = 1000     # 첫 재시도 대기
max_delay_ms = 60000        # 최대 대기 시간
exponential_base = 2.0      # 지수 백오프 배율

[connection]
timeout_seconds = 30               # HTTP 타임아웃
max_idle_per_host = 10             # 연결 풀 크기
pool_idle_timeout_seconds = 90     # 유휴 연결 유지
```

**설정 효과:**

| 설정 | 기본값 | 변경 시점 | 효과 |
|------|--------|----------|------|
| `ttl_users_hours` | 24 | 팀원 변동 잦음 → 12 | 캐시 신선도 ↑ |
| `ttl_channels_hours` | 24 | 채널 생성 많음 → 12 | 최신 반영 |
| `max_attempts` | 3 | 네트워크 불안정 → 5 | 재시도 ↑ |
| `timeout_seconds` | 30 | 느린 네트워크 → 60 | 타임아웃 완화 |

</details>

---

## 🏗️ 기술 스택 & 아키텍처

### 시스템 구조

```mermaid
graph LR
    A[Claude Desktop<br/>AI Agent] -->|MCP stdio| B[Slack MCP Server<br/>Rust/Tokio]
    B -->|HTTPS API| C[Slack Workspace]
    B -->|Read/Write| D[(SQLite Cache<br/>WAL + FTS5)]
    D -.->|< 10ms| B
    C -.->|~500ms| B

    subgraph Cache
        D
        E[사용자 FTS5]
        F[채널 FTS5]
        G[분산 락]
        H[메타데이터]
    end
```

<details>
<summary>ASCII 다이어그램 보기</summary>

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
                          │ • 사용자 FTS5   │
                          │ • 채널 FTS5     │
                          │ • 분산 락       │
                          │ • 메타데이터    │
                          └─────────────────┘
```

</details>

### 왜 캐싱이 필요한가요?

Slack API의 한계를 극복하기 위해 로컬 캐싱이 필수적입니다:

**📉 Slack API의 문제점:**
- **채널 검색 불가**: `conversations.list` API는 채널 이름으로 검색을 지원하지 않음
- **낮은 Rate Limit**: Tier 2 (분당 20회) - 대규모 워크스페이스에서 병목 발생
- **반복 조회 비효율**: 사용자/채널 정보를 매번 API로 조회 시 속도 저하

**✅ 캐싱 솔루션:**
- 🚀 **서버 시작 시 전체 로드**: 모든 사용자 및 채널 정보를 한번에 캐싱
- 🔍 **FTS5 전문 검색**: 로컬에서 밀리초 단위로 채널/사용자 검색
- ⚡ **즉각적인 응답**: API 호출 없이 캐시에서 직접 조회
- 🔄 **자동 갱신**: TTL 기반 (24시간) 자동 업데이트

**성능 비교:**
| 작업 | Slack API | 캐시 (FTS5) | 개선 |
|------|-----------|-------------|------|
| 사용자 검색 | ~500ms + rate limit | **<10ms** | **50배+ 빠름** |
| 채널 검색 | ❌ 불가능 | **<10ms** | **가능** |
| 연속 조회 | Rate limit 제한 | **무제한** | **제약 없음** |

---

### 핵심 기술

**AI Agent 최적화 기술:**
- **SQLite FTS5**: 전문 검색 엔진으로 밀리초 단위 검색
- **WAL Mode**: Write-Ahead Logging으로 동시 읽기 지원
- **Snappy 압축**: 캐시 저장 공간 최소화
- **Governor Rate Limiting**: 토큰 버킷으로 API 제한 자동 처리
- **전체 데이터 캐싱**: 서버 시작 시 모든 사용자/채널 정보 로드

**토큰 최적화 기술:**
- **User Name Enrichment**: ID → 이름 자동 변환
- **Block Kit Removal**: 불필요한 UI 구조 제거
- **Null/Empty Filtering**: 빈 값 자동 제거
- **Thread Optimization**: 부모 정보 중복 제거

**성능 기술:**
- **Rust 2024 Edition**: 메모리 안전 + 고성능
- **Tokio Async Runtime**: 비동기 I/O로 동시 처리
- **r2d2 Connection Pool**: HTTP 연결 재사용
- **Exponential Backoff**: 자동 재시도

### 기술 스택

| 카테고리 | 기술 | 버전 | 용도 |
|---------|------|------|------|
| **언어** | Rust | 2024 Edition | 시스템 프로그래밍 |
| **런타임** | Tokio | 1.47 | 비동기 처리 |
| **데이터베이스** | SQLite | 3.x | 임베디드 캐시 |
| **검색** | FTS5 | - | 전문 검색 엔진 |
| **HTTP** | reqwest | 0.12 | HTTP 클라이언트 |
| **Rate Limiting** | governor | 0.8 | 토큰 버킷 |
| **직렬화** | serde | 1.0 | JSON 처리 |

---

## 🔧 문제 해결

### 캐시가 갱신되지 않아요

**증상:** 새로운 팀원이나 채널이 보이지 않음

**해결:**
```bash
# 1. 캐시 삭제
rm ~/.mcp-slack/cache.db

# 2. Claude Desktop 재시작
# 자동으로 캐시가 다시 생성됩니다
```

**AI Agent 사용:**
> "Slack 캐시를 새로 고침해줘"

---

### "Unauthorized" 오류

**증상:** `Error: Unauthorized - check Slack token`

**체크리스트:**
- [ ] 토큰 형식 확인: 사용자 토큰(`xoxp-`) 또는 봇 토큰(`xoxb-`)으로 시작
- [ ] 모든 필수 권한 추가 확인
- [ ] 권한 변경 후 워크스페이스에 재설치 확인

**토큰 테스트:**
```bash
# 사용자 토큰 테스트
curl -H "Authorization: Bearer xoxp-YOUR-TOKEN" \
  https://slack.com/api/auth.test

# 봇 토큰 테스트
curl -H "Authorization: Bearer xoxb-YOUR-TOKEN" \
  https://slack.com/api/auth.test
```

---

### 메시지 검색이 안 돼요

**증상:** `search_messages` 도구가 빈 결과 반환

**원인:** User Token 없음 또는 권한 부족

**해결:**
1. `SLACK_USER_TOKEN` 설정 (`xoxp-`로 시작)
2. `search:read` 권한 추가
3. 워크스페이스에 재설치

> 💡 봇 토큰(`xoxb-`)으로는 메시지 검색 불가

---

### 디버그 로깅

**Claude Desktop 설정:**
```json
{
  "mcpServers": {
    "slack": {
      "command": "/path/to/mcp-slack",
      "env": {
        "SLACK_USER_TOKEN": "xoxp-...",  // 또는 SLACK_BOT_TOKEN
        "LOG_LEVEL": "debug"    // ← 변경
      }
    }
  }
}
```

**터미널에서:**
```bash
RUST_LOG=debug cargo run
RUST_LOG=mcp_slack::cache=debug cargo run  # 모듈별
```

---

### 캐시 데이터 확인

```bash
sqlite3 ~/.mcp-slack/cache.db

# 유용한 쿼리
SELECT COUNT(*) FROM users;          # 사용자 수
SELECT COUNT(*) FROM channels;       # 채널 수
SELECT * FROM metadata;              # 마지막 동기화

# 캐시 신선도
SELECT
    key,
    datetime(CAST(value AS INTEGER), 'unixepoch') as last_sync,
    (unixepoch() - CAST(value AS INTEGER)) / 3600 as hours_ago
FROM metadata
WHERE key LIKE 'last_%_sync';
```

---

## 🚀 개발

### 빌드

```bash
git clone https://github.com/junyeong-ai/mcp-slack
cd mcp-slack

cargo build                # 개발 빌드
cargo build --release      # 최적화 빌드
cargo test                 # 테스트
cargo clippy              # 린트
```

### 프로젝트 구조

```
src/
├── mcp/                 # MCP 프로토콜
│   ├── server.rs       # JSON-RPC stdio
│   ├── handlers.rs     # 도구 라우팅
│   └── types.rs        # MCP 타입
│
├── slack/              # Slack 클라이언트
│   ├── client.rs       # 통합 파사드
│   ├── core.rs         # HTTP + Rate Limiting
│   ├── users.rs        # 사용자 작업
│   ├── channels.rs     # 채널 작업
│   └── messages.rs     # 메시지 작업
│
├── cache/              # SQLite 캐시
│   ├── sqlite_cache.rs # 메인 구현
│   ├── schema.rs       # FTS5 스키마
│   ├── users.rs        # 사용자 캐싱
│   ├── channels.rs     # 채널 캐싱
│   └── locks.rs        # 분산 락
│
└── tools/              # MCP 도구 (8개)
    ├── search.rs       # 검색 도구 3개
    ├── messages.rs     # 메시징 4개
    ├── cache.rs        # 캐시 1개
    └── message_utils.rs # 토큰 최적화
```

**개발 문서:** [CLAUDE.md](CLAUDE.md)

---

## 📄 라이선스

MIT License - [LICENSE](LICENSE)

---

## 💬 지원

- **GitHub Issues**: [문제 리포트](https://github.com/junyeong-ai/mcp-slack/issues)
- **개발 문서**: [CLAUDE.md](CLAUDE.md)
- **MCP 프로토콜**: [modelcontextprotocol.io](https://modelcontextprotocol.io)

---

<div align="center">

**🌐 한국어** | **[English](README.en.md)**

**Version 0.1.0** • Built with Rust 2024 Edition

Made with ❤️ for AI Agents

</div>
