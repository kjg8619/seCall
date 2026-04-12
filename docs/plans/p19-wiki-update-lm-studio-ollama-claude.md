---
type: plan
status: in_progress
slug: p19-wiki-update-lm-studio-ollama-claude
updated_at: 2026-04-11
---

# Plan: P19 — wiki update 백엔드 선택 (LM Studio / Ollama / Claude)

## Description

`secall wiki update`가 `claude -p` subprocess로 고정돼 있어 토큰 비용이 크고 로컬 LLM 사용이 불가한 문제를 해결한다. `WikiBackend` trait으로 실행 레이어를 추상화하고 LM Studio(OpenAI 호환), Ollama(`/api/generate`), Claude(기존 subprocess) 세 가지 구현체를 제공한다. CLI `--backend` 플래그로 선택하며, config에서 기본값 설정 가능하다.

## Expected Outcome

```bash
secall wiki update --backend lmstudio --session abc123
secall wiki update --backend ollama --since 2026-04-01
secall wiki update --backend claude          # 기본값, 현재 동작과 동일
secall wiki update                           # config wiki.default_backend 참조
```

```toml
# .secall.toml
[wiki]
default_backend = "lmstudio"

[wiki.backends.lmstudio]
api_url = "http://localhost:1234"
model = "lmstudio-community/gemma-4-e4b-it"
max_tokens = 3000

[wiki.backends.ollama]
api_url = "http://localhost:11434"
model = "gemma3:27b"

[wiki.backends.claude]
model = "sonnet"
```

## Subtasks

| # | 제목 | 파일 | 의존성 |
|---|------|------|--------|
| 01 | `WikiBackend` trait + 구현체 3개 | secall-core/src/wiki/ (신규) | 없음 |
| 02 | Config — `WikiConfig` 추가 | vault/config.rs | 없음 |
| 03 | CLI `--backend` 플래그 + `run_update()` 연결 | wiki.rs, main.rs | Task 01, 02 |

## Constraints

- `reqwest`는 `secall-core`의 workspace dep으로 이미 있음 (`crates/secall-core/Cargo.toml:19`)
- `serde_json`도 이미 있음
- `secall` 바이너리 crate에는 `reqwest`가 없음 → backend 구현은 `secall-core`에 위치
- Claude 백엔드는 subprocess 유지 (MCP tool 접근 필요, HTTP API로 대체 불가)
- Ollama/LM Studio 백엔드는 1차에서 non-streaming (단순 구현 우선)

## Non-goals

- turn 압축/context window 관리 (별도 이슈)
- 스트리밍 출력 지원 (Ollama/LM Studio)
- 멀티에이전트 오케스트레이션
- OpenAI API 직접 지원 (LM Studio가 OpenAI 호환이므로 커버됨)
