---
type: task
status: pending
plan: p27-bm25-only-graph-semantic-25
task_number: 2
title: "ingest.rs 방어 로직 + 전체 테스트 검증"
parallel_group: 1
depends_on: [1]
updated_at: 2026-04-15
---

# Task 02 — ingest.rs 방어 로직 + 전체 테스트 검증

## Changed files

- `crates/secall/src/commands/ingest.rs:365`

## Change description

`ingest.rs`의 시맨틱 엣지 추출 진입 조건에 방어 로직을 추가한다.
사용자가 `secall.toml`을 수동 편집하여 `graph.semantic = true`이면서
`embedding.backend = "none"`인 불일치 상태를 만들 수 있으므로,
`semantic_backend`가 `"disabled"`인 경우에도 LLM 호출을 건너뛰도록 보강한다.

### 구현 단계

1. `ingest.rs:365` — 기존 조건:

```rust
if config.graph.semantic && !no_semantic && !new_session_ids.is_empty() {
```

이를 다음으로 변경:

```rust
let semantic_enabled = config.graph.semantic
    && config.graph.semantic_backend != "disabled"
    && !no_semantic
    && !new_session_ids.is_empty();
if semantic_enabled {
```

2. 이렇게 하면 `semantic_backend = "disabled"`일 때도 LLM 호출 블록 전체를 건너뛴다.
   기존 `semantic.rs:282`의 내부 가드와 이중 방어가 되어, Ollama 모델 언로드 시도 등 불필요한 네트워크 호출도 방지된다.

## Dependencies

- Task 01 완료 후 진행 (init이 올바른 설정을 생성해야 방어 로직의 의미가 명확)

## Verification

```bash
cargo check -p secall 2>&1 | tail -5
```

```bash
cargo test 2>&1 | tail -30
```

전체 `cargo test`를 실행하여 regression이 없는지 확인한다.
사용자 요청: "반드시 이슈처리하고 테스트 돌려봐야해"

## Risks

- `semantic_backend` 필드가 빈 문자열일 가능성 → `config.rs:170`에서 기본값 `"ollama"` 설정되므로 빈 문자열 불가
- 조건 변경으로 기존에 `semantic = true` + `semantic_backend = "ollama"`인 정상 사용자가 영향받을 가능성 → `"ollama" != "disabled"`이므로 영향 없음

## Scope boundary

수정 금지 파일:
- `crates/secall/src/commands/init.rs` — Task 01 영역
- `crates/secall-core/src/vault/config.rs` — 기본값 변경 불필요
- `crates/secall-core/src/graph/semantic.rs` — 내부 가드 유지, 수정 불필요
