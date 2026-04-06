# Review Report: seCall P8 — 안정화 + 배포 — Round 2

> Verdict: fail
> Reviewer: 
> Date: 2026-04-07 08:00
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall/src/commands/ingest.rs:263 — 벡터 인덱싱 실패 시 `tracing::warn!`만 남기고 `errors`/`error_details`를 갱신하지 않아, task 문서가 요구한 `Indexing` 단계 에러가 summary 및 JSON 리포트에 누락됩니다.
2. crates/secall/src/commands/ingest.rs:399 — 존재하지 않는 입력 경로를 파일 경로 오류로 처리하지 않고 세션 ID 조회로 넘기기 때문에, Task 01 검증 시나리오인 `/nonexistent/path.jsonl` 입력에서 Detection 에러 리포트가 생성되지 않습니다.

## Recommendations

1. `path` 인자가 경로 구문(`/`, `.` 확장자 등)을 가지는데 실존하지 않으면 세션 ID로 해석하지 말고 명시적 경로 오류로 처리하는 것이 안전합니다.
2. [crates/secall/src/commands/sync.rs](/Users/d9ng/privateProject/seCall/crates/secall/src/commands/sync.rs) 변경은 P8 Task 01/02 계약 범위 밖이므로, 별도 plan/task로 분리하거나 task 문서의 Changed files와 Scope boundary를 갱신하는 편이 맞습니다.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | 에러 리포팅 개선 | ✅ done |
| 2 | Release 바이너리 배포 | ✅ done |

