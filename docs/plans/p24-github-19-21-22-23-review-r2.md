# Review Report: P24 — GitHub 이슈 일괄 수정 (#19, #21, #22, #23) — Round 2

> Verdict: fail
> Reviewer: 
> Date: 2026-04-14 16:07
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall/src/commands/ingest.rs:535 — `db.count_turns_for_session(&session.id).unwrap_or(0)`가 DB 조회 오류를 0으로 숨깁니다. 이 경우 `session.turns.len() > db_turn_count + 10 && session.turns.len() > db_turn_count * 2` 조건이 쉽게 참이 되어, 실제로는 오류 상황인데 기존 세션을 삭제하고 자동 재인제스트하는 잘못된 동작이 발생할 수 있습니다.

## Recommendations

1. Task 04에서는 `count_turns_for_session()` 실패를 skip 또는 error로 명시 처리해서, auto re-ingest 판단이 DB 오류에 의해 트리거되지 않도록 하세요.
2. 다음 리워크에서는 result 문서에 task별 Verification 명령과 결과를 그대로 남겨 두면 재검토가 더 명확해집니다.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | FTS5 중복 행 수정 | ✅ done |
| 2 | --local-only pull 스킵 + help 수정 | ✅ done |
| 3 | wiki --since 타임존 정규화 | ✅ done |
| 4 | compact 이후 이전 turn 누락 | ✅ done |

