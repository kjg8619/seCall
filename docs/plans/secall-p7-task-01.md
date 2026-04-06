---
type: task
plan: secall-p7
task_number: 1
status: draft
updated_at: 2026-04-07
depends_on: []
parallel_group: A
---

# Task 01: 빈 세션 자동 스킵

## Changed files

- `crates/secall/src/main.rs:32-44` — Ingest struct에 `--min-turns` CLI 옵션 추가
- `crates/secall/src/commands/ingest.rs:27-32` — `run()` 함수에 `min_turns` 파라미터 추가
- `crates/secall/src/commands/ingest.rs:66-181` — `ingest_sessions()` 에서 턴 수 체크 후 skip
- `crates/secall/src/commands/sync.rs:200-224` — `run_auto_ingest()` 호출 시 min_turns 전달

## Change description

### 1단계: CLI 옵션 추가

`main.rs` Ingest variant에 옵션 추가:

```rust
Ingest {
    path: Option<String>,
    #[arg(long)]
    auto: bool,
    #[arg(long)]
    cwd: Option<PathBuf>,
    /// Skip sessions with fewer turns than this (default: 0 = no filter)
    #[arg(long, default_value = "0")]
    min_turns: usize,
}
```

### 2단계: ingest 로직에 필터 적용

`ingest.rs`의 `ingest_sessions()` 함수에 `min_turns: usize` 파라미터 추가. 세션 파싱 직후 (`parser.parse()` 또는 `parser.parse_all()` 반환 시점) 턴 수 확인:

```rust
if min_turns > 0 && session.turns.len() < min_turns {
    stats.skipped += 1;
    continue;
}
```

### 3단계: sync에서도 적용

`sync.rs`의 `run_auto_ingest()`에서 `ingest_sessions()` 호출 시 기본값 전달. sync는 기본적으로 `min_turns = 0` (필터 없음). 추후 config 파일에서 읽을 수 있도록 확장 가능하지만 이 task에서는 하드코딩.

### 4단계: 출력 메시지

기존 `IngestStats`에 `skipped_min_turns: usize` 필드 추가하여 "N skipped (too few turns)" 별도 표시. 기존 `skipped` (duplicate)과 구분.

## Dependencies

- 없음 (독립 task)

## Verification

```bash
# 1. 빌드 확인
cargo build -p secall

# 2. 테스트 통과
cargo test --all

# 3. --min-turns 옵션 동작 확인 (help에 표시)
cargo run -p secall -- ingest --help | grep min-turns

# 4. 실제 동작 테스트 — min-turns=100 으로 설정하면 대부분 skip
cargo run -p secall -- ingest --auto --min-turns 100
# 예상 출력: "N skipped (too few turns)" 포함
```

## Risks

- `ingest_sessions()` 시그니처 변경 → sync.rs에서 호출하는 부분도 같이 수정 필요
- `IngestStats` 필드 추가 → `print_ingest_result()` (output.rs)도 수정 필요

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/ingest/types.rs` — Session 구조체 변경 불필요
- `crates/secall-core/src/ingest/*.rs` — 파서 로직 변경 불필요
- `crates/secall-core/src/mcp/` — MCP 서버는 Task 03 범위
