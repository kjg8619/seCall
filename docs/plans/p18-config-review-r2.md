# Review Report: P18 — 세션 분류 시스템 (config 기반) — Round 2

> Verdict: fail
> Reviewer: 
> Date: 2026-04-11 21:05
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall-core/src/search/vector.rs:311 — `exclude_session_types`가 vector 검색 경로에서 무시되어 semantic/MCP 검색에 automated 세션이 섞입니다.
2. crates/secall-core/src/store/db.rs:623 — backfill 대상 조회가 user turn 없는 세션을 누락시켜 일부 기존 세션이 재분류되지 않습니다.

## Recommendations

1. `exclude_session_types`를 vector 경로까지 전달할 수 있게 `SessionMeta` 또는 별도 조회에 `session_type`를 포함하고, semantic-only 검색 회귀 테스트를 추가하세요.
2. backfill 쿼리를 `LEFT JOIN` 또는 세션 전체 순회 방식으로 바꿔 user turn이 없는 세션도 `classification.default`로 처리되게 하고, 해당 케이스 테스트를 추가하세요.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | DB 스키마 v4 | ✅ done |
| 2 | Config | ✅ done |
| 3 | Ingest | ✅ done |
| 4 | Search | ✅ done |
| 5 | Backfill 명령 | ✅ done |

