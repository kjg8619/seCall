---
type: task
status: pending
plan: p27-bm25-only-graph-semantic-25
task_number: 1
title: "init.rs에서 BM25-only 선택 시 graph.semantic = false 자동 설정"
parallel_group: 1
depends_on: []
updated_at: 2026-04-15
---

# Task 01 — init.rs에서 BM25-only 선택 시 graph.semantic = false 자동 설정

## Changed files

- `crates/secall/src/commands/init.rs:162-166`

## Change description

`init.rs` Step 5 (임베딩 백엔드 선택) 영역에서 `embedding.backend = "none"` 분기 직후에
`graph.semantic = false`를 함께 설정한다.

### 구현 단계

1. `init.rs:162-166` — `config.embedding.backend` 설정 블록 직후 (167행 부근)에 다음 로직 추가:

```rust
// BM25-only 선택 시 시맨틱 그래프 추출도 비활성화
if config.embedding.backend == "none" {
    config.graph.semantic = false;
    println!("  → BM25만 사용: 시맨틱 그래프 추출도 비활성화됩니다.");
}
```

2. 반대로 `ollama` 선택 시에는 기존 기본값(`semantic = true`)을 유지하므로 별도 처리 불필요.

## Dependencies

- 없음 (독립 작업)

## Verification

```bash
cargo check -p secall 2>&1 | tail -5
```

```bash
cargo test -p secall -- init 2>&1 | tail -20
```

```bash
# Manual: secall init 실행 → "none (BM25만 사용)" 선택 →
# 생성된 secall.toml에서 graph.semantic = false 확인
```

## Risks

- `config.graph` 필드가 `init.rs` 시점에 아직 초기화되지 않았을 가능성 → `GraphConfig::default()`에서 `semantic: true`로 초기화되므로 문제 없음 (config.rs:169 확인됨)
- `secall init --non-interactive` 경로가 있다면 동일 로직 적용 필요 → 현재 non-interactive 모드 없음

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/vault/config.rs` — 기본값 변경 불필요
- `crates/secall-core/src/graph/semantic.rs` — 기존 `semantic_backend != "disabled"` 가드 충분
- `crates/secall/src/commands/ingest.rs` — Task 02 영역
