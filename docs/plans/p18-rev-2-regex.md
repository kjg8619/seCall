---
type: plan
status: in_progress
updated_at: 2026-04-11
supersedes: docs/plans/p18-config.md
---

# P18 Rev.2 — 세션 분류 시스템 (regex 에러 처리 강화)

## Description

P18 구현 3차 리뷰 실패의 근본 원인 수정.

`classify.rs`와 `ingest.rs`에서 regex 패턴을 매 세션 루프마다 재컴파일하면서
`.ok()`로 컴파일 오류를 무음 무시한다. 잘못된 패턴은 항상
`classification.default`로 폴백되어 분류 오류가 발생해도 사용자에게 전달되지 않는다.

이번 Rev.2는 단 하나의 Task로 이 문제를 수정한다:

- 세션 루프 진입 전 모든 rules의 regex를 일괄 컴파일 (pre-compile)
- 컴파일 실패 시 즉시 `Err` 반환 (fast-fail, `.ok()` 제거)

## Expected Outcome

- `.secall.toml`에 유효하지 않은 regex 패턴이 있으면 `secall ingest` 또는
  `secall classify --backfill` 실행 즉시 오류 메시지 출력 후 종료
- 유효한 패턴의 경우 분류 동작은 P18 원본과 동일
- `cargo test -p secall-core -p secall` 전체 통과

## Subtasks

| # | 제목 | 파일 |
|---|------|------|
| 01 | Regex 사전 컴파일 및 에러 전파 | `classify.rs`, `ingest.rs` |

## Constraints

- Task 01만 수정
- P18 Task 01~05에서 구현된 DB 스키마, Config 구조체, BM25/벡터 필터, backfill 쿼리는 건드리지 않는다

## Non-goals

- `ClassificationConfig` / `ClassificationRule` 구조체 변경 없음
- DB 스키마 변경 없음
- `SearchFilters` 변경 없음
- `ingest.rs` 호출 구조 외 기타 리팩토링 없음
