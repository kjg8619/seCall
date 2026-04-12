---
type: task
status: pending
plan: p18-config
task: 01
updated_at: 2026-04-11
---

# Task 01 — DB 스키마 v4: `session_type` 컬럼 추가

## Changed files

- `crates/secall-core/src/store/schema.rs:1` — `CURRENT_SCHEMA_VERSION` 3 → 4, `CREATE_SESSIONS`에 `session_type` 컬럼 추가
- `crates/secall-core/src/store/db.rs:69-77` — `migrate()` 함수에 v3→v4 마이그레이션 블록 추가

## Change description

### 1. schema.rs

`CURRENT_SCHEMA_VERSION`을 4로 올리고, `CREATE_SESSIONS` 상수의 컬럼 목록에 `session_type TEXT DEFAULT 'interactive'`를 추가한다.

```rust
// schema.rs:1
pub const CURRENT_SCHEMA_VERSION: u32 = 4;
```

`CREATE_SESSIONS` 문자열 안 `status TEXT DEFAULT 'raw'` 뒤에:
```sql
session_type TEXT DEFAULT 'interactive'
```

### 2. db.rs — migrate() v3→v4 블록

기존 패턴(`if current < 2`, `if current < 3`)을 따라 v4 블록 추가:

```rust
if current < 4 {
    // session_type 컬럼이 없으면 추가
    let has_col: bool = self.conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'session_type'",
        [],
        |r| r.get::<_, i64>(0),
    ).unwrap_or(0) > 0;

    if !has_col {
        self.conn
            .execute("ALTER TABLE sessions ADD COLUMN session_type TEXT DEFAULT 'interactive'", [])?;
    }
}
```

`if current < CURRENT_SCHEMA_VERSION` 블록은 기존 위치(line 74) 그대로 유지한다.

## Dependencies

- 없음 (다른 Task와 독립)

## Verification

```bash
cargo test -p secall-core -- test_schema_version --nocapture
cargo test -p secall-core -- test_migrate --nocapture
cargo check -p secall-core
```

기대 결과:
- `test_schema_version` → `schema_version = 4`
- 기존 DB를 열었을 때 `sessions.session_type` 컬럼이 존재하고 기본값 `'interactive'`

## Risks

- 기존 `INSERT OR IGNORE INTO sessions(...)` 구문이 `session_type` 없이 동작 → Task 03에서 수정. 이 Task 완료 후 바로 `cargo test` 하면 기존 테스트는 통과해야 함 (컬럼 기본값 덕분).
- `pragma_table_info` 조회는 SQLite-only이지만 프로젝트 전체가 SQLite 기반이므로 문제없음.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/vault/config.rs` (Task 02 영역)
- `crates/secall-core/src/ingest/types.rs` (Task 03 영역)
- `crates/secall-core/src/search/bm25.rs` (Task 03/04 영역)
