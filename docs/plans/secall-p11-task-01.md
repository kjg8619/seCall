---
type: task
plan: secall-p11
task_number: 1
title: DB 트랜잭션 + 중단-재시작 안전성
status: draft
depends_on: []
parallel_group: A
updated_at: 2026-04-07
---

# Task 01 — DB 트랜잭션 + 중단-재시작 안전성

## Changed files

| 파일 | 변경 유형 |
|------|----------|
| `crates/secall-core/src/search/vector.rs:47-104` | 수정 — `index_session()`에 세션 단위 트랜잭션 래핑 |
| `crates/secall-core/src/store/db.rs:258-274` | 수정 — `find_sessions_without_vectors()` 부분 임베딩 감지 |
| `crates/secall-core/src/store/db.rs:344-382` | 수정 — `delete_session_vectors()` 메서드 추가 |
| `crates/secall-core/src/store/db.rs` (VectorRepo trait 근처) | 수정 — trait에 `delete_session_vectors`, `begin_transaction`, `commit_transaction` 추가 |
| `crates/secall-core/src/search/vector.rs:503+` | 수정 — 관련 테스트 추가 |

## Change description

### 1. 세션 벡터 삭제 메서드 추가 (db.rs)

`VectorRepo` impl에 추가:

```rust
pub fn delete_session_vectors(&self, session_id: &str) -> Result<usize> {
    let deleted = self.conn().execute(
        "DELETE FROM turn_vectors WHERE session_id = ?1",
        rusqlite::params![session_id],
    )?;
    Ok(deleted)
}
```

### 2. 세션 단위 트랜잭션 (vector.rs:index_session)

`index_session()` 전체를 트랜잭션으로 래핑:

```rust
pub async fn index_session(&self, db: &Database, session: &Session) -> Result<IndexStats> {
    let chunks = chunk_session(session);
    db.init_vector_table()?;

    // 1) 기존 벡터 삭제 (부분 임베딩 정리)
    let deleted = db.delete_session_vectors(&session.id)?;
    if deleted > 0 {
        tracing::info!(session_id = %session.id, deleted, "cleaned up partial vectors");
    }

    // 2) 임베딩 수행 (기존 batch 로직)
    let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
    // ... embed_batch 호출 ...

    // 3) 벡터 INSERT는 세션 단위 트랜잭션 내에서 실행
    db.execute_in_transaction(|conn| {
        for (embedding, chunk) in embeddings_and_chunks {
            // INSERT INTO turn_vectors ...
        }
        Ok(())
    })?;

    Ok(stats)
}
```

**핵심 설계**:
- 임베딩 계산은 트랜잭션 밖에서 수행 (CPU 시간 동안 DB lock 안 걸림)
- DELETE + INSERT만 트랜잭션으로 묶어서 원자성 보장
- 중단 시: 트랜잭션 미커밋 → 자동 롤백 → DELETE도 롤백 → 기존 상태 유지
- 재시작 시: `find_sessions_without_vectors()`가 해당 세션을 다시 선택

### 3. execute_in_transaction 헬퍼 (db.rs)

```rust
pub fn execute_in_transaction<F, T>(&self, f: F) -> Result<T>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T>,
{
    let conn = self.conn();
    conn.execute_batch("BEGIN")?;
    match f(conn) {
        Ok(val) => {
            conn.execute_batch("COMMIT")?;
            Ok(val)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}
```

### 4. find_sessions_without_vectors 보완 (db.rs:258-274)

현재 쿼리는 `NOT IN (SELECT DISTINCT session_id FROM turn_vectors)` — 1행이라도 있으면 완료 처리.

**개선**: 부분 임베딩 세션도 포함하도록 변경. DELETE-first 전략이므로 사실상 기존 쿼리 그대로 동작하지만, 안전장치로 보강:

```sql
SELECT id FROM sessions
WHERE id NOT IN (SELECT DISTINCT session_id FROM turn_vectors)
```

DELETE-first 전략 하에서는 이 쿼리가 올바르게 동작:
- 완전 임베딩된 세션 → turn_vectors에 있음 → 제외 ✅
- 부분 임베딩 후 중단 → DELETE가 롤백됨 → 부분 행 남아있음 → 재시작 시 `index_session()`에서 DELETE 후 재삽입 ✅
- 미임베딩 세션 → turn_vectors에 없음 → 포함 ✅

### 5. --all 모드 안전성

`secall embed --all` 실행 시 `list_all_session_ids()` 사용 → 모든 세션 순회 → `index_session()` 진입 시 DELETE 후 재삽입. 기존 벡터가 있어도 안전하게 덮어쓰기.

## Dependencies

- 없음 (첫 번째 태스크, Task 02와 parallel_group A로 병렬 가능)

## Verification

```bash
# 타입 체크
cargo check --all

# 전체 테스트 (기존 벡터 테스트 포함)
cargo test --all

# clippy
cargo clippy --all-targets -- -D warnings

# 벡터 관련 테스트만
cargo test -p secall-core vector
cargo test -p secall-core test_insert_and_search
```

## Risks

- **트랜잭션 중 DB lock**: SQLite는 WAL 모드에서도 쓰기 lock이 배타적. 대량 INSERT 시 다른 읽기가 지연될 수 있으나, embed 중 동시 검색은 일반적이지 않아 실질적 영향 낮음.
- **DELETE-first 비용**: 세션당 기존 벡터 DELETE가 추가되지만, `--all` 아닌 일반 모드에서는 대부분 0행 삭제 (영향 없음).
- **ANN 인덱스 정합성**: DELETE로 DB에서 벡터를 제거해도 ANN 인덱스(usearch)에는 여전히 존재. Task 03에서 ANN rebuild 전략과 함께 처리. 이 태스크에서는 ANN add 동작만 유지.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/search/embedding.rs` — Task 02 영역
- `crates/secall/src/commands/embed.rs` — Task 03, 04 영역
- `crates/secall/src/main.rs` — Task 03, 04 영역
- `crates/secall-core/src/search/ann.rs` — Task 03 영역
- `crates/secall-core/src/search/chunker.rs` — 변경 없음
