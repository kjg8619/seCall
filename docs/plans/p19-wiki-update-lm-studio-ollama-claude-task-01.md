---
type: task
status: pending
plan: p19-wiki-update-lm-studio-ollama-claude
task: 01
updated_at: 2026-04-11
---

# Task 01 — `WikiBackend` trait 정의 + 구현체 3개

## Changed files

- `crates/secall-core/src/wiki/mod.rs` (신규) — `WikiBackend` trait, pub use 정리
- `crates/secall-core/src/wiki/claude.rs` (신규) — `ClaudeBackend` (기존 subprocess 로직 이전)
- `crates/secall-core/src/wiki/ollama.rs` (신규) — `OllamaBackend` (reqwest `/api/generate`)
- `crates/secall-core/src/wiki/lmstudio.rs` (신규) — `LmStudioBackend` (OpenAI-compatible `/v1/chat/completions`)
- `crates/secall-core/src/lib.rs:9` — `pub mod wiki;` 추가

## Change description

### 1. lib.rs — wiki 모듈 등록

```rust
// lib.rs:9 에 추가
pub mod wiki;
```

### 2. wiki/mod.rs — trait 정의 + pub use

```rust
pub mod claude;
pub mod lmstudio;
pub mod ollama;

pub use claude::ClaudeBackend;
pub use lmstudio::LmStudioBackend;
pub use ollama::OllamaBackend;

use crate::error::Result;

/// wiki 생성 프롬프트를 LLM에 전달하고 결과를 반환하는 추상 인터페이스
#[async_trait::async_trait]
pub trait WikiBackend: Send + Sync {
    /// 프롬프트를 전달하고 LLM 응답 텍스트를 반환한다.
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// 백엔드 이름 (로그/표시용)
    fn name(&self) -> &'static str;
}
```

> `async_trait` crate 필요 여부 확인: `secall-core/Cargo.toml`에 없으면 추가. 또는 `async fn`을 반환 타입 `Pin<Box<dyn Future>>` 으로 수동 구현하여 의존성 회피 가능.

### 3. wiki/claude.rs — 기존 subprocess 로직 이전

`commands/wiki.rs`의 subprocess 실행 부분(line 35-100)을 이 파일로 이전한다.

```rust
pub struct ClaudeBackend {
    pub model: String,  // "sonnet" | "opus"
}

#[async_trait::async_trait]
impl WikiBackend for ClaudeBackend {
    fn name(&self) -> &'static str { "claude" }

    async fn generate(&self, prompt: &str) -> Result<String> {
        use std::io::{BufRead, Write as _};
        use std::process::Stdio;

        if !crate::command_exists("claude") {
            anyhow::bail!(
                "Claude Code CLI not found in PATH. \
                 Install: https://docs.anthropic.com/claude-code"
            );
        }

        let model_id = match self.model.as_str() {
            "opus" => "claude-opus-4-6",
            _ => "claude-sonnet-4-6",
        };

        let mut child = std::process::Command::new("claude")
            .args(["-p", "--model", model_id])
            .arg("--allowedTools")
            .arg("mcp__secall__recall,mcp__secall__get,mcp__secall__status,mcp__secall__wiki_search,Read,Write,Edit,Glob,Grep")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes())?;
        }

        let output = if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);
            let mut lines = Vec::new();
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        eprintln!("  | {}", l);
                        lines.push(l);
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to read claude stdout");
                        break;
                    }
                }
            }
            lines.join("\n")
        } else {
            String::new()
        };

        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("claude exited with code {:?}", status.code());
        }

        Ok(output)
    }
}
```

> **주의**: `ClaudeBackend::generate()`는 `current_dir` 설정이 필요함. vault path를 인자로 받거나 `generate_in_dir(&self, prompt: &str, dir: &Path)` 시그니처로 확장 검토.

### 4. wiki/ollama.rs — Ollama 백엔드

`OllamaEmbedder`(embedding.rs:5-85)의 reqwest 패턴을 참조해 구현한다.

```rust
pub struct OllamaBackend {
    pub api_url: String,   // "http://localhost:11434"
    pub model: String,     // "gemma3:27b"
    pub max_tokens: u32,   // 기본 4096
}

#[async_trait::async_trait]
impl WikiBackend for OllamaBackend {
    fn name(&self) -> &'static str { "ollama" }

    async fn generate(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/generate", self.api_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false,
                "options": { "num_predict": self.max_tokens }
            }))
            .send()
            .await
            .map_err(|e| crate::SecallError::Other(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error: {body}");
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| crate::SecallError::Other(e.to_string()))?;

        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Ollama response missing 'response' field").into())
    }
}
```

### 5. wiki/lmstudio.rs — LM Studio 백엔드 (OpenAI compatible)

```rust
pub struct LmStudioBackend {
    pub api_url: String,   // "http://localhost:1234"
    pub model: String,     // "lmstudio-community/gemma-4-e4b-it"
    pub max_tokens: u32,   // 기본 3000
}

#[async_trait::async_trait]
impl WikiBackend for LmStudioBackend {
    fn name(&self) -> &'static str { "lmstudio" }

    async fn generate(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/v1/chat/completions", self.api_url))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": self.max_tokens,
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| crate::SecallError::Other(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LM Studio API error: {body}");
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| crate::SecallError::Other(e.to_string()))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("LM Studio response missing content field").into())
    }
}
```

## Dependencies

- `reqwest.workspace = true` — 이미 `crates/secall-core/Cargo.toml:19`에 있음
- `serde_json.workspace = true` — 이미 있음
- `async_trait` — 없으면 `Cargo.toml`에 `async-trait = "0.1"` 추가 필요. 또는 `std::future::Future` + `Pin<Box<...>>` 수동 구현으로 회피
- Task 02와 독립적으로 진행 가능

## Verification

```bash
cargo check -p secall-core
cargo test -p secall-core -- wiki --nocapture
```

수동 검증 (Ollama 로컬 실행 중일 때):
```bash
# Manual: OllamaBackend::generate("hello") 호출이 텍스트 응답 반환하는지 단위 테스트 작성 권장
```

## Risks

- `async_trait` crate 추가 시 컴파일 시간 소폭 증가. 회피하려면 `Box<dyn Future + Send>` 반환 타입으로 수동 구현
- `ClaudeBackend`는 `current_dir`를 subprocess에 설정해야 함. vault path를 `generate()` 시그니처에 포함하거나 `ClaudeBackend` 생성 시 주입하는 방식 중 선택 필요. Task 03 담당자와 합의 필요
- Ollama `stream: false` 옵션이 응답 전까지 HTTP 연결을 유지함 → 대용량 wiki update 시 타임아웃 발생 가능. `reqwest::ClientBuilder::timeout()` 설정 검토

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/vault/config.rs` (Task 02 영역)
- `crates/secall/src/commands/wiki.rs` (Task 03 영역)
- `crates/secall/src/main.rs` (Task 03 영역)
