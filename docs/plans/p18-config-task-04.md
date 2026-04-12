---
type: task
status: pending
plan: p18-config
task: 04
updated_at: 2026-04-11
---

# Task 04 — Search: `SearchFilters`에 `session_type` 필터 추가

## Changed files

- `crates/secall-core/src/search/bm25.rs:21-27` — `SearchFilters`에 `exclude_session_types` 필드 추가
- `crates/secall-core/src/search/bm25.rs:350-385` — `search_fts()` SQL에 `session_type` 필터 조건 추가
- `crates/secall-core/src/mcp/server.rs:62-68` — `recall` 도구 기본값에 `exclude_session_types: vec!["automated"]` 설정
- `crates/secall/src/commands/recall.rs:50-56` — `--include-automated` 플래그 추가

## Change description

### 1. bm25.rs — `SearchFilters` 필드 추가

`SearchFilters` 구조체(line 21)에 필드 추가:

```rust
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub project: Option<String>,
    pub agent: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub max_per_session: Option<usize>,
    /// 제외할 session_type 목록. 기본값 [] (제외 없음)
    pub exclude_session_types: Vec<String>,
}
```

### 2. bm25.rs — `search_fts()` SQL 수정

`search_fts()` 함수(line 350)의 SQL 쿼리를 동적으로 구성:

```rust
fn search_fts(...) {
    let since_str = filters.since.map(|dt| dt.to_rfc3339());
    let until_str = filters.until.map(|dt| dt.to_rfc3339());

    // session_type 제외 조건 동적 생성
    let exclude_clause = if filters.exclude_session_types.is_empty() {
        String::new()
    } else {
        let placeholders: String = filters
            .exclude_session_types
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 5))  // ?5, ?6, ...
            .collect::<Vec<_>>()
            .join(", ");
        format!("AND (sessions.session_type IS NULL OR sessions.session_type NOT IN ({placeholders}))")
    };

    let sql = format!(
        "SELECT turns_fts.session_id, turns_fts.turn_id, turns_fts.content, bm25(turns_fts) as score
         FROM turns_fts
         JOIN sessions ON turns_fts.session_id = sessions.id
         WHERE turns_fts.content MATCH ?1
           AND (?2 IS NULL OR sessions.start_time >= ?2)
           AND (?3 IS NULL OR sessions.start_time < ?3)
           {exclude_clause}
         ORDER BY score
         LIMIT ?4"
    );

    // params: [query, since, until, limit, ...exclude_types]
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(tokenized_query.to_string()),
        Box::new(since_str),
        Box::new(until_str),
        Box::new(limit as i64),
    ];
    for t in &filters.exclude_session_types {
        params.push(Box::new(t.clone()));
    }

    // rusqlite::params_from_iter 사용
    let mut stmt = self.conn().prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
        Ok(FtsRow { ... })
    })?;
    ...
}
```

> **주의**: rusqlite의 동적 params는 `params_from_iter`를 사용하되, `dyn ToSql` 트레이트 객체 처리가 필요. 구현 시 `rusqlite::types::ToSql`의 `as_ref()` 패턴 또는 별도 enum wrapper 사용.

### 3. mcp/server.rs — recall 기본값 설정

`recall()` 함수(line 62)의 `SearchFilters` 초기화:

```rust
let mut base_filters = SearchFilters {
    project: params.project,
    agent: params.agent,
    since: None,
    until: None,
    exclude_session_types: vec!["automated".to_string()],  // 추가
    ..Default::default()
};
```

### 4. recall.rs — `--include-automated` 플래그

`run_recall()` 함수에 파라미터 추가 및 CLI 정의:

```rust
// recall.rs
pub async fn run_recall(
    query: &str,
    // ... 기존 파라미터 ...
    include_automated: bool,  // 추가
) -> Result<()> {
    let filters = SearchFilters {
        // ...
        exclude_session_types: if include_automated {
            vec![]
        } else {
            vec!["automated".to_string()]
        },
        ..Default::default()
    };
    ...
}
```

`main.rs`의 `Recall` 커맨드에 플래그 추가:
```rust
/// Include automated sessions in search results
#[arg(long)]
include_automated: bool,
```

## Dependencies

- Task 01 완료 (`sessions.session_type` 컬럼 존재)
- Task 03과 독립적으로 진행 가능 (SearchFilters 변경은 bm25.rs의 다른 영역)

## Verification

```bash
cargo check -p secall-core
cargo check -p secall
cargo test -p secall-core -- test_index_and_search --nocapture
```

수동 검증:
```bash
# Manual: automated 세션 ingest 후
# secall recall "테스트" → automated 세션 미포함 확인
# secall recall "테스트" --include-automated → automated 세션 포함 확인
```

## Risks

- `rusqlite`의 동적 파라미터 처리가 복잡함. `params_from_iter`가 `&dyn ToSql` 슬라이스를 요구하므로 타입 조율 필요. 구현이 복잡하면 exclude 조건을 최대 지원 개수(예: 5개)로 고정하고 `?5`, `?6`... 자리를 미리 생성하는 단순화 방식도 가능.
- `..Default::default()`를 사용하는 모든 `SearchFilters` 생성 부분은 `exclude_session_types`가 `Vec::new()`로 초기화되어 기존 동작 유지됨. MCP recall만 기본값 `["automated"]`로 설정.
- vector search(`search_repo.rs`)에도 동일한 필터 적용이 필요할 수 있음 → 1차 구현에서는 BM25에만 적용하고 RRF에서 BM25 결과를 기준으로 vector를 후처리하는 현재 구조상 자연스럽게 적용됨.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/store/schema.rs` (Task 01 영역)
- `crates/secall-core/src/store/db.rs` (Task 01 영역)
- `crates/secall-core/src/vault/config.rs` (Task 02 영역)
- `crates/secall-core/src/ingest/types.rs` (Task 03 영역)
- `crates/secall/src/commands/ingest.rs` (Task 03 영역)
- `crates/secall/src/commands/classify.rs` (Task 05 영역)
