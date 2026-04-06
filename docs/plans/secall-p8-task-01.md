---
type: task
plan: secall-p8
task_number: 1
status: draft
updated_at: 2026-04-07
depends_on: []
parallel_group: A
---

# Task 01: 에러 리포팅 개선

## Changed files

- `crates/secall/src/commands/ingest.rs:21-25` — `IngestStats`에 에러 상세 목록 추가
- `crates/secall/src/commands/ingest.rs:55-60` — Summary 출력에 에러 상세 포함
- `crates/secall/src/commands/ingest.rs:82-163` — 에러 수집 로직 변경 (카운트 → 구조체 수집)
- `crates/secall/src/output.rs:45-100` — `print_ingest_summary()` 함수 추가, JSON 에러 리포트 지원

## Change description

### 1단계: IngestError 구조체 정의

`ingest.rs`에 에러 상세를 담는 구조체 추가:

```rust
#[derive(Debug, serde::Serialize)]
pub struct IngestError {
    pub path: String,
    pub session_id: Option<String>,
    pub phase: IngestPhase,
    pub message: String,
}

#[derive(Debug, serde::Serialize)]
pub enum IngestPhase {
    Detection,    // detect_parser 실패
    Parsing,      // parse/parse_all 실패
    DuplicateCheck, // DB 체크 실패
    VaultWrite,   // vault 쓰기 실패
    Indexing,     // BM25/벡터 인덱싱 실패
}
```

### 2단계: IngestStats 확장

```rust
pub struct IngestStats {
    pub ingested: usize,
    pub skipped: usize,
    pub errors: usize,
    pub error_details: Vec<IngestError>,
}
```

기존 `errors += 1` 위치를 `error_details.push(IngestError { ... })` + `errors += 1`로 변경. 총 5곳:
- `ingest.rs:85-90` — Detection 실패 (path + error)
- `ingest.rs:117-121` — parse_all 실패 (path + error)
- `ingest.rs:137-141` — DB check 실패 (path + error)
- `ingest.rs:159-162` — parse 실패 (path + error)
- `ingest_single_session()` 내부 — vault/index 실패 (session_id + error)

### 3단계: Text 출력 개선

`run()` 함수의 Summary 출력 후 에러가 있으면 상세 목록 추가:

```
Summary: 5 ingested, 2 skipped (duplicate), 3 errors

Errors:
  [Detection] /path/to/file.jsonl — unsupported format: unknown
  [Parsing]   /path/to/other.json — invalid JSON at line 42
  [Indexing]  abc12345 — BM25 tokenizer error: ...
```

### 4단계: JSON 출력 개선

`output.rs`에 `print_ingest_summary()` 추가. `--format json`일 때 구조화된 JSON 출력:

```json
{
  "summary": {
    "ingested": 5,
    "skipped": 2,
    "errors": 3
  },
  "errors": [
    {
      "path": "/path/to/file.jsonl",
      "session_id": null,
      "phase": "Detection",
      "message": "unsupported format: unknown"
    }
  ]
}
```

### 5단계: exit code

기존 동작 유지:
- 에러 0건 → exit 0
- 에러 있지만 일부 성공 → exit 0 (현재 동작 유지, 경고만)
- 전체 실패 (ingested == 0 && errors > 0) → `run()` 끝에서 `Err(anyhow!("all sessions failed"))` 반환

## Dependencies

- 없음 (독립 task)
- `serde::Serialize` — IngestError/IngestPhase JSON 직렬화용 (workspace에 이미 있음)

## Verification

```bash
# 1. 빌드 확인
cargo build -p secall

# 2. 테스트 통과
cargo test --all

# 3. 존재하지 않는 파일로 에러 리포트 확인
cargo run -p secall -- ingest /nonexistent/path.jsonl 2>&1
# 예상: [Detection] 에러 상세 출력

# 4. JSON 포맷 에러 리포트
cargo run -p secall -- --format json ingest /nonexistent/path.jsonl 2>&1
# 예상: JSON 구조체 출력

# 5. 정상 ingest 시 에러 없음 확인
cargo run -p secall -- ingest --auto 2>&1
# 예상: "Summary: N ingested, M skipped (duplicate), 0 errors" (에러 상세 없음)
```

## Risks

- `IngestStats` 구조체 변경 → P7 Task 01 (`skipped_min_turns`)과 충돌 가능. P7이 먼저 머지되면 필드 합산 필요
- `ingest_single_session()` 내부의 에러 수집 → 함수 시그니처에 `&mut Vec<IngestError>` 추가 또는 IngestStats 자체를 mut ref로 전달
- `serde::Serialize` derive → IngestPhase enum에 `#[serde(rename_all = "snake_case")]` 필요

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/error.rs` — SecallError enum 변경 불필요 (IngestError는 별도 CLI-level 구조체)
- `crates/secall-core/src/ingest/` — 파서 에러 메시지는 이미 충분
- `crates/secall-core/src/mcp/` — MCP 서버 에러 처리는 별도
- `crates/secall/src/commands/sync.rs` — sync 에러 리포팅은 이 task 범위 밖
- `.github/workflows/ci.yml` — CI 변경 금지 (Task 02 범위)
