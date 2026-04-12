---
type: task
status: pending
plan: p18-config
task: 02
updated_at: 2026-04-11
---

# Task 02 — Config: `ClassificationConfig` 추가

## Changed files

- `crates/secall-core/src/vault/config.rs:48-51` — `IngestConfig`에 `classification` 필드 추가
- `crates/secall-core/src/vault/config.rs` (신규 구조체) — `ClassificationRule`, `ClassificationConfig`
- `Cargo.toml` — `regex = "1"` workspace dep 추가
- `crates/secall-core/Cargo.toml` — `regex.workspace = true` 추가

## Change description

### 1. Cargo.toml — regex crate 추가

`Cargo.toml` workspace deps 섹션에:
```toml
regex = "1"
```

`crates/secall-core/Cargo.toml` dependencies 섹션에:
```toml
regex.workspace = true
```

### 2. config.rs — 새 구조체 정의

`IngestConfig` 구조체 위에 추가:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    /// 첫 번째 user turn 내용에 매칭할 regex 패턴
    pub pattern: String,
    /// 매칭 시 부여할 session_type (예: "automated", "health_check")
    pub session_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationConfig {
    /// 규칙에 매칭되지 않을 때 기본 session_type
    #[serde(default = "default_session_type")]
    pub default: String,
    /// 순서대로 매칭 시도, 첫 번째 매칭 규칙 적용
    #[serde(default)]
    pub rules: Vec<ClassificationRule>,
    /// 임베딩을 skip할 session_type 목록
    #[serde(default)]
    pub skip_embed_types: Vec<String>,
}

fn default_session_type() -> String {
    "interactive".to_string()
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        ClassificationConfig {
            default: default_session_type(),
            rules: Vec::new(),
            skip_embed_types: Vec::new(),
        }
    }
}
```

### 3. config.rs — `IngestConfig`에 필드 추가

`IngestConfig` 구조체(line 48)에:
```rust
pub struct IngestConfig {
    pub tool_output_max_chars: usize,
    pub thinking_included: bool,
    pub classification: ClassificationConfig,  // 추가
}
```

`impl Default for IngestConfig`(line 103)에:
```rust
IngestConfig {
    tool_output_max_chars: 500,
    thinking_included: true,
    classification: ClassificationConfig::default(),  // 추가
}
```

### 4. 사용자 config 예시 (문서화 목적, 코드 변경 없음)

```toml
[ingest.classification]
default = "interactive"
skip_embed_types = ["automated"]

[[ingest.classification.rules]]
pattern = "^\\[당월 rawdata\\]"
session_type = "automated"

[[ingest.classification.rules]]
pattern = "^# Wiki Incremental Update Prompt"
session_type = "automated"
```

## Dependencies

- 없음 (Task 01과 독립)

## Verification

```bash
cargo check -p secall-core
cargo test -p secall-core -- config --nocapture
```

기대 결과: `ClassificationConfig::default()`가 올바른 기본값을 반환하고, TOML deserialization이 정상 동작.

## Risks

- `regex` crate 컴파일 시간 증가 (약 2-5초)
- `serde(default)` 누락 시 기존 `.secall.toml`을 가진 사용자가 파싱 오류를 겪을 수 있음 → 모든 필드에 `#[serde(default)]` 적용 필수

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/store/schema.rs` (Task 01 영역)
- `crates/secall-core/src/store/db.rs` (Task 01 영역)
- `crates/secall-core/src/ingest/types.rs` (Task 03 영역)
