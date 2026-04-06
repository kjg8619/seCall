---
type: task
plan: secall-p9-chatgpt
task_number: 2
status: draft
updated_at: 2026-04-07
depends_on: [1]
parallel_group: B
---

# Task 02: ChatGPT 파서 구현

## Changed files

- `crates/secall-core/src/ingest/chatgpt.rs` — 신규 파일, ChatGptParser 구현
- `crates/secall-core/src/ingest/types.rs:7-11` — `AgentKind::ChatGpt` variant 추가
- `crates/secall-core/src/ingest/types.rs:15-22` — `as_str()` 매칭 추가 (`"chatgpt"`)
- `crates/secall-core/src/ingest/mod.rs:3` — `pub mod chatgpt;` 추가
- `crates/secall-core/src/ingest/detect.rs:6-12` — `use super::chatgpt::ChatGptParser;` import 추가
- `crates/secall-core/src/ingest/detect.rs` — ChatGPT JSON/ZIP 감지 로직 추가
- `crates/secall/src/commands/ingest.rs:94` — ClaudeAi와 동일하게 ChatGpt도 `parse_all()` 경로 라우팅

## Change description

### 1단계: AgentKind 확장

`types.rs`에 variant 추가:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentKind {
    ClaudeCode,
    ClaudeAi,
    ChatGpt,    // 추가
    Codex,
    GeminiCli,
}

impl AgentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ...
            AgentKind::ChatGpt => "chatgpt",
        }
    }
}
```

### 2단계: serde 구조체 (chatgpt.rs)

Task 01의 분석 결과를 기반으로 구조체 정의. 예상 구조:

```rust
#[derive(Debug, Deserialize)]
struct GptConversation {
    conversation_id: String,
    title: Option<String>,
    create_time: Option<f64>,       // Unix epoch float
    update_time: Option<f64>,
    default_model_slug: Option<String>,
    mapping: HashMap<String, MappingNode>,
    current_node: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MappingNode {
    id: String,
    message: Option<GptMessage>,
    parent: Option<String>,
    children: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GptMessage {
    id: String,
    author: GptAuthor,
    content: GptContent,
    create_time: Option<f64>,
    metadata: Option<GptMetadata>,
}

#[derive(Debug, Deserialize)]
struct GptAuthor {
    role: String,  // "system", "user", "assistant", "tool"
}

#[derive(Debug, Deserialize)]
struct GptContent {
    content_type: String,  // "text", "code", "multimodal_text", etc.
    parts: Option<Vec<serde_json::Value>>,  // [string | object]
}

#[derive(Debug, Deserialize)]
struct GptMetadata {
    model_slug: Option<String>,
    finish_details: Option<serde_json::Value>,
}
```

### 3단계: 트리 선형화

`current_node`에서 root까지 parent 체인을 역추적하여 선형 배열 생성:

```rust
fn linearize_mapping(conv: &GptConversation) -> Vec<&GptMessage> {
    let current = conv.current_node.as_deref().unwrap_or("");
    let mut chain = Vec::new();
    let mut node_id = current.to_string();
    
    loop {
        let Some(node) = conv.mapping.get(&node_id) else { break };
        if let Some(ref msg) = node.message {
            chain.push(msg);
        }
        match &node.parent {
            Some(pid) => node_id = pid.clone(),
            None => break,
        }
    }
    
    chain.reverse();
    chain
}
```

### 4단계: Session 변환

`conversation_to_session()` 함수:

- `conversation_id` → `Session.id`
- `title` → `Session.project`
- `create_time` (epoch float → DateTime<Utc>) → `Session.start_time`
- `default_model_slug` → `Session.model`
- 선형화된 메시지 → `Session.turns[]`
  - `author.role == "system"` → skip
  - `author.role == "user"` → `Role::User`
  - `author.role == "assistant"` → `Role::Assistant`
  - `author.role == "tool"` → `Role::System` (또는 Action으로 변환)
- `content.parts[]` → 텍스트만 추출 (`string` 타입만, object는 `[첨부파일]` 표시)
- `host` → `gethostname()` 설정

### 5단계: ZIP 처리

claude_ai.rs의 ZIP 해제 로직 재사용 패턴:
1. ZIP magic bytes (`PK\x03\x04`) 확인
2. `zip::ZipArchive`로 열기
3. `conversations.json` 엔트리 찾아 읽기
4. JSON 파싱 → `Vec<GptConversation>`
5. 각 conversation에 `conversation_to_session()` 적용

### 6단계: SessionParser trait 구현

```rust
pub struct ChatGptParser;

impl SessionParser for ChatGptParser {
    fn can_parse(&self, path: &Path) -> bool { ... }
    fn parse(&self, _path: &Path) -> Result<Session> {
        Err(SecallError::UnsupportedFormat("use parse_all for ChatGPT".into()))
    }
    fn agent_kind(&self) -> AgentKind { AgentKind::ChatGpt }
    fn parse_all(&self, path: &Path) -> Result<Vec<Session>> { ... }
}
```

`parse()`는 1:N이므로 사용 불가 → `parse_all()`만 구현. claude_ai.rs와 동일 패턴.

### 7단계: detect.rs에 감지 로직 추가

```rust
// ChatGPT export: ZIP (conversations.json 내 conversation_id 키)
if ext == "zip" {
    // 이미 ClaudeAi ZIP 감지 후 도달
    // ZIP 내부에 conversations.json이 있고 conversation_id 키가 있으면 ChatGPT
}

// ChatGPT export: conversations.json (JSON array with conversation_id + mapping)
if ext == "json" {
    // JSON array의 첫 요소에 mapping + conversation_id 키가 있으면 ChatGPT
}
```

**감지 우선순위**: claude.ai 감지가 먼저 (기존 코드 유지) → ChatGPT 감지는 그 후.
구분 기준:
- claude.ai: `chat_messages` + `uuid` 키
- ChatGPT: `mapping` + `conversation_id` 키

### 8단계: ingest.rs 라우팅

`ingest.rs:94`의 `AgentKind::ClaudeAi` 분기와 동일하게 ChatGpt도 `parse_all()` 경로 추가:

```rust
if parser.agent_kind() == AgentKind::ClaudeAi 
    || parser.agent_kind() == AgentKind::ChatGpt {
    match parser.parse_all(session_path) { ... }
}
```

## Dependencies

- **Task 01** — 실제 데이터 분석으로 serde 구조체 확정
- `zip` crate — workspace에 이미 있음 (`zip = "2"`)
- `HashMap` — `std::collections::HashMap` (표준 라이브러리)
- `gethostname` crate — workspace에 이미 있음

## Verification

```bash
# 1. 빌드 확인
cargo build -p secall

# 2. 테스트 통과
cargo test --all

# 3. AgentKind::ChatGpt가 as_str()에서 "chatgpt" 반환하는지 확인
cargo test -p secall-core -- chatgpt --nocapture

# 4. detect_parser가 ChatGPT conversations.json을 감지하는지 확인
cargo test -p secall-core -- detect_chatgpt --nocapture

# 5. clippy 클린
cargo clippy --all-targets -- -D warnings
```

## Risks

- **트리 선형화 실패**: current_node가 없거나 parent 체인이 끊어진 경우 → orphan 노드 무시, 가용한 체인만 추출
- **content.parts 다양성**: multimodal_text, execution_output 등 예상 못한 타입 → `serde_json::Value`로 받아 텍스트만 추출, 나머지는 `[unsupported content]` 표시
- **대용량 export**: 수천 대화 → 메모리 사용량 증가. claude_ai.rs와 동일하게 전체 JSON 로드 방식이므로 수 GB export는 문제될 수 있음 (현실적으로 수백 MB 이하)
- **claude.ai vs ChatGPT ZIP 구분**: 둘 다 ZIP인 경우 → ZIP 내부 파일 목록으로 구분 (claude.ai: `conversations.json` + `chat_messages`, ChatGPT: `conversations.json` + `mapping`)
- **epoch float 정밀도**: `f64` → `DateTime<Utc>` 변환 시 밀리초 이하 손실 (무시 가능)
- **빈 대화**: mapping에 system 메시지만 있는 경우 → turns 0개 세션 생성 (P7 Task 01의 min-turns로 필터링 가능)

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/ingest/claude_ai.rs` — claude.ai 파서 변경 불필요
- `crates/secall-core/src/ingest/claude.rs` — Claude Code 파서 변경 불필요
- `crates/secall-core/src/ingest/codex.rs` — Codex 파서 변경 불필요
- `crates/secall-core/src/ingest/gemini.rs` — Gemini 파서 변경 불필요
- `crates/secall-core/src/mcp/` — MCP 서버 변경 불필요
- `crates/secall-core/src/search/` — 검색 로직 변경 불필요
- `crates/secall/src/commands/sync.rs` — sync 변경 불필요
