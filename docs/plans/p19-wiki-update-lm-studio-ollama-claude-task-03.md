---
type: task
status: pending
plan: p19-wiki-update-lm-studio-ollama-claude
task: 03
updated_at: 2026-04-11
---

# Task 03 — CLI `--backend` 플래그 + `run_update()` 연결

## Changed files

- `crates/secall/src/commands/wiki.rs:6-104` — `run_update()` 시그니처에 `backend: Option<&str>` 추가, subprocess 로직을 `WikiBackend::generate()` 호출로 교체, 백엔드 선택 로직 추가
- `crates/secall/src/main.rs:218-237` — `WikiAction::Update`에 `--backend` 플래그 추가
- `crates/secall/src/main.rs:363-371` — 핸들러에서 `backend` 파라미터 전달
- `crates/secall/Cargo.toml` — `secall-core`의 `wiki` 모듈을 통해 간접 사용 (직접 dep 추가 없음)

## Change description

### 1. main.rs — `WikiAction::Update`에 `--backend` 추가

`WikiAction::Update` 열거형 변형(line 220):

```rust
WikiAction::Update {
    /// Model: opus or sonnet (Claude 백엔드 전용)
    #[arg(long, default_value = "sonnet")]
    model: String,

    /// Backend: claude | ollama | lmstudio (기본값: config wiki.default_backend)
    #[arg(long)]
    backend: Option<String>,

    /// Only process sessions since this date (YYYY-MM-DD)
    #[arg(long)]
    since: Option<String>,

    /// Incremental mode: update for a specific session
    #[arg(long)]
    session: Option<String>,

    /// Print the prompt without executing
    #[arg(long)]
    dry_run: bool,
},
```

핸들러(line 363):
```rust
WikiAction::Update { model, backend, since, session, dry_run } => {
    commands::wiki::run_update(
        &model,
        backend.as_deref(),
        since.as_deref(),
        session.as_deref(),
        dry_run,
    ).await?;
}
```

### 2. wiki.rs — `run_update()` 리팩토링

#### 2-1. 시그니처 변경

```rust
pub async fn run_update(
    model: &str,
    backend: Option<&str>,   // 추가
    since: Option<&str>,
    session: Option<&str>,
    dry_run: bool,
) -> Result<()> {
```

#### 2-2. 백엔드 선택 로직 (기존 subprocess 로직 대체)

step 3(dry-run) 이후, 기존 step 4-5 전체를 다음으로 교체:

```rust
// 3. dry-run: print prompt and exit
if dry_run {
    println!("{prompt}");
    return Ok(());
}

// 4. 백엔드 선택: --backend 플래그 → config wiki.default_backend → "claude"
let backend_name = backend
    .map(|s| s.to_string())
    .unwrap_or_else(|| config.wiki.default_backend.clone());

let target = if let Some(sid) = session {
    format!("session {}", &sid[..sid.len().min(8)])
} else {
    "all sessions".to_string()
};
eprintln!("Wiki update: {} (backend: {})", target, backend_name);

// 5. WikiBackend 인스턴스 생성
let backend_box: Box<dyn secall_core::wiki::WikiBackend> = match backend_name.as_str() {
    "ollama" => {
        let cfg = config.wiki_backend_config("ollama");
        Box::new(secall_core::wiki::OllamaBackend {
            api_url: cfg.api_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: cfg.model.unwrap_or_else(|| "llama3".to_string()),
            max_tokens: cfg.max_tokens,
        })
    }
    "lmstudio" => {
        let cfg = config.wiki_backend_config("lmstudio");
        Box::new(secall_core::wiki::LmStudioBackend {
            api_url: cfg.api_url.unwrap_or_else(|| "http://localhost:1234".to_string()),
            model: cfg.model.unwrap_or_else(|| "local-model".to_string()),
            max_tokens: cfg.max_tokens,
        })
    }
    "claude" | _ => {
        if backend.is_some() && backend_name != "claude" {
            anyhow::bail!("Unknown backend '{}'. Supported: claude, ollama, lmstudio", backend_name);
        }
        Box::new(secall_core::wiki::ClaudeBackend {
            model: model.to_string(),
        })
    }
};

// 6. 생성 실행
eprintln!("  Launching {}...", backend_box.name());
let output = backend_box.generate(&prompt).await?;

eprintln!("  ✓ Wiki update complete.");
if !output.trim().is_empty() {
    tracing::debug!(output_len = output.len(), backend = backend_name, "wiki backend produced output");
}

Ok(())
```

#### 2-3. ClaudeBackend의 current_dir 처리

`ClaudeBackend::generate()`가 subprocess에 `current_dir`를 설정해야 한다. Task 01에서 `ClaudeBackend`를 설계할 때 vault path 주입 방식을 선택했다면, 이 파일에서 `config.vault.path`를 `ClaudeBackend` 생성자에 전달한다:

```rust
// current_dir를 ClaudeBackend에 주입하는 경우
Box::new(secall_core::wiki::ClaudeBackend {
    model: model.to_string(),
    vault_path: config.vault.path.clone(),
})
```

Task 01 담당자와 `ClaudeBackend` 인터페이스를 합의한 후 구현한다.

## Dependencies

- Task 01 완료 (`WikiBackend` trait, 구현체 3개 존재)
- Task 02 완료 (`WikiConfig`, `wiki_backend_config()` 메서드 사용)

## Verification

```bash
cargo check -p secall
cargo build -p secall 2>&1 | tail -5
```

수동 검증:
```bash
# Manual: Claude 백엔드 (기존 동작 유지 확인)
# secall wiki update --dry-run --session <id>
# → 프롬프트 출력만 하고 종료

# Manual: 알 수 없는 백엔드 에러 처리 확인
# secall wiki update --backend unknown
# → "Unknown backend 'unknown'" 에러 출력
```

## Risks

- `ClaudeBackend`의 `current_dir` 처리 방식이 Task 01과 이 Task 간 인터페이스 합의 없이 구현되면 컴파일 오류 발생. Task 01 먼저 완료 후 진행 권장.
- `backend_name.as_str()`의 `_ =>` 기본 분기가 Claude를 반환하므로, 오타가 있는 백엔드 이름도 조용히 Claude로 fallback될 수 있음. 명시적 에러 처리 로직을 위 코드에 포함했으나 `backend.is_some()` 조건 체크로만 구분되므로 로직 주의.
- `secall` crate에 `reqwest`가 없으므로 backend 인스턴스를 직접 생성하는 코드는 `secall-core` 타입만 사용해야 함. 이미 `secall_core::wiki::OllamaBackend { ... }` 형태로 설계돼 있어 문제없음.

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/wiki/` (Task 01 영역 — 인터페이스 변경은 Task 01 담당자와 합의 필요)
- `crates/secall-core/src/vault/config.rs` (Task 02 영역)
- `crates/secall-core/src/search/` (이 플랜과 무관)
- `crates/secall-core/src/mcp/` (이 플랜과 무관)
