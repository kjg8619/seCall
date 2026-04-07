---
type: task
plan: secall-p11
task_number: 3
title: 세션 병렬 처리 + ANN 저장 빈도 감소
status: draft
depends_on: [1, 2]
parallel_group: null
updated_at: 2026-04-07
---

# Task 03 — 세션 병렬 처리 + ANN 저장 빈도 감소

## Changed files

| 파일 | 변경 유형 |
|------|----------|
| `crates/secall/src/commands/embed.rs:7-67` | 수정 — 순차 for loop → `futures::stream::buffered` 병렬 처리 |
| `crates/secall/src/main.rs:98-106` | 수정 — `--concurrency N` CLI 옵션 추가 |
| `crates/secall-core/src/search/vector.rs:47-104` | 수정 — ANN 저장 로직 외부화, `index_session()`에서 ANN save 제거 |
| `crates/secall-core/src/search/vector.rs` | 수정 — `save_ann()` pub 메서드 추가 |
| `crates/secall-core/src/search/ann.rs` | 확인만 — `save()` 메서드 기존 사용 (수정 불필요) |
| `Cargo.toml` (secall crate) | 수정 — `futures` 의존성 추가 (이미 있을 수 있음) |

## Change description

### 1. embed.rs 병렬화

현재 (line 35-63):
```rust
for (i, sid) in session_ids.iter().enumerate() {
    // 1개씩 순차 처리
}
```

변경:
```rust
use futures::stream::{self, StreamExt};

let concurrency = concurrency.unwrap_or(4); // CLI에서 전달
let indexer = Arc::new(indexer);
let db = Arc::new(db);
let counter = Arc::new(AtomicUsize::new(0));

stream::iter(session_ids.iter().cloned())
    .map(|sid| {
        let indexer = Arc::clone(&indexer);
        let db = Arc::clone(&db);
        let counter = Arc::clone(&counter);
        let total = total;
        async move {
            let session = match db.get_session_for_embedding(&sid) {
                Ok(s) => s,
                Err(e) => {
                    let i = counter.fetch_add(1, Ordering::Relaxed) + 1;
                    eprintln!("  [{i}/{total}] {} — load failed: {e}", &sid[..sid.len().min(8)]);
                    return;
                }
            };
            let i = counter.fetch_add(1, Ordering::Relaxed) + 1;
            match indexer.index_session(&db, &session).await {
                Ok(stats) => eprintln!(
                    "  [{i}/{total}] {} — {} chunks",
                    &sid[..sid.len().min(8)], stats.chunks_embedded
                ),
                Err(e) => eprintln!(
                    "  [{i}/{total}] {} — failed: {e}",
                    &sid[..sid.len().min(8)]
                ),
            }
        }
    })
    .buffer_unordered(concurrency)
    .collect::<()>()
    .await;
```

**참고**: `Database`는 `rusqlite::Connection`이 `!Send` — `Arc<Database>`를 직접 공유할 수 없을 수 있음. 대안:
- a) `Database`를 세션마다 새로 open (SQLite WAL 모드에서 안전, 약간의 오버헤드)
- b) `r2d2` 등 connection pool 도입
- c) `db` 접근을 `spawn_blocking` 내에서만 수행

**권장**: (a) 가장 단순. `Database::open(&db_path)`가 가볍고 WAL 모드에서 다중 커넥션 안전. 각 future에서 독립 `Database` 인스턴스 사용.

```rust
async move {
    let db = Database::open(&db_path).expect("db open");
    // ... index_session(&db, &session) ...
}
```

### 2. VectorIndexer의 Send + Sync

`VectorIndexer`는 `embedder: Box<dyn Embedder>` — `Embedder: Send + Sync`이므로 `Arc<VectorIndexer>`로 공유 가능.

`AnnIndex`의 usearch `Index`가 `Send + Sync`인지 확인 필요. usearch-rs는 내부적으로 thread-safe — 문서에 명시됨.

### 3. ANN 저장 빈도 감소

현재 (vector.rs:94-101): 매 세션마다 `ann.save()` 호출.

변경:
- `index_session()`에서 ANN save 로직 **제거**
- `VectorIndexer`에 pub 메서드 추가:

```rust
pub fn save_ann_if_present(&self) -> Result<()> {
    if let Some(ref ann) = self.ann_index {
        ann.save()?;
    }
    Ok(())
}
```

- `embed.rs`에서 호출 빈도 제어:

```rust
let save_interval = 50; // 50세션마다 저장
let mut sessions_since_save = 0;

// stream 처리 후 또는 callback으로:
sessions_since_save += 1;
if sessions_since_save >= save_interval {
    indexer.save_ann_if_present()?;
    sessions_since_save = 0;
}

// 최종 저장 (항상)
indexer.save_ann_if_present()?;
```

**병렬 처리와 ANN 저장 조합**: 병렬 세션들이 동시에 `ann.add()`를 호출하므로, ANN save는 병렬 스트림 외부에서 주기적으로 호출. 방법:
- `buffer_unordered` 결과를 `for_each`로 받으면서 카운터 업데이트
- 또는 완료 후 한 번만 save (가장 단순, 중단 시 ANN 유실 감수 — DB에서 재구축 가능)

**권장**: 완료 후 1회 save + SIGINT 핸들러에서 save. ANN은 DB에서 rebuild 가능하므로 중간 유실 허용.

### 4. CLI 옵션 추가 (main.rs)

```rust
Embed {
    #[arg(long)]
    all: bool,
    #[arg(long)]
    batch_size: Option<usize>,
    /// Number of sessions to embed concurrently (default: 4)
    #[arg(long, default_value = "4")]
    concurrency: usize,
},
```

### 5. futures 의존성

`Cargo.toml` (secall crate)에 `futures` 추가:
```toml
[dependencies]
futures = "0.3"
```

## Dependencies

- **Task 01 필수**: `index_session()`의 트랜잭션 래핑 + DELETE-first 전략이 병렬 안전의 전제조건. 각 세션이 독립 DB 커넥션에서 자체 트랜잭션으로 실행되므로 경합 없음.
- **Task 02 필수**: ORT batch inference가 되어야 병렬화의 실질적 효과 발생. 순차 inference에 병렬 세션을 올리면 Mutex 경합만 증가.

## Verification

```bash
# 타입 체크
cargo check --all

# 전체 테스트
cargo test --all

# clippy
cargo clippy --all-targets -- -D warnings

# CLI 도움말 확인
cargo run -- embed --help

# 실제 테스트 (소규모 세션으로)
cargo run -- embed --concurrency 2
```

## Risks

- **SQLite WAL 동시 쓰기**: WAL 모드에서 다중 커넥션의 동시 쓰기는 지원되지만, 높은 동시성에서 `SQLITE_BUSY` 에러 가능. 대응: `busy_timeout(5000)` 설정 (rusqlite `open_with_flags` 시).
- **메모리 사용량**: 동시 N개 세션의 chunks + embeddings가 메모리에 동시 존재. N=4, 평균 50 chunks × 1024 dim × 4 bytes = ~200KB/세션 — 총 ~800KB. 문제 없음.
- **ANN 동시 add()**: usearch의 `add()`가 thread-safe인지 확인 필수. usearch-rs 문서: "Index is thread-safe for insertions and searches" — OK.
- **진행률 순서 뒤섞임**: `buffer_unordered`이므로 출력 순서가 비순차적. UX에는 영향 없지만 로그 정렬이 안 됨 — 허용 가능.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/search/embedding.rs` — Task 02에서 완료됨
- `crates/secall-core/src/store/db.rs` (트랜잭션 로직) — Task 01에서 완료됨
- `crates/secall-core/src/store/schema.rs` — 변경 없음
- `crates/secall-core/src/search/chunker.rs` — 변경 없음
- 검색 경로 (`recall`, `mcp`) — 변경 없음
