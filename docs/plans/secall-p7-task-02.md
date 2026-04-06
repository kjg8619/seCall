---
type: task
plan: secall-p7
task_number: 2
status: draft
updated_at: 2026-04-07
depends_on: []
parallel_group: A
---

# Task 02: 임베딩 갭 메우기

## Changed files

- `crates/secall/src/commands/embed.rs` — 스텁을 실제 구현으로 교체 (전체 재작성)
- `crates/secall/src/main.rs:94-98` — Embed subcommand에 `--batch-size` 옵션 추가
- `crates/secall-core/src/store/db.rs` — `find_sessions_without_vectors()` 메서드 확인 (이미 존재, lint.rs에서 사용 중)

## Change description

### 1단계: DB에서 미임베딩 세션 조회

`db.find_sessions_without_vectors()`는 이미 `lint.rs:213-224`에서 사용 중. 이 메서드를 `embed.rs`에서도 호출.

### 2단계: embed.rs 구현

```rust
pub async fn run(all: bool, batch_size: Option<usize>) -> Result<()> {
    let config = Config::load_or_default();
    let db = Database::open(&get_default_db_path())?;
    
    let vector_indexer = create_vector_indexer(&config).await;
    let Some(indexer) = vector_indexer else {
        eprintln!("No embedding backend available.");
        eprintln!("  1. Download model: secall model download");
        eprintln!("  2. Check config: [embedding] section in config.toml");
        return Ok(());
    };
    
    // 미임베딩 세션 조회
    let session_ids = if all {
        db.all_session_ids()?
    } else {
        db.find_sessions_without_vectors()?
    };
    
    if session_ids.is_empty() {
        println!("All sessions already embedded.");
        return Ok(());
    }
    
    println!("Embedding {} sessions...", session_ids.len());
    
    for (i, sid) in session_ids.iter().enumerate() {
        // DB에서 세션 로드 → index_session() 호출
        let session = db.get_session_for_embedding(sid)?;
        let stats = indexer.index_session(&db, &session).await?;
        eprintln!("  [{}/{}] {} — {} chunks", i+1, session_ids.len(), &sid[..8], stats.chunks_embedded);
    }
    
    println!("Done.");
    Ok(())
}
```

### 3단계: 세션 데이터 로드

`db.get_session_for_embedding(session_id)` 메서드 필요 — DB의 turns 테이블에서 session_id로 조회하여 Session 구조체 재구성. 이미 `get_session()`이 있으면 재사용, 없으면 추가.

확인 필요: `crates/secall-core/src/store/db.rs`에서 Session 재구성 가능한 메서드 존재 여부.

### 4단계: CLI 옵션

`main.rs` Embed variant에 `--batch-size` 추가 (기본 32). `--all` 옵션은 이미 존재 (line 96).

### 5단계: 진행률 표시

세션 수가 수백 개이므로 `[N/total]` 형태로 진행률 출력.

## Dependencies

- ONNX 모델 다운로드 필요 (`secall model download`)
- `find_sessions_without_vectors()` — 이미 `crates/secall-core/src/store/db.rs`에 존재
- `index_session()` — `crates/secall-core/src/search/vector.rs:47-104`에 존재

## Verification

```bash
# 1. 빌드 확인
cargo build -p secall

# 2. 테스트 통과
cargo test --all

# 3. help에 embed 옵션 표시
cargo run -p secall -- embed --help

# 4. 임베딩 갭 확인 (lint L004)
cargo run -p secall -- lint 2>&1 | grep L004

# 5. embed 실행 (모델 있는 경우)
cargo run -p secall -- embed
# 예상: "Embedding N sessions..." → 진행률 → "Done."

# 6. 재실행 시 "All sessions already embedded." 출력
cargo run -p secall -- embed
```

## Risks

- ONNX 모델 미다운로드 시 → graceful error 필수 (panic 금지)
- 대량 세션 (600+) 임베딩 시 시간 소요 → 진행률 표시로 대응
- `get_session_for_embedding()` 메서드가 없을 수 있음 → DB 쿼리 추가 필요
- ANN 인덱스 reserve 부족 시 경고 → P6에서 이미 auto-reserve 구현됨

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/search/vector.rs` — `index_session()` 기존 로직 변경 불필요
- `crates/secall-core/src/search/ann.rs` — ANN 인덱스 로직 변경 불필요
- `crates/secall-core/src/ingest/` — 파서/ingest 로직 변경 불필요
- `crates/secall-core/src/mcp/` — MCP 서버는 Task 03 범위
