---
type: task
status: draft
updated_at: 2026-04-14
plan: p23-refactor-boundaries
task_number: 01
depends_on: []
parallel_group: A
---

# Task 01 — bm25.rs에서 repo impl을 store/로 이동

## Changed files

- `crates/secall-core/src/search/bm25.rs`
  - 줄 194~352: `impl SessionRepo for Database` 블록 **삭제**
  - 줄 354~끝(테스트 제외): `impl SearchRepo for Database` 블록 **삭제**
  - 필요한 use문 정리 (Database import 제거 등)
- `crates/secall-core/src/store/session_repo.rs`
  - 기존 trait 정의 유지
  - `impl SessionRepo for Database` 추가 (bm25.rs에서 이동한 코드)
  - 필요한 use문 추가
- `crates/secall-core/src/store/search_repo.rs`
  - 기존 trait 정의 유지
  - `impl SearchRepo for Database` 추가 (bm25.rs에서 이동한 코드)
  - 필요한 use문 추가
- `crates/secall-core/src/store/mod.rs`
  - 필요시 모듈 re-export 조정

새로 생성되는 파일 없음. 기존 파일에 코드 이동.

## Change description

### 이동 대상

1. **`impl SessionRepo for Database`** (bm25.rs:195~352, ~158줄)
   - `insert_session`, `update_session_vault_path`, `insert_turn`,
     `session_exists`, `session_exists_by_prefix`, `get_session_meta`,
     `is_session_open`, `delete_session`
   - → `store/session_repo.rs`로 이동

2. **`impl SearchRepo for Database`** (bm25.rs:355~끝의 impl 블록)
   - `insert_fts`, `search_fts`
   - → `store/search_repo.rs`로 이동

### 원칙

- 함수 본문을 한 글자도 바꾸지 않는다 (순수 이동)
- trait 정의는 그대로 둔다
- bm25.rs에는 `Bm25Indexer`, `SearchFilters`, `SearchResult`, snippet/normalize, 테스트만 남긴다
- `Database` import가 bm25.rs에서 더 이상 필요 없으면 제거

### 참고: graph_repo.rs 패턴

`store/graph_repo.rs`(342줄)가 이미 trait + impl을 한 파일에 두는 패턴을 사용 중.
동일 패턴을 따른다.

## Dependencies

- 없음 (Task 02와 병렬 가능)

## Verification

```bash
# 1. 컴파일
cargo check -p secall-core -p secall 2>&1 | tail -10

# 2. 전체 테스트
cargo test -p secall-core -p secall 2>&1 | tail -30

# 3. bm25.rs에 SQL 문자열 없음 확인
grep -n "INSERT\|SELECT\|CREATE\|UPDATE\|DELETE" crates/secall-core/src/search/bm25.rs || echo "OK: no SQL in bm25.rs"

# 4. bm25.rs에 Database import 없음 확인
grep -n "use.*store.*Database\|use.*db.*Database" crates/secall-core/src/search/bm25.rs || echo "OK: no Database import in bm25.rs"
```

## Risks

- `SessionRepo`와 `SearchRepo` trait은 다른 파일에서 `use crate::search::bm25::*` 경로로 import될 수 있음.
  trait 자체는 `store/`에 있으므로 `use crate::store::session_repo::SessionRepo` 경로가 정상.
  기존에 bm25 경로로 trait을 가져오는 코드가 있으면 경로 수정 필요.
- impl 블록 안에서 `crate::ingest::*`, `crate::error::*` 등 다른 모듈 타입을 사용하므로 use문 정리 필요.

## Scope boundary (수정 금지 파일)

- `crates/secall-core/src/search/vector.rs` — Task 02 영역
- `crates/secall-core/src/store/db.rs` — Task 03 영역
- `crates/secall/src/commands/*` — 이번 라운드 범위 밖
