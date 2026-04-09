---
type: plan
status: draft
updated_at: 2026-04-09
version: 1
---

# seCall P14 — 검색 품질 개선

## Description

gbrain 레퍼런스 분석을 바탕으로 seCall 하이브리드 검색 파이프라인의 두 가지 핵심 약점을 개선한다.

**문제 1 — 벡터 검색이 BM25에 종속:**
현재 `hybrid.rs:92-106`에서 BM25 결과의 session_id를 추출하여 벡터 검색 범위를 제한한다.
BM25에 매칭되지 않지만 의미적으로 관련된 세션은 벡터 검색에서도 누락된다.

**문제 2 — 동일 세션 턴이 결과 독점:**
RRF 합산 후 동일 세션의 여러 턴이 상위를 점령할 수 있다.
사용자는 다양한 세션에서 결과를 보고 싶지만, 하나의 긴 대화가 결과를 지배한다.

## Expected Outcome

- `secall recall "쿼리"` 실행 시, BM25에 매칭되지 않지만 의미적으로 관련된 세션도 결과에 포함
- 같은 세션의 턴이 결과를 독점하지 않고, 다양한 세션이 골고루 노출
- MCP recall 도구에도 동일하게 적용
- 기존 테스트 전부 통과 + 새 동작 검증 테스트 추가

## Subtasks

| # | Title | 공수 | parallel_group | depends_on |
|---|-------|------|---------------|------------|
| 01 | 벡터 검색 독립 실행 | Small | A | — |
| 02 | 세션 레벨 결과 다양성 | Small | A | — |

두 작업은 독립적으로 수행 가능하나, 최종 통합 테스트는 두 작업 완료 후 수행한다.

## Constraints

- `hybrid.rs` 중심 변경 — 외부 인터페이스(CLI, MCP) 변경 최소화
- `SearchFilters`에 필드 추가 시 `Default` trait 유지 — 기존 호출자 변경 불필요
- 기존 RRF 테스트 유지 + 새 테스트 추가

## Non-goals

- Multi-Query Expansion — 현재 키워드 확장으로 충분
- Semantic Chunking — 턴 단위가 자연 경계
- Jaccard 텍스트 중복 제거 — 세션 레벨 dedup으로 충분
- 검색 파라미터 설정 파일화 — 하드코딩 유지
- MCP 서버 API 변경 — 내부 동작만 개선
