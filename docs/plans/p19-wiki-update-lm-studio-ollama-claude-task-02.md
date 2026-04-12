---
type: task
status: pending
plan: p19-wiki-update-lm-studio-ollama-claude
task: 02
updated_at: 2026-04-11
---

# Task 02 — Config: `WikiConfig` 추가

## Changed files

- `crates/secall-core/src/vault/config.rs` — `WikiBackendConfig`, `WikiConfig` 구조체 추가, `Config`에 `wiki` 필드 추가

## Change description

### 1. 새 구조체 정의

`HooksConfig` 구조체(line 78) 아래 또는 파일 말미에 추가:

```rust
/// 개별 백엔드 설정 (LM Studio, Ollama, Claude 공용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiBackendConfig {
    /// API 엔드포인트 (Claude 백엔드는 사용 안 함)
    pub api_url: Option<String>,
    /// 모델 이름
    pub model: Option<String>,
    /// 최대 생성 토큰 수
    #[serde(default = "default_wiki_max_tokens")]
    pub max_tokens: u32,
}

fn default_wiki_max_tokens() -> u32 {
    4096
}

impl Default for WikiBackendConfig {
    fn default() -> Self {
        WikiBackendConfig {
            api_url: None,
            model: None,
            max_tokens: default_wiki_max_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiConfig {
    /// 기본 사용 백엔드: "claude" | "ollama" | "lmstudio"
    #[serde(default = "default_wiki_backend")]
    pub default_backend: String,
    /// 백엔드별 설정 맵
    #[serde(default)]
    pub backends: std::collections::HashMap<String, WikiBackendConfig>,
}

fn default_wiki_backend() -> String {
    "claude".to_string()
}

impl Default for WikiConfig {
    fn default() -> Self {
        WikiConfig {
            default_backend: default_wiki_backend(),
            backends: std::collections::HashMap::new(),
        }
    }
}
```

### 2. `Config` 구조체에 `wiki` 필드 추가

`Config` 구조체(line 8):

```rust
pub struct Config {
    pub vault: VaultConfig,
    pub ingest: IngestConfig,
    pub search: SearchConfig,
    pub hooks: HooksConfig,
    pub embedding: EmbeddingConfig,
    pub output: OutputConfig,
    pub wiki: WikiConfig,   // 추가
}
```

`impl Default for Config`에:
```rust
wiki: WikiConfig::default(),
```

### 3. 편의 메서드 추가 (선택)

`Config` impl 블록에 백엔드 설정 조회 헬퍼:

```rust
/// 특정 백엔드의 설정을 반환한다. 없으면 기본값.
pub fn wiki_backend_config(&self, name: &str) -> WikiBackendConfig {
    self.wiki.backends.get(name).cloned().unwrap_or_default()
}
```

### 4. 사용자 TOML 예시 (문서화 목적)

```toml
[wiki]
default_backend = "lmstudio"

[wiki.backends.lmstudio]
api_url = "http://localhost:1234"
model = "lmstudio-community/gemma-4-e4b-it"
max_tokens = 3000

[wiki.backends.ollama]
api_url = "http://localhost:11434"
model = "gemma3:27b"
max_tokens = 4096

[wiki.backends.claude]
model = "sonnet"  # "opus" 도 가능
```

## Dependencies

- Task 01과 독립적으로 진행 가능
- `std::collections::HashMap` — 표준 라이브러리, 추가 dep 없음

## Verification

```bash
cargo check -p secall-core
cargo test -p secall-core -- config --nocapture
```

기대 결과:
- `Config::default().wiki.default_backend == "claude"`
- 기존 `.secall.toml` (wiki 섹션 없음)을 로드해도 파싱 오류 없이 기본값 반환

## Risks

- `Config` 구조체 변경으로 `config.rs`의 `impl Default for Config` 누락 시 컴파일 오류 → `wiki: WikiConfig::default()` 반드시 추가
- 기존 사용자의 `.secall.toml`에 `[wiki]` 섹션이 없으면 `serde(default)` 덕분에 자동으로 기본값 적용됨. 하지만 `backends`의 `HashMap` 키로 존재하지 않는 백엔드 이름을 `--backend` 플래그로 지정하면 Task 03에서 에러 처리 필요
- `WikiConfig`의 `backends` 필드가 `HashMap`이므로 TOML에서 `[[wiki.backends.lmstudio]]` 배열 문법이 아닌 `[wiki.backends.lmstudio]` 테이블 문법 사용해야 함 (이미 예시에 반영됨)

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/wiki/` (Task 01 영역)
- `crates/secall/src/commands/wiki.rs` (Task 03 영역)
- `crates/secall/src/main.rs` (Task 03 영역)
