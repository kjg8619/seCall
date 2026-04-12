---
type: task
status: pending
plan: p18-config
task: 05
updated_at: 2026-04-11
---

# Task 05 — Backfill: `secall classify --backfill`

## Changed files

- `crates/secall/src/commands/classify.rs` (신규) — backfill 로직 구현
- `crates/secall/src/commands/mod.rs` — `pub mod classify;` 추가
- `crates/secall/src/main.rs` — `Classify` 서브커맨드 등록 및 핸들러 연결

## Change description

### 1. classify.rs (신규)

기존 세션의 첫 번째 user turn을 읽어 현재 설정된 규칙을 재적용하고 `sessions.session_type`을 업데이트한다.

```rust
// crates/secall/src/commands/classify.rs
use anyhow::Result;
use secall_core::{store::db::Database, vault::Config};

pub async fn run_backfill(dry_run: bool) -> Result<()> {
    let config = Config::load_or_default();
    let db = Database::open(&config)?;
    let classification = &config.ingest.classification;

    if classification.rules.is_empty() {
        eprintln!("No classification rules found in config. Add [ingest.classification.rules] to .secall.toml");
        return Ok(());
    }

    // 1. 전체 세션 목록 조회 (id + 첫 번째 user turn content)
    let sessions: Vec<(String, String)> = db.get_all_sessions_first_user_turn()?;

    let total = sessions.len();
    let mut updated = 0usize;
    let mut skipped = 0usize;

    for (session_id, first_content) in &sessions {
        // 규칙 매칭
        let matched_type = classification.rules.iter().find_map(|rule| {
            regex::Regex::new(&rule.pattern)
                .ok()
                .filter(|re| re.is_match(first_content))
                .map(|_| rule.session_type.clone())
        });
        let new_type = matched_type.unwrap_or_else(|| classification.default.clone());

        if dry_run {
            eprintln!("  [dry-run] {} → {}", &session_id[..8.min(session_id.len())], new_type);
        } else {
            db.update_session_type(session_id, &new_type)?;
        }
        updated += 1;
    }

    eprintln!(
        "Backfill {}complete: {}/{} sessions classified ({} skipped)",
        if dry_run { "(dry-run) " } else { "" },
        updated,
        total,
        skipped,
    );
    Ok(())
}
```

### 2. DB 메서드 추가 (`bm25.rs` 또는 `db.rs`)

`SessionRepo` 트레이트 또는 `Database`에 두 메서드 추가:

```rust
// 전체 세션의 (id, 첫 번째 user turn content) 반환
fn get_all_sessions_first_user_turn(&self) -> Result<Vec<(String, String)>> {
    // SELECT s.id, t.content FROM sessions s
    // JOIN turns t ON t.session_id = s.id
    // WHERE t.role = 'user'
    // GROUP BY s.id HAVING t.turn_index = MIN(t.turn_index)
}

// session_type 업데이트
fn update_session_type(&self, session_id: &str, session_type: &str) -> Result<()> {
    self.conn().execute(
        "UPDATE sessions SET session_type = ?1 WHERE id = ?2",
        rusqlite::params![session_type, session_id],
    )?;
    Ok(())
}
```

> 이 두 메서드 추가는 `bm25.rs`의 `Database impl` 블록 또는 별도 `classify_repo.rs`에 위치시킨다.

### 3. commands/mod.rs

```rust
pub mod classify;  // 추가
```

### 4. main.rs — 서브커맨드 등록

`Commands` 열거형에 추가:

```rust
/// Classify sessions by config rules
Classify {
    /// Preview changes without writing to DB
    #[arg(long)]
    dry_run: bool,
},
```

매칭 핸들러:
```rust
Commands::Classify { dry_run } => {
    commands::classify::run_backfill(dry_run).await?;
}
```

## Dependencies

- Task 01 완료 (`sessions.session_type` 컬럼 존재)
- Task 02 완료 (`ClassificationConfig` 타입 사용)
- Task 03, 04와 독립적으로 진행 가능

## Verification

```bash
cargo check -p secall
cargo build -p secall 2>&1 | tail -5
```

수동 검증:
```bash
# Manual: .secall.toml에 규칙 설정 후
# secall classify --dry-run
# → 각 세션의 예상 session_type 출력 확인
# secall classify --backfill  (실제 적용)
# sqlite3 ~/.secall/secall.db "SELECT session_type, COUNT(*) FROM sessions GROUP BY session_type;"
```

## Risks

- 세션 수가 많을 경우(1000개+) `get_all_sessions_first_user_turn()` 쿼리가 느릴 수 있음. 1차에서는 허용. 필요 시 청크 단위 처리 추가.
- `GROUP BY + MIN(turn_index)` 쿼리가 의도한 첫 user turn을 반환하는지 검증 필요. `turn_index`가 0부터 시작하는지 확인 (`ingest/types.rs`의 `Turn.index` 참조).
- `--backfill` 플래그를 명시적으로 요구하지 않으면 `secall classify` 단독 실행 시 동작 불명확 → 현재 설계에서는 `--dry-run` 없이 실행하면 바로 적용. 추후 `--backfill` 플래그로 명시화 고려.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/store/schema.rs` (Task 01 영역)
- `crates/secall-core/src/store/db.rs`의 `migrate()` (Task 01 영역, 단 신규 메서드 추가는 허용)
- `crates/secall-core/src/vault/config.rs` (Task 02 영역)
- `crates/secall-core/src/ingest/types.rs` (Task 03 영역)
- `crates/secall/src/commands/ingest.rs` (Task 03 영역)
- `crates/secall-core/src/search/bm25.rs:21-31` `SearchFilters` (Task 04 영역)
- `crates/secall-core/src/mcp/server.rs` (Task 04 영역)
