# Review Report: P24 — GitHub 이슈 일괄 수정 (#19, #21, #22, #23) — Round 1

> Verdict: fail
> Reviewer: 
> Date: 2026-04-14 15:46
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. docs/plans/p24-github-19-21-22-23-task-02.md:15 — Task 02의 Changed files 계약에는 `crates/secall/src/main.rs` 수정이 필수로 적혀 있지만, 실제 작업 트리에는 해당 파일 변경이 없어 계약이 충족되지 않았습니다.
2. crates/secall-core/src/store/session_repo.rs:675 — `--since 2026-04-10`을 `2026-04-10T00:00:00+00:00`로 정규화하면 `2026-04-09T15:00:00Z`처럼 KST로는 2026-04-10인 세션이 제외됩니다. Task 03 문서가 해결 대상으로 적은 타임존 누락 버그가 그대로 남아 있습니다.
3. docs/plans/p24-github-19-21-22-23-task-04.md:15 — Task 04의 Changed files 계약에는 `crates/secall-core/src/ingest/claude.rs` 수정이 필수로 적혀 있지만, 실제 구현은 `crates/secall/src/commands/ingest.rs`와 `session_repo.rs`만 변경했고 `claude.rs`는 수정되지 않았습니다.

## Recommendations

1. Task 02는 task 문서를 실제 구현 범위에 맞게 정정하거나, 정말 계약대로 진행할 것이라면 `main.rs`에 검증 가능한 변경을 추가하세요.
2. Task 03는 입력 날짜를 UTC로 고정하지 말고 사용자 기대 타임존 기준 경계로 해석하도록 정리한 뒤, `2026-04-09T15:00:00Z`가 포함되는 회귀 테스트를 추가하세요.
3. Task 04는 task 문서의 Changed files를 실제 수정 전략과 일치시키거나, 원안대로 `claude.rs`를 수정해야 하는지 다시 결정한 뒤 재검토를 요청하세요.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | FTS5 중복 행 수정 | ✅ done |
| 2 | --local-only pull 스킵 + help 수정 | ✅ done |
| 3 | wiki --since 타임존 정규화 | ✅ done |
| 4 | compact 이후 이전 turn 누락 | ✅ done |

