---
type: plan
status: in_progress
updated_at: 2026-04-15
github_issue: "#25"
---

# P27 — BM25-only 선택 시 graph semantic 자동 비활성화 (#25)

## 배경

`secall init`에서 `embedding.backend = none` (BM25만 사용)을 선택해도
`[graph]` 설정이 기본값(`semantic = true`, `semantic_backend = "ollama"`)으로 남아서
`secall ingest` 시 Ollama 호출 → WARN 발생.

사용자 기대: "BM25만 사용" 선택 시 Ollama 의존 기능 전체가 꺼져야 함.

## 수정 방향

1. `init.rs`: `embedding.backend = none` 선택 시 `graph.semantic = false` 자동 설정
2. `ingest.rs`: semantic 진입 조건에 방어 로직 추가 (수동 설정 파일 편집 대응)

## Subtasks

| # | 제목 | 파일 |
|---|------|------|
| 1 | init.rs에서 BM25-only 선택 시 graph.semantic = false 자동 설정 | `init.rs` |
| 2 | ingest.rs 방어 로직 + 전체 테스트 검증 | `ingest.rs` |

## Expected Outcome

- `secall init`에서 BM25-only 선택 → `graph.semantic = false` 자동 설정
- `secall ingest` 시 Ollama WARN 발생하지 않음
- 기존에 `semantic = true`로 명시 설정한 사용자는 영향 없음
- `cargo test` 전체 통과

## Non-goals

- `secall.toml` 마이그레이션 (기존 설정 파일 자동 수정)
- `secall init`에 graph 설정 별도 질문 추가
- #26 Codex wiki backend (별도 PR 대기)

## Version

- v1 — 2026-04-15 초안
