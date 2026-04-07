---
type: task
plan: secall-p11
task_number: 4
title: batch_size CLI 연결 + 진행률 표시 개선
status: draft
depends_on: [3]
parallel_group: null
updated_at: 2026-04-07
---

# Task 04 — batch_size CLI 연결 + 진행률 표시 개선

## Changed files

| 파일 | 변경 유형 |
|------|----------|
| `crates/secall/src/commands/embed.rs` | 수정 — batch_size를 indexer에 전달, 진행률 표시 개선 |
| `crates/secall-core/src/search/vector.rs:47-56` | 수정 — `index_session()`이 batch_size 파라미터 수용 |
| `crates/secall-core/src/search/vector.rs:28-32` | 수정 — `VectorIndexer`에 설정 필드 추가 |

## Change description

### 1. batch_size 파라미터 연결

현재 `embed.rs:32`에서 `_batch_size`로 읽기만 하고 미사용.
`vector.rs:56`에서 `let batch_size = 32;`로 하드코딩.

**변경 1 — VectorIndexer 설정**:

```rust
pub struct VectorIndexer {
    embedder: Box<dyn Embedder>,
    ann_index: Option<AnnIndex>,
    batch_size: usize,  // 추가
}

impl VectorIndexer {
    pub fn new(embedder: Box<dyn Embedder>) -> Self {
        VectorIndexer {
            embedder,
            ann_index: None,
            batch_size: 32,  // 기본값
        }
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);  // 최소 1
        self
    }
}
```

**변경 2 — index_session()에서 self.batch_size 사용**:

```rust
// vector.rs:56 변경
let batch_size = self.batch_size;  // 기존: let batch_size = 32;
```

**변경 3 — embed.rs에서 전달**:

```rust
let batch_size = batch_size.unwrap_or(32);
// create_vector_indexer 후:
let indexer = indexer.with_batch_size(batch_size);
```

### 2. 진행률 표시 개선

현재: `[1/1242] a1b2c3d4 — 50 chunks` (세션 완료 시 1줄)

**개선**: 시작 시간 기록 → ETA 계산 + chunks/sec 표시:

```rust
use std::time::Instant;

let start = Instant::now();
let completed = AtomicUsize::new(0);
let total_chunks = AtomicUsize::new(0);

// 각 세션 완료 후:
let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
let chunks_done = total_chunks.fetch_add(stats.chunks_embedded, Ordering::Relaxed) + stats.chunks_embedded;
let elapsed = start.elapsed().as_secs_f64();
let rate = chunks_done as f64 / elapsed;
let remaining = total - done;
let eta_secs = if rate > 0.0 { remaining as f64 / (done as f64 / elapsed) } else { 0.0 };
let eta_min = (eta_secs / 60.0).ceil() as u64;

eprintln!(
    "  [{done}/{total}] {} — {} chunks ({:.1} chunks/s, ETA ~{eta_min}m)",
    &sid[..sid.len().min(8)],
    stats.chunks_embedded,
    rate,
);
```

**시작 시 요약**:

```rust
eprintln!("Embedding {} session(s) [batch_size={}, concurrency={}]...", total, batch_size, concurrency);
```

**완료 시 요약**:

```rust
let elapsed = start.elapsed();
let mins = elapsed.as_secs() / 60;
let secs = elapsed.as_secs() % 60;
eprintln!(
    "\nDone: {} sessions, {} chunks in {}m {}s ({:.1} chunks/s)",
    total,
    total_chunks.load(Ordering::Relaxed),
    mins, secs,
    total_chunks.load(Ordering::Relaxed) as f64 / elapsed.as_secs_f64(),
);
```

### 3. ingest 경로에도 batch_size 전달

`commands/ingest.rs`에서 `engine.index_session_vectors()`를 호출하는 경로도 동일하게 적용되도록 `SearchEngine`이 batch_size를 `VectorIndexer`에 전달하는지 확인. 현재 ingest는 `create_vector_indexer()`의 기본값(32)을 사용하므로 별도 변경 불필요 — 기본값이 CLI와 동일.

## Dependencies

- **Task 03 필수**: 병렬 처리 구조가 완성된 후 진행률 표시를 맞춤

## Verification

```bash
# 타입 체크
cargo check --all

# 전체 테스트
cargo test --all

# clippy
cargo clippy --all-targets -- -D warnings

# CLI 도움말 — batch_size, concurrency 표시 확인
cargo run -- embed --help

# 실제 실행 (소규모)
cargo run -- embed --batch-size 16 --concurrency 2
# 예상 출력:
# Embedding 5 session(s) [batch_size=16, concurrency=2]...
#   [1/5] a1b2c3d4 — 42 chunks (12.3 chunks/s, ETA ~2m)
#   ...
# Done: 5 sessions, 210 chunks in 0m 17s (12.4 chunks/s)
```

## Risks

- **ETA 부정확**: 초기 세션(chunk 수 편차가 클 때) ETA가 흔들릴 수 있음. 이동평균 등 고급 기법은 이 태스크 범위 밖 — 단순 전체 평균으로 충분.
- **Atomic 카운터 정밀도**: `buffer_unordered`에서 다수 세션이 거의 동시에 완료되면 진행률 번호가 약간 뒤섞일 수 있음 — 기능적 문제 없음.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/search/embedding.rs` — Task 02에서 완료됨
- `crates/secall-core/src/store/db.rs` — Task 01에서 완료됨
- `crates/secall-core/src/search/ann.rs` — Task 03에서 완료됨
- `crates/secall-core/src/search/chunker.rs` — 변경 없음
- 검색 경로 (`recall`, `mcp`, `get`) — 변경 없음
- `README.md` — 기능 완료 후 별도 업데이트
