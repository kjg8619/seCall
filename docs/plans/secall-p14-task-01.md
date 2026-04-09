---
type: task
status: draft
plan: secall-p14
task_number: 1
title: "벡터 검색 독립 실행"
parallel_group: A
depends_on: []
updated_at: 2026-04-09
---

# Task 01: 벡터 검색 독립 실행

## 문제

`hybrid.rs:92-106`에서 BM25 결과의 session_id를 추출하여 벡터 검색 범위를 제한한다:

```rust
// 현재 코드 (hybrid.rs:92-106)
let candidate_ids: Vec<String> = { /* BM25 결과에서 추출 */ };
let ids_opt = if candidate_ids.is_empty() {
    None  // BM25 결과 없음 → 전체 검색 (실제로는 거의 발생 안 함)
} else {
    Some(candidate_ids.as_slice())  // ← BM25 범위로 제한
};
vi.search(db, query, candidate_limit, filters, ids_opt)
```

BM25에 매칭되지 않지만 의미적으로 관련된 세션은 벡터 검색에서도 누락된다.

## Changed files

| 파일 | 변경 | 비고 |
|---|---|---|
| `crates/secall-core/src/search/hybrid.rs:80-122` | 수정 | `search()` 메서드의 벡터 검색 호출 변경 |

## Change description

### Step 1: 벡터 검색을 BM25와 독립 실행

`SearchEngine::search()` 메서드(hybrid.rs:80-122)를 수정:

```rust
pub async fn search(
    &self,
    db: &Database,
    query: &str,
    filters: &SearchFilters,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let candidate_limit = limit * 3;

    let bm25_results = self.bm25.search(db, query, candidate_limit, filters)?;

    // 벡터 검색을 독립 실행 (BM25 결과로 범위 제한하지 않음)
    let vector_results = if let Some(vi) = &self.vector {
        vi.search(db, query, candidate_limit, filters, None)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if bm25_results.is_empty() && vector_results.is_empty() {
        return Ok(Vec::new());
    }

    if vector_results.is_empty() {
        return Ok(bm25_results.into_iter().take(limit).collect());
    }

    if bm25_results.is_empty() {
        return Ok(vector_results.into_iter().take(limit).collect());
    }

    let mut combined = reciprocal_rank_fusion(&bm25_results, &vector_results, RRF_K);
    combined.truncate(limit);
    Ok(combined)
}
```

**핵심 변경:**
1. `candidate_ids` 추출 로직 전체 제거 (92-99행)
2. 벡터 검색 호출 시 `ids_opt` 대신 `None` 전달 — 전체 범위 검색
3. BM25만 있는 경우 / 벡터만 있는 경우 / 둘 다 있는 경우 분기 정리

### Step 2: 기존 테스트 수정 + 새 테스트 추가

`hybrid.rs`의 기존 테스트(`test_rrf_basic` 등)는 `reciprocal_rank_fusion()` 함수 단위 테스트이므로 변경 불필요.

새 테스트 추가:

```rust
#[test]
fn test_rrf_vector_only_returns_results() {
    // 벡터 결과만 있고 BM25 결과가 비어있을 때도 결과 반환 확인
    let vector = vec![
        make_result("X", 0, 1.0),
        make_result("Y", 0, 0.5),
    ];
    let combined = reciprocal_rank_fusion(&[], &vector, RRF_K);
    assert_eq!(combined.len(), 2);
    assert_eq!(combined[0].session_id, "X");
    // 점수가 정규화되어 있는지 확인
    assert!((combined[0].score - 1.0).abs() < 0.01);
}
```

> 참고: 이 테스트는 이미 `test_rrf_vector_only`로 존재 (hybrid.rs:310-316). search() 메서드의 분기 변경은 통합 테스트로 검증.

## Dependencies

- 없음 (독립 작업)

## Verification

```bash
# 1. 컴파일 확인
cargo check --all

# 2. 기존 테스트 전부 통과
cargo test --all

# 3. hybrid 모듈 테스트 집중 확인
cargo test -p secall-core -- search::hybrid --nocapture

# 4. clippy 경고 없음
cargo clippy --all -- -D warnings
```

## Risks

- **벡터 검색 범위 확대 → BLOB 스캔 시 latency 증가**: ANN(usearch) 사용 시 O(log n)이라 영향 미미. BLOB fallback 시 O(n)이지만, 현재 BM25 결과가 없을 때도 `None`으로 전체 스캔하는 코드 경로(`hybrid.rs:102-103`)가 이미 존재하므로 새로운 위험이 아님.
- **결과 수 증가**: 벡터 전용 결과가 추가되므로 RRF 합산 대상이 늘어남. `truncate(limit)`으로 최종 결과 수는 동일.

## Scope boundary

다음 파일은 이 task에서 수정하지 않음:
- `crates/secall-core/src/search/vector.rs` — 벡터 검색 내부 로직 변경 없음
- `crates/secall-core/src/search/bm25.rs` — BM25 내부 로직 변경 없음
- `crates/secall-core/src/mcp/server.rs` — MCP 서버는 `search()` 호출만 하므로 자동 적용
- `crates/secall/src/commands/recall.rs` — CLI도 `search()` 호출만 하므로 자동 적용
