# Review Report: seCall P11 — 임베딩 성능 최적화 — Round 2

> Verdict: fail
> Reviewer: 
> Date: 2026-04-07 18:35
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall-core/src/search/vector.rs:170 — ANN freshness를 `ann.size() >= db_count`로만 판단하지만, `index_session()`는 기존 DB row를 삭제 후 새 rowid로 다시 넣고(`crates/secall-core/src/search/vector.rs:104-123`), `AnnIndex`는 삭제 API 없이 add만 지원합니다(`crates/secall-core/src/search/ann.rs:58`). 그래서 `secall embed --all` 또는 재임베딩 이후 ANN에는 삭제된 옛 rowid가 남은 채 size가 DB보다 크거나 같을 수 있고, 이 경우 검색이 계속 ANN 경로를 탑니다. 이후 stale rowid는 `db.get_vector_meta(...).ok()?`에서 조용히 버려지지만 BLOB 스캔으로 보충하지 않으므로, 실제로 커밋된 최신 벡터가 검색 결과에서 누락될 수 있습니다.

## Recommendations

1. ANN 파일을 신뢰하기 전에 DB 기준으로 재구축하거나, stale rowid가 하나라도 나오면 즉시 BLOB 스캔으로 폴백해 결과를 재계산하세요.
2. 재임베딩/`--all` 경로에서는 add-only ANN 구조를 유지하지 말고, 세션 삭제가 발생한 시점에 ANN 전체 rebuild 또는 새 파일 교체 전략을 명시적으로 적용하는 편이 안전합니다.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | DB 트랜잭션 + 중단-재시작 안전성 | ✅ done |
| 2 | ORT 진짜 batch inference | ✅ done |
| 3 | 세션 병렬 처리 + ANN 저장 빈도 감소 | ✅ done |
| 4 | batch_size CLI 연결 + 진행률 표시 개선 | ✅ done |

