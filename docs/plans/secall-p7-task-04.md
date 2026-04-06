---
type: task
plan: secall-p7
task_number: 4
status: draft
updated_at: 2026-04-07
depends_on: [1]
parallel_group: B
---

# Task 04: Incremental wiki in sync

## Changed files

- `crates/secall/src/commands/sync.rs:77-83` — Phase 3 이후에 Phase 3.5 (wiki update) 삽입
- `crates/secall/src/commands/sync.rs:13` — `run()` 시그니처에 `no_wiki: bool` 추가
- `crates/secall/src/commands/ingest.rs` — `ingest_sessions()` 반환값에 새로 ingest된 session_id 목록 추가
- `crates/secall/src/main.rs:124-133` — Sync subcommand에 `--no-wiki` 옵션 추가
- `crates/secall/src/commands/wiki.rs:6-70` — `run_update()` 재사용 (변경 없음, 호출만)

## Change description

### 1단계: ingest에서 새 session_id 수집

`IngestStats`에 `new_session_ids: Vec<String>` 필드 추가. `ingest_sessions()` 내에서 성공적으로 ingest된 세션의 ID를 수집.

```rust
pub struct IngestStats {
    pub ingested: usize,
    pub skipped: usize,
    pub errors: usize,
    pub skipped_min_turns: usize,  // Task 01에서 추가
    pub new_session_ids: Vec<String>,  // 이 task에서 추가
}
```

### 2단계: Sync에 Phase 3.5 추가

`sync.rs`의 Phase 3 (ingest) 이후, Phase 4 (push) 이전에 위키 생성 단계 삽입:

```rust
// === Phase 3.5: Incremental wiki (새 세션 → wiki 갱신) ===
if !no_wiki && !ingest_result.new_session_ids.is_empty() {
    if !command_exists("claude") {
        eprintln!("  ⚠ Claude CLI not found, skipping wiki update.");
    } else {
        eprintln!("Updating wiki for {} new sessions...", ingest_result.new_session_ids.len());
        for sid in &ingest_result.new_session_ids {
            match wiki::run_update("sonnet", None, Some(sid), false).await {
                Ok(()) => eprintln!("  ✓ wiki updated for {}", &sid[..8]),
                Err(e) => eprintln!("  ⚠ wiki failed for {}: {e}", &sid[..8]),
            }
        }
    }
}
```

### 3단계: CLI 옵션

`main.rs` Sync variant에 추가:

```rust
Sync {
    #[arg(long)]
    local_only: bool,
    #[arg(long)]
    dry_run: bool,
    /// Skip incremental wiki generation
    #[arg(long)]
    no_wiki: bool,
}
```

### 4단계: wiki 모델 설정

incremental wiki는 기본적으로 `sonnet` 모델 사용 (비용 절감). 사용자가 원하면 config에서 변경 가능하도록 하되, 이 task에서는 하드코딩.

### 5단계: dry-run 지원

`dry_run` 모드일 때 Phase 3.5도 "[DRY RUN] Would update wiki for N new sessions" 출력만.

## Dependencies

- **Task 01** — `IngestStats`에 `skipped_min_turns` 추가가 선행되어야 `new_session_ids` 추가 시 충돌 없음
- `wiki.rs:run_update()` — 이미 `--session` 모드 지원 (line 20-24)
- `wiki.rs:command_exists()` — 이미 구현됨 (line 147-153)

## Verification

```bash
# 1. 빌드 확인
cargo build -p secall

# 2. 테스트 통과
cargo test --all

# 3. --no-wiki 옵션 help에 표시
cargo run -p secall -- sync --help | grep no-wiki

# 4. dry-run으로 Phase 3.5 확인
cargo run -p secall -- sync --local-only --dry-run
# 예상: "[DRY RUN] Phase 3.5: Would update wiki..." 또는 새 세션 없으면 skip

# 5. 실제 sync (--no-wiki로 wiki 건너뛰기)
cargo run -p secall -- sync --local-only --no-wiki
# 예상: Phase 3.5 없이 완료

# 6. 실제 sync (wiki 포함, claude CLI 있는 경우)
# Manual: secall sync --local-only 실행 후 wiki/ 디렉토리에 새 파일 생성 확인
```

## Risks

- claude CLI 없는 환경에서 → `command_exists("claude")` 체크 + 경고 메시지로 graceful skip
- incremental wiki가 세션당 1회 claude 호출 → 많은 세션 ingest 시 비용 증가. 10개 이상이면 배치 모드 권장 메시지 출력 고려
- `run_update()`가 sync 함수에서 async로 호출됨 — 이미 async 컨텍스트이므로 문제 없음
- wiki 생성 중 에러가 push를 막으면 안 됨 → wiki 에러는 경고만, push는 계속 진행

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/mcp/` — Task 03 범위
- `crates/secall/src/commands/embed.rs` — Task 02 범위
- `docs/prompts/` — Task 05 범위
- `crates/secall-core/src/vault/git.rs` — git 로직 변경 불필요
