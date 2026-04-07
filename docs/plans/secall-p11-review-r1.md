# Review Report: seCall P11 — 임베딩 성능 최적화 — Round 1

> Verdict: fail
> Reviewer: 
> Date: 2026-04-07 18:26
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall-core/src/search/vector.rs:93 — `index_session()`가 배치 임베딩 실패(`embed_errors`)나 개별 INSERT 실패(`insert_errors`)를 기록만 하고 트랜잭션 클로저는 끝까지 `Ok(())`를 반환합니다. 그래서 세션이 일부 청크만 INSERT된 상태로 커밋될 수 있습니다. 그런데 `crates/secall-core/src/store/db.rs:258`의 `find_sessions_without_vectors()`는 여전히 `turn_vectors`에 행이 1개라도 있으면 완료로 간주하므로, 부분 인덱싱된 세션이 재시작 후 영구히 스킵됩니다. 이는 Task 01의 "중단 후 재시작 시 중복/누락 없음" 계약을 직접 위반합니다.
2. crates/secall/src/commands/embed.rs:120 — ANN 인덱스를 전체 작업 종료 후 한 번만 저장합니다. 따라서 세션 벡터는 DB에 커밋됐더라도 프로세스가 중간에 중단되면 ANN 파일은 이전 상태로 남습니다. 다음 실행에서 `crates/secall-core/src/search/vector.rs:326`이 그 오래된 ANN 파일을 다시 로드하고, 검색은 `crates/secall-core/src/search/vector.rs:163`에서 ANN 결과를 우선 사용하며 DB 선형 스캔으로 보정하지 않습니다. 결과적으로 이미 커밋된 벡터가 검색에서 누락될 수 있어 Task 03의 재시작 안전성과 검색 무사이드이펙트 요구를 만족하지 못합니다.

## Recommendations

1. 세션 단위 원자성을 실제로 보장하려면, 어떤 청크라도 임베딩/INSERT에 실패하면 해당 세션 트랜잭션 전체를 실패시켜 롤백하거나, 최소한 완료 판정을 `expected chunk count == stored vector count` 기준으로 바꾸어 부분 세션을 다시 집계해야 합니다.
2. ANN은 DB와 동일한 복구 단위를 가져야 합니다. 중단 후에도 검색 누락이 없게 하려면 주기 저장 또는 종료 시그널 저장만으로 끝내지 말고, 시작 시 DB에서 ANN을 재구축하거나 stale/missing 상태를 감지해 BLOB 스캔으로 안전하게 폴백하는 경로가 필요합니다.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | DB 트랜잭션 + 중단-재시작 안전성 | ✅ done |
| 2 | ORT 진짜 batch inference | ✅ done |
| 3 | 세션 병렬 처리 + ANN 저장 빈도 감소 | ✅ done |
| 4 | batch_size CLI 연결 + 진행률 표시 개선 | ✅ done |

