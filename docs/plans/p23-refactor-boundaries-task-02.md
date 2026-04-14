---
type: task
status: draft
updated_at: 2026-04-14
plan: p23-refactor-boundaries
task_number: 02
depends_on: []
parallel_group: A
---

# Task 02 — vector.rs에서 repo impl을 store/로 이동

## Changed files

- `crates/secall-core/src/search/vector.rs`
  - 줄 458~600: `impl VectorRepo for Database` 블록 **삭제**
  - 줄 602~: `floats_to_bytes`, `bytes_to_floats` 헬퍼 — VectorRepo impl이 사용하므로 함께 이동하거나 공용 위치로 이동
  - 필요한 use문 정리
- `crates/secall-core/src/store/vector_repo.rs`
  - 기존 trait 정의 유지
  - `impl VectorRepo for Database` 추가 (vector.rs에서 이동한 코드)
  - `floats_to_bytes`, `bytes_to_floats` 헬퍼 함수 포함
  - 필요한 use문 추가
- `crates/secall-core/src/store/mod.rs`
  - 필요시 모듈 re-export 조정

새로 생성되는 파일 없음.

## Change description

### 이동 대상

**`impl VectorRepo for Database`** (vector.rs:459~600, ~142줄)
- `init_vector_table`, `insert_vector`, `search_vectors`, `get_vector_meta`
- → `store/vector_repo.rs`로 이동

**헬퍼 함수** (vector.rs:602~)
- `floats_to_bytes(floats: &[f32]) -> Vec<u8>`
- `bytes_to_floats(bytes: &[u8]) -> Vec<f32>`
- VectorRepo impl 내부에서만 사용되면 `store/vector_repo.rs`로 함께 이동
- vector.rs 본체(VectorIndexer)에서도 사용하면 `pub(crate)` 함수로 두거나 양쪽에서 접근 가능한 위치에 배치

### 원칙

- 함수 본문을 바꾸지 않는다 (순수 이동)
- trait 정의는 그대로 둔다
- vector.rs에는 `VectorIndexer`, embedding/ANN 로직, 테스트만 남긴다

## Dependencies

- 없음 (Task 01과 병렬 가능)

## Verification

```bash
# 1. 컴파일
cargo check -p secall-core -p secall 2>&1 | tail -10

# 2. 전체 테스트
cargo test -p secall-core -p secall 2>&1 | tail -30

# 3. vector.rs에 직접 SQL 문자열 없음 확인
grep -n "INSERT\|SELECT\|CREATE TABLE\|CREATE INDEX" crates/secall-core/src/search/vector.rs || echo "OK: no SQL in vector.rs"

# 4. vector.rs에 Database import 없음 확인 (VectorRepo trait 사용은 OK)
grep -n "use.*store.*Database\|use.*db.*Database" crates/secall-core/src/search/vector.rs || echo "OK: no Database import in vector.rs"
```

## Risks

- `floats_to_bytes`/`bytes_to_floats`가 vector.rs 본체(VectorIndexer)에서도 사용되는지 확인 필요.
  양쪽에서 사용하면 `crate::store::vector_repo`에 `pub(crate)`로 두고 vector.rs에서 import.
- `search_vectors` impl 내부에 cosine similarity 계산이 포함되어 있을 수 있음 (brute-force fallback).
  이 경우 알고리즘 자체는 건드리지 않고 그대로 이동.

## Scope boundary (수정 금지 파일)

- `crates/secall-core/src/search/bm25.rs` — Task 01 영역
- `crates/secall-core/src/store/db.rs` — Task 03 영역
- `crates/secall/src/commands/*` — 이번 라운드 범위 밖
