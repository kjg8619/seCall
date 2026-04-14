---
type: task
status: draft
updated_at: 2026-04-14
plan: p23-refactor-boundaries
task_number: 03
depends_on: [1, 2]
parallel_group: null
---

# Task 03 — db.rs 메서드를 카테고리별 repo trait에 편입

## Changed files

- `crates/secall-core/src/store/db.rs`
  - 40+ pub 메서드를 카테고리별로 적절한 repo 파일로 이동
  - 잔류: `open`, `open_memory`, `migrate`, `conn`, `with_transaction`, `schema_version`, `table_exists` (~100줄 이하)
- `crates/secall-core/src/store/session_repo.rs`
  - trait에 세션 관련 메서드 추가 + impl 추가
- `crates/secall-core/src/store/search_repo.rs`
  - trait에 FTS 관련 메서드 추가 + impl 추가
- `crates/secall-core/src/store/vector_repo.rs`
  - trait에 벡터 관련 메서드 추가 + impl 추가
- `crates/secall-core/src/store/mod.rs`
  - 필요시 re-export 조정
- Task 01, 02에서 이동된 파일들의 trait 정의 확장

새로 생성 가능: `crates/secall-core/src/store/stats.rs` (통계/진단 메서드용, 선택)

## Change description

### db.rs 메서드 분류 및 행선지

| 카테고리 | 메서드 | 행선지 |
|----------|--------|--------|
| **세션 CRUD** | `insert_session_from_vault`, `get_session_for_embedding`, `get_session_with_turns`, `get_sessions_since`, `get_all_sessions_for_classify`, `get_sessions_for_date`, `update_session_summary`, `update_session_type`, `delete_session`, `list_all_session_ids`, `get_session_vault_path`, `list_session_vault_paths`, `migrate_vault_paths_to_relative` | `session_repo.rs` trait 확장 |
| **통계/진단** | `get_stats`, `count_sessions`, `count_fts_rows`, `count_turns`, `count_vectors`, `has_embeddings`, `list_projects`, `list_agents`, `agent_counts`, `find_sessions_without_vectors`, `find_orphan_vectors`, `find_duplicate_ingest_entries` | `session_repo.rs` trait 확장 또는 별도 `stats.rs` |
| **캐시** | `get_query_cache`, `set_query_cache` | `search_repo.rs` trait 확장 |
| **토픽** | `get_topics_for_sessions` | `session_repo.rs` trait 확장 |
| **턴 조회** | `get_turn` | `session_repo.rs` trait 확장 |
| **DB 인프라** | `open`, `open_memory`, `migrate`, `conn`, `with_transaction`, `schema_version`, `table_exists` | `db.rs`에 잔류 |
| **위키** | db.rs 799~: 두 번째 `impl Database` 블록의 `get_session_with_turns`, `get_sessions_since` | `session_repo.rs` trait 확장 |

### 작업 단계

1. 세션 CRUD 메서드를 `session_repo.rs` trait에 추가하고 impl 이동
2. 통계/진단 메서드를 trait에 추가하거나 `stats.rs`로 분리 (판단 기준: 10개 이상이면 분리)
3. 캐시 메서드를 `search_repo.rs` trait에 추가
4. db.rs에 인프라 메서드만 남기기
5. 매 단계 `cargo test` 실행

### 원칙

- 메서드 본문을 바꾸지 않는다
- 호출부의 `db.method()` 패턴이 trait import만 추가하면 그대로 동작하도록 유지
- trait 메서드로 전환 시 `&self` 시그니처 유지 (Database가 trait을 impl하므로 호출 코드 변경 최소화)

## Dependencies

- Task 01, 02 완료 필수 (repo 파일에 이미 impl이 들어간 상태에서 작업)

## Verification

```bash
# 1. 컴파일
cargo check -p secall-core -p secall 2>&1 | tail -10

# 2. 전체 테스트
cargo test -p secall-core -p secall 2>&1 | tail -30

# 3. db.rs 줄 수 확인 (목표: 200줄 이하)
wc -l crates/secall-core/src/store/db.rs

# 4. db.rs에 SELECT/INSERT 등 쿼리가 남아있지 않은지 확인 (migrate 제외)
grep -n "SELECT\|INSERT\|UPDATE.*SET\|DELETE FROM" crates/secall-core/src/store/db.rs | grep -v "migrate\|schema\|CREATE\|PRAGMA" || echo "OK: queries moved out"
```

## Risks

- **호출부 변경 범위가 넓음**: `db.method()` 호출이 commands/, mcp/, wiki/ 등 여러 파일에 분산.
  trait이 `impl for Database`이므로 `use crate::store::session_repo::SessionRepo;`만 추가하면
  기존 `db.method()` 호출이 그대로 동작. 하지만 누락 시 컴파일 에러 발생.
- **두 번째 `impl Database` 블록** (db.rs:799~): P22에서 추가된 위키용 메서드.
  이것도 `session_repo.rs`로 이동 대상.
- 통계 메서드 10개 이상이 한 trait에 몰리면 trait이 비대해질 수 있음.
  → 별도 `StatsRepo` trait + `stats.rs` 분리 고려.

## Scope boundary (수정 금지 파일)

- `crates/secall-core/src/search/bm25.rs` — Task 01에서 이미 정리됨
- `crates/secall-core/src/search/vector.rs` — Task 02에서 이미 정리됨
- 검색 알고리즘 로직 (hybrid.rs, ann.rs, query_expand.rs 등)
- CLI 명령 구조 (commands/*.rs의 구조 자체)
