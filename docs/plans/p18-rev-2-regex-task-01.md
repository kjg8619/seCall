---
type: task
status: in_progress
updated_at: 2026-04-11
plan: p18-rev-2-regex
task_number: 01
depends_on: []
parallel_group: null
---

# Task 01 — Regex 사전 컴파일 및 에러 전파

## Changed files

- `crates/secall/src/commands/classify.rs`
  - 줄 27~33: `.ok()` regex 블록 → pre-compile + `?` 전파로 교체
- `crates/secall/src/commands/ingest.rs`
  - 줄 154 앞: session 루프 시작 직전에 pre-compile 블록 삽입
  - 줄 305: `ingest_single_session()` 함수 시그니처에 `compiled_rules` 파라미터 추가
  - 줄 328~348: 루프 내 분류 블록을 pre-compiled rules 사용으로 교체
  - 줄 182~198, 244~260: `ingest_single_session()` 두 호출 지점에 `&compiled_rules` 인수 추가

새로 생성되는 파일 없음.

## Change description

### 문제 상세

`classify.rs:27-33` (현재 코드):

```rust
let matched_type = classification.rules.iter().find_map(|rule| {
    regex::Regex::new(&rule.pattern)
        .ok()                                        // ← 컴파일 오류 무시
        .filter(|re| re.is_match(first_content))
        .map(|_| rule.session_type.clone())
});
```

`ingest.rs:339-344` (현재 코드):

```rust
let matched_type = classification.rules.iter().find_map(|rule| {
    regex::Regex::new(&rule.pattern)
        .ok()                                        // ← 컴파일 오류 무시
        .filter(|re| re.is_match(first_user_content))
        .map(|_| rule.session_type.clone())
});
```

두 코드 모두 세션마다 regex를 재컴파일하고, 실패하면 `.ok()` → `None`으로
변환하여 해당 규칙을 조용히 건너뛴다.

---

### 수정 단계

#### Step 1 — `classify.rs`: for 루프 앞에 pre-compile 삽입, 루프 내 분류 블록 교체

```rust
// ── for 루프 직전 ──────────────────────────────────────────────────────────
let compiled_rules: Vec<(regex::Regex, String)> = classification
    .rules
    .iter()
    .map(|rule| {
        regex::Regex::new(&rule.pattern)
            .map(|re| (re, rule.session_type.clone()))
            .map_err(|e| {
                anyhow::anyhow!(
                    "invalid regex pattern {:?}: {}",
                    rule.pattern,
                    e
                )
            })
    })
    .collect::<anyhow::Result<_>>()?;   // ← 하나라도 실패하면 즉시 Err 반환

// ── 기존 분류 블록 (줄 27~33) 교체 ──────────────────────────────────────────
let matched_type = compiled_rules.iter().find_map(|(re, session_type)| {
    if re.is_match(first_content) {
        Some(session_type.clone())
    } else {
        None
    }
});
```

`classify.rs`의 `run_backfill()` 함수는 `Result<()>`를 반환하므로 `?` 사용 가능.

---

#### Step 2 — `ingest.rs`: session 루프 앞 pre-compile (줄 154 앞)

`ingest_single_session()` 함수 반환 타입이 `()` 이므로 내부에서 `?` 사용 불가.
대신 루프 진입 전 호출부에서 미리 컴파일한다.

```rust
// for session_path in &paths { 앞에 삽입
let compiled_rules: Vec<(regex::Regex, String)> = {
    let classification = &config.ingest.classification;
    classification
        .rules
        .iter()
        .map(|rule| {
            regex::Regex::new(&rule.pattern)
                .map(|re| (re, rule.session_type.clone()))
                .map_err(|e| {
                    anyhow::anyhow!(
                        "invalid regex pattern {:?}: {}",
                        rule.pattern,
                        e
                    )
                })
        })
        .collect::<anyhow::Result<_>>()?
};
```

이 블록을 감싸는 `ingest_paths()` 함수는 `Result<IngestStats>`를 반환하므로
`?` 사용 가능.

---

#### Step 3 — `ingest.rs`: `ingest_single_session()` 시그니처 변경 (줄 305)

```rust
// 변경 전
fn ingest_single_session(
    config: &Config,
    db: &Database,
    engine: &SearchEngine,
    vault: &Vault,
    mut session: secall_core::ingest::Session,
    format: &OutputFormat,
    min_turns: usize,
    force: bool,
    ...
)

// 변경 후: config 뒤에 compiled_rules 추가
fn ingest_single_session(
    config: &Config,
    compiled_rules: &[(regex::Regex, String)],  // ← 추가
    db: &Database,
    engine: &SearchEngine,
    vault: &Vault,
    mut session: secall_core::ingest::Session,
    format: &OutputFormat,
    min_turns: usize,
    force: bool,
    ...
)
```

---

#### Step 4 — `ingest.rs`: 분류 블록 교체 (줄 328~348)

```rust
// 변경 후
{
    let classification = &config.ingest.classification;
    if !compiled_rules.is_empty() {
        let first_user_content = session
            .turns
            .iter()
            .find(|t| t.role == secall_core::ingest::Role::User)
            .map(|t| t.content.as_str())
            .unwrap_or("");

        let matched_type = compiled_rules.iter().find_map(|(re, session_type)| {
            if re.is_match(first_user_content) {
                Some(session_type.clone())
            } else {
                None
            }
        });

        session.session_type =
            matched_type.unwrap_or_else(|| classification.default.clone());
    }
}
```

---

#### Step 5 — `ingest.rs`: 두 호출 지점에 `&compiled_rules` 인수 추가

- 줄 182 (`ClaudeAi`/`ChatGpt` 경로 내 `ingest_single_session(config, ...)`)
- 줄 244 (1:1 파서 경로 내 `ingest_single_session(config, ...)`)

두 곳 모두 `config` 뒤에 `&compiled_rules,` 삽입.

## Dependencies

- `regex` crate — 이미 `crates/secall/Cargo.toml`에 포함됨
- 다른 subtask 없음

## Verification

```bash
# 1. 컴파일 확인
cargo build -p secall 2>&1 | tail -20

# 2. 전체 테스트 (secall-core + secall)
cargo test -p secall-core -p secall 2>&1 | tail -30

# 3. 유효하지 않은 regex 감지 수동 확인
# .secall.toml에 다음을 임시 추가:
#   [[ingest.classification.rules]]
#   pattern = "[invalid"
#   session_type = "test"
# 이후 실행:
# Manual: cargo run -p secall -- classify --backfill --dry-run
# 기대값: exit 1, stderr에 "invalid regex pattern" 문자열 포함
```

> ⚠️ Verification 결과를 반드시 제출하세요. 미제출 시 Reviewer가 conditional 판정합니다.

## Risks

- `ingest_single_session()` 함수 시그니처 변경이 포함됨. 단, 이 함수는 동일 파일(`ingest.rs`) 내에서만 2곳 호출되므로 영향 범위가 파일 내부로 제한됨.
- `classification.rules`가 비어 있는 경우: `compiled_rules`도 비어 있어 분류를 건너뛴다 — 기존 동작과 동일.
- `compiled_rules.is_empty()` 체크를 유지해야 `rules = []` 설정 시 불필요한 `session_type` 덮어쓰기가 발생하지 않음.

## Scope boundary (수정 금지 파일)

다음 파일은 이 Task에서 수정하지 않는다:

- `crates/secall-core/src/vault/config.rs` — `ClassificationConfig` 구조체 (P18 Task 02)
- `crates/secall-core/src/store/schema.rs` — DB 스키마 (P18 Task 01)
- `crates/secall-core/src/store/db.rs` — DB 쿼리 (P18 Task 01, 2nd Rework)
- `crates/secall-core/src/search/bm25.rs` — BM25 필터 (P18 Task 03)
- `crates/secall-core/src/search/vector.rs` — 벡터 필터 (P18 2nd Rework)
- `crates/secall-core/src/search/hybrid.rs` — 하이브리드 검색 (P18 Task 03)
- `crates/secall-core/src/hooks/mod.rs` — 훅 (P18 Task 04)
- `crates/secall/src/commands/classify.rs` 내 `run_backfill()` 외 다른 함수
