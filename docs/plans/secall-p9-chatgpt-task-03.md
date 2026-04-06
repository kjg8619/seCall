---
type: task
plan: secall-p9-chatgpt
task_number: 3
status: draft
updated_at: 2026-04-07
depends_on: [2]
parallel_group: C
---

# Task 03: 테스트 + E2E 검증

## Changed files

- `crates/secall-core/src/ingest/chatgpt.rs` — `#[cfg(test)] mod tests` 섹션 추가
- `crates/secall-core/src/ingest/detect.rs` — ChatGPT 감지 테스트 추가 (기존 `mod tests` 내)

## Change description

### 1단계: 단위 테스트 (chatgpt.rs)

#### 1-1. 트리 선형화 테스트

```rust
#[test]
fn test_linearize_simple_chain() {
    // system → user → assistant (선형)
    // current_node = assistant
    // 결과: [user, assistant] (system skip)
}

#[test]
fn test_linearize_with_regeneration() {
    // system → user → assistant_v1
    //                ↘ assistant_v2  ← current_node
    // 결과: [user, assistant_v2]
}

#[test]
fn test_linearize_missing_current_node() {
    // current_node가 None인 경우
    // 결과: 빈 배열 또는 fallback (마지막 leaf 노드)
}

#[test]
fn test_linearize_orphan_nodes() {
    // parent가 존재하지 않는 노드가 있는 경우
    // 결과: 끊어진 지점까지만 추출
}
```

#### 1-2. Session 변환 테스트

```rust
#[test]
fn test_conversation_to_session_basic() {
    // 기본 대화 (user → assistant, 2턴)
    // 확인: session.id, agent, project, start_time, turns.len(), roles
}

#[test]
fn test_epoch_to_datetime() {
    // 1711234567.123 → 2024-03-23T...
    // null → fallback (Utc::now() 또는 epoch 0)
}

#[test]
fn test_content_parts_text_extraction() {
    // parts: ["hello", {"type": "image", ...}, "world"]
    // 결과: "hello\n[첨부파일]\nworld"
}

#[test]
fn test_tool_role_handling() {
    // author.role == "tool" 인 메시지
    // 결과: Action::ToolUse 또는 content에 포함
}

#[test]
fn test_model_slug_extraction() {
    // default_model_slug: "gpt-4o"
    // 결과: session.model == Some("gpt-4o")
}
```

#### 1-3. ZIP 파싱 테스트

```rust
#[test]
fn test_parse_all_from_json() {
    // conversations.json 직접 파싱 (ZIP 없이)
    // 테스트 fixture: 최소 JSON (2 conversations, 각 2턴)
}

#[test]
fn test_parse_all_from_zip() {
    // 메모리에 ZIP 생성 → conversations.json 포함
    // parse_all() 호출 → sessions 반환 확인
}

#[test]
fn test_empty_conversations() {
    // conversations.json = [] (빈 배열)
    // 결과: Ok(vec![])
}
```

### 2단계: 감지 테스트 (detect.rs)

```rust
#[test]
fn test_detect_chatgpt_json() {
    // conversations.json with mapping + conversation_id
    // detect_parser() → ChatGptParser
}

#[test]
fn test_detect_chatgpt_vs_claude_ai() {
    // claude.ai conversations.json (chat_messages + uuid)
    // → ClaudeAiParser (기존 동작 유지)
    // ChatGPT conversations.json (mapping + conversation_id)
    // → ChatGptParser
}
```

### 3단계: E2E 검증 (실제 데이터)

사용자의 실제 ChatGPT export로 전체 파이프라인 테스트:

```bash
# 1. ingest
cargo run -p secall -- ingest desktop_conversation/chatgpt/*.zip

# 2. vault 파일 생성 확인
find ~/Documents/Obsidian\ Vault/seCall/raw/sessions/ -name "chatgpt_*" | wc -l

# 3. 검색 동작 확인
cargo run -p secall -- recall "검색어" --agent chatgpt --limit 3

# 4. vault MD 내용 확인 (frontmatter 정상)
head -20 ~/Documents/Obsidian\ Vault/seCall/raw/sessions/*/chatgpt_*.md
```

### 4단계: 기존 테스트 회귀 확인

```bash
# 전체 테스트 통과 확인
cargo test --all

# claude.ai, Claude Code, Codex, Gemini 감지 테스트가 깨지지 않았는지 확인
cargo test -p secall-core -- detect --nocapture
```

## Dependencies

- **Task 02** — 파서 구현 완료 후 테스트 작성
- ChatGPT export ZIP (E2E 테스트용, 없으면 단위 테스트만)
- `tempfile` crate — 테스트 임시 파일 (workspace에 이미 dev-dependency)
- `zip` crate — ZIP fixture 생성 (테스트에서 메모리 ZIP 생성)

## Verification

```bash
# 1. 단위 테스트 통과
cargo test -p secall-core -- chatgpt --nocapture

# 2. 감지 테스트 통과
cargo test -p secall-core -- detect_chatgpt --nocapture

# 3. 전체 테스트 회귀 없음
cargo test --all

# 4. clippy 클린
cargo clippy --all-targets -- -D warnings

# 5. E2E (실제 데이터 있는 경우)
cargo run -p secall -- ingest desktop_conversation/chatgpt/*.zip 2>&1
# 예상: "Parsed N conversations from ..." + "Summary: N ingested, ..."
```

## Risks

- **실제 데이터 없으면 E2E 불가** → 단위 테스트로 커버, E2E는 데이터 확보 후 별도 실행
- **테스트 fixture와 실제 데이터 불일치** → claude.ai 때와 동일한 실수 방지를 위해 Task 01 분석 결과 기반으로 fixture 작성
- **ZIP fixture 생성 복잡도** → `zip` crate의 `ZipWriter::new(Cursor::new(Vec::new()))` 사용

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/ingest/claude_ai.rs` — claude.ai 테스트 변경 불필요
- `crates/secall-core/src/ingest/claude.rs` — Claude Code 테스트 변경 불필요
- `crates/secall/src/commands/` — CLI 커맨드 테스트는 이 task 범위 밖
- `docs/plans/secall-p7*.md` — P7 범위
- `docs/plans/secall-p8*.md` — P8 범위
