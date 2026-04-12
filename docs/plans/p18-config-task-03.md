---
type: task
status: pending
plan: p18-config
task: 03
updated_at: 2026-04-11
---

# Task 03 — Ingest: 세션 분류 적용 + 임베딩 skip

## Changed files

- `crates/secall-core/src/ingest/types.rs:28` — `Session` 구조체에 `session_type: String` 필드 추가
- `crates/secall/src/commands/ingest.rs:319-420` — `process_session()` 함수에 분류 로직 추가, `vector_tasks.push()` 전 skip 조건 추가
- `crates/secall-core/src/search/bm25.rs:213-237` — `insert_session()` SQL INSERT 구문에 `session_type` 컬럼 반영

## Change description

### 1. types.rs — `Session`에 `session_type` 필드 추가

`Session` 구조체(line 28)에 필드 추가:
```rust
pub struct Session {
    pub id: String,
    pub agent: AgentKind,
    // ... 기존 필드 ...
    pub session_type: String,  // 추가. 기본값 "interactive"
}
```

각 파서(`claude.rs`, `codex.rs`, `gemini.rs` 등)의 `Session { .. }` 생성 부분에 `session_type: "interactive".to_string()` 추가. 파서 파일은 이 Task의 **Scope boundary에 포함**되지만, 컴파일 오류 해소를 위한 기본값 초기화만 수행한다.

### 2. ingest.rs — `process_session()`에 분류 로직 삽입

`process_session()` 함수 진입 직후(턴 수 필터 이후)에 분류 로직 추가:

```rust
// 세션 분류: 첫 번째 user turn의 내용을 규칙과 매칭
let classification = &config.ingest.classification;
if !classification.rules.is_empty() {
    let first_user_content = session
        .turns
        .iter()
        .find(|t| t.role == secall_core::ingest::Role::User)
        .map(|t| t.content.as_str())
        .unwrap_or("");

    let matched_type = classification.rules.iter().find_map(|rule| {
        regex::Regex::new(&rule.pattern)
            .ok()
            .filter(|re| re.is_match(first_user_content))
            .map(|_| rule.session_type.clone())
    });

    session.session_type = matched_type.unwrap_or_else(|| classification.default.clone());
}
```

`vector_tasks.push(session)` (line 420) 직전에 skip 조건 추가:

```rust
// 임베딩 skip 여부 확인
let skip_embed = config.ingest.classification
    .skip_embed_types
    .contains(&session.session_type);

if !skip_embed {
    vector_tasks.push(session);
}
```

### 3. bm25.rs — `insert_session()` SQL 수정

`insert_session()` (line 213)의 SQL을 `session_type` 컬럼 포함하도록 수정:

```rust
self.conn().execute(
    "INSERT OR IGNORE INTO sessions(id, agent, model, project, cwd, git_branch, host,
      start_time, end_time, turn_count, tokens_in, tokens_out, tools_used, tags,
      summary, ingested_at, status, session_type)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
    rusqlite::params![
        // ... 기존 params ...
        &session.session_type,  // ?18 추가
    ],
)?;
```

## Dependencies

- Task 01 완료 (DB에 `session_type` 컬럼 존재해야 INSERT 가능)
- Task 02 완료 (`ClassificationConfig` 타입 사용)

## Verification

```bash
cargo check -p secall-core
cargo check -p secall
cargo test -p secall-core -- test_index_and_search --nocapture
cargo test -p secall -- --nocapture 2>&1 | tail -20
```

수동 검증:
```bash
# Manual: secall ingest 실행 후 DB에서 session_type 확인
# sqlite3 ~/.secall/secall.db "SELECT id, session_type FROM sessions LIMIT 10;"
```

## Risks

- `Session` 구조체에 필드 추가 시 모든 파서(claude.rs, codex.rs, gemini.rs, chatgpt.rs 등)의 `Session { .. }` 리터럴에 컴파일 오류 발생 → 각 파서에 `session_type: "interactive".to_string()` 추가 필수
- `regex::Regex::new()` 호출이 ingest마다 일어남 → 규칙 수가 많으면 성능 이슈. 규모가 작으므로(보통 10개 미만) 허용. 추후 `OnceLock`으로 캐싱 가능
- `INSERT OR IGNORE` 패턴이므로 이미 존재하는 세션은 `session_type`이 갱신되지 않음 → backfill은 Task 05에서 별도 처리

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/store/schema.rs` (Task 01 영역)
- `crates/secall-core/src/store/db.rs` (Task 01 영역)
- `crates/secall-core/src/vault/config.rs` (Task 02 영역)
- `crates/secall-core/src/search/bm25.rs:21-31` — `SearchFilters` 구조체 (Task 04 영역)
- `crates/secall-core/src/mcp/server.rs` (Task 04 영역)
- `crates/secall/src/commands/recall.rs` (Task 04 영역)
