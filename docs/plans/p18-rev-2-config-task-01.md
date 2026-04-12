---
type: task
status: in_progress
updated_at: 2026-04-11
plan: p18-rev-2-config
task_number: 01
depends_on: []
parallel_group: null
---

# Task 01 — Regex 사전 컴파일 및 에러 전파

## Changed files

- `crates/secall/src/commands/classify.rs` — 줄 27~33 (regex `.ok()` 블록 전체)
- `crates/secall/src/commands/ingest.rs` — 줄 154 (session 루프 시작 전), 줄 305 (함수 시그니처), 줄 328~348 (루프 내 분류 블록)

새로 생성되는 파일 없음.

## Change description

### 문제

`classify.rs:27-33`과 `ingest.rs:339-344`에서 regex를 매 세션마다 재컴파일한다. `Regex::new(&rule.pattern).ok()`는 컴파일 실패 시 `None`을 반환하므로, 잘못된 패턴은 조용히 건너뛰어 항상 `classification.default`로 분류된다. 사용자에게 오류 신호가 전달되지 않는다.

### 수정 접근법

**A. `classify.rs` — 루프 진입 전 사전 컴파일**

```rust
// 기존 (lines 27-33)
let matched_type = classification.rules.iter().find_map(|rule| {
    regex::Regex::new(&rule.pattern)
        .ok()
        .filter(|re| re.is_match(first_content))
        .map(|_| rule.session_type.clone())
});
```

다음으로 변경:

```rust
// 세션 루프 진입 전 (for 루프 앞)
let compiled_rules: Vec<(regex::Regex, String)> = classification
    .rules
    .iter()
    .map(|rule| {
        regex::Regex::new(&rule.pattern)
            .map(|re| (re, rule.session_type.clone()))
            .map_err(|e| anyhow::anyhow!("invalid regex pattern {:?}: {}", rule.pattern, e))
    })
    .collect::<anyhow::Result<_>>()?;

// 루프 내 분류
let matched_type = compiled_rules.iter().find_map(|(re, session_type)| {
    if re.is_match(first_content) {
        Some(session_type.clone())
    } else {
        None
    }
});
```

**B. `ingest.rs` — 함수 시그니처 변경 + 호출부 사전 컴파일**

`ingest_single_session()`이 `()` 반환이므로 내부에서 `?` 전파가 불가. 대신:

1. **호출부 (루프 앞 ~줄 154)** 에서 규칙을 미리 컴파일:

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
                .map_err(|e| anyhow::anyhow!("invalid regex pattern {:?}: {}", rule.pattern, e))
        })
        .collect::<anyhow::Result<_>>()?
};
```

2. **`ingest_single_session()` 시그니처 (줄 305)** 에 `compiled_rules` 파라미터 추가:

```rust
fn ingest_single_session(
    config: &Config,
    compiled_rules: &[(regex::Regex, String)],  // 추가
    db: &Database,
    ...
```

3. **분류 블록 (줄 328~348)** 에서 사전 컴파일된 rules 사용:

```rust
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

        session.session_type = matched_type.unwrap_or_else(|| classification.default.clone());
    }
}
```

4. **`ingest_single_session()` 두 호출부** (줄 182~198, 줄 244~260) 에 `&compiled_rules` 인수 추가.

## Dependencies

없음. `regex` crate는 이미 `Cargo.toml`에 포함.

## Verification

```bash
# 1. 타입 체크 및 컴파일
cargo build -p secall 2>&1 | tail -20

# 2. 전체 테스트
cargo test -p secall-core -p secall 2>&1 | tail -30

# 3. 유효하지 않은 regex 감지 수동 확인
# .secall.toml 임시 수정: pattern = "[invalid" 추가 후
# Manual: secall classify --backfill --dry-run
# 기대값: 오류 메시지 출력 후 exit(1) — "invalid regex pattern" 포함
```

## Risks

- `ingest_single_session()` 시그니처 변경이 있음. 이 함수는 `ingest.rs` 내부에서만 2곳 호출되므로 영향 범위가 파일 내로 제한된다.
- `classification.rules.is_empty()` 체크 로직: 기존에는 `!classification.rules.is_empty()`일 때만 분류 실행. `compiled_rules`가 비어있으면 분류를 건너뛰므로 동일한 동작을 유지한다.

## Scope boundary (수정 금지 파일)

다음 파일은 이 Task에서 수정하지 않는다:

- `crates/secall-core/src/vault/config.rs` — `ClassificationConfig` 구조체 (P18 Task 02)
- `crates/secall-core/src/store/schema.rs` — DB 스키마 (P18 Task 01)
- `crates/secall-core/src/store/db.rs` — DB 쿼리 (P18 Task 01, 2nd Rework)
- `crates/secall-core/src/search/bm25.rs` — BM25 필터 (P18 Task 03)
- `crates/secall-core/src/search/vector.rs` — 벡터 필터 (P18 2nd Rework)
- `crates/secall-core/src/search/hybrid.rs` — 하이브리드 검색 (P18 Task 03)
- `crates/secall-core/src/hooks/mod.rs` — 훅 (P18 Task 04)
