---
type: task
status: draft
plan: secall-p14
task_number: 2
title: "세션 레벨 결과 다양성"
parallel_group: A
depends_on: []
updated_at: 2026-04-09
---

# Task 02: 세션 레벨 결과 다양성

## 문제

RRF 합산 후 동일 세션의 여러 턴이 상위를 점령할 수 있다.
예: 긴 대화 세션 "A"의 턴 3, 5, 7, 12가 모두 상위 10개에 포함되면 다른 세션 결과가 밀려남.

현재 dedup은 `(session_id, turn_index)` 중복만 제거 (hybrid.rs:18-19) — 같은 세션의 다른 턴은 모두 유지.

## Changed files

| 파일 | 변경 | 비고 |
|---|---|---|
| `crates/secall-core/src/search/hybrid.rs` | 수정 | `diversify_by_session()` 함수 추가, `search()`에서 호출 |
| `crates/secall-core/src/search/bm25.rs:19-25` | 수정 | `SearchFilters`에 `max_per_session` 필드 추가 |
| `crates/secall-core/src/mcp/server.rs:138-146` | 수정 | MCP dedup 블록에 세션 다양성 적용 |

## Change description

### Step 1: SearchFilters에 max_per_session 필드 추가

`bm25.rs:19-25`:

```rust
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub project: Option<String>,
    pub agent: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    /// 세션당 최대 결과 수 (None = 제한 없음)
    pub max_per_session: Option<usize>,
}
```

`Default`에서 `max_per_session`은 `None` — 기존 호출자 변경 불필요.

### Step 2: diversify_by_session() 함수 추가

`hybrid.rs`에 새 함수:

```rust
/// 세션당 최대 N개 턴만 유지하여 결과 다양성 확보.
/// 입력은 점수 내림차순 정렬되어 있어야 함.
fn diversify_by_session(results: Vec<SearchResult>, max_per_session: usize) -> Vec<SearchResult> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    results
        .into_iter()
        .filter(|r| {
            let count = counts.entry(r.session_id.clone()).or_insert(0);
            if *count < max_per_session {
                *count += 1;
                true
            } else {
                false
            }
        })
        .collect()
}
```

### Step 3: search()에서 diversify 적용

`hybrid.rs`의 `search()` 메서드 끝부분:

```rust
let mut combined = reciprocal_rank_fusion(&bm25_results, &vector_results, RRF_K);

// 세션 다양성 적용 (기본값: 세션당 최대 2개)
let max_per = filters.max_per_session.unwrap_or(2);
combined = diversify_by_session(combined, max_per);

combined.truncate(limit);
Ok(combined)
```

**BM25-only 경로에도 적용:**

```rust
if vector_results.is_empty() {
    let mut results: Vec<_> = bm25_results.into_iter().collect();
    let max_per = filters.max_per_session.unwrap_or(2);
    results = diversify_by_session(results, max_per);
    results.truncate(limit);
    return Ok(results);
}
```

### Step 4: MCP 서버 dedup 블록에 적용

`mcp/server.rs:138-146`의 기존 dedup 로직 뒤에 다양성 적용:

```rust
// 기존 dedup
all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
let mut seen = std::collections::HashSet::new();
all_results.retain(|r| seen.insert((r.session_id.clone(), r.turn_index)));

// 세션 다양성 적용
let max_per = base_filters.max_per_session.unwrap_or(2);
let mut session_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
all_results.retain(|r| {
    let count = session_counts.entry(r.session_id.clone()).or_insert(0);
    if *count < max_per {
        *count += 1;
        true
    } else {
        false
    }
});

all_results.truncate(limit);
```

### Step 5: 테스트 추가

`hybrid.rs`에 단위 테스트:

```rust
#[test]
fn test_diversify_by_session() {
    let results = vec![
        make_result("A", 0, 1.0),
        make_result("A", 1, 0.9),
        make_result("A", 2, 0.8),
        make_result("B", 0, 0.7),
        make_result("C", 0, 0.6),
    ];
    let diversified = diversify_by_session(results, 2);
    // A는 2개만 유지, B와 C는 그대로
    assert_eq!(diversified.len(), 4);
    assert_eq!(diversified.iter().filter(|r| r.session_id == "A").count(), 2);
    assert_eq!(diversified.iter().filter(|r| r.session_id == "B").count(), 1);
    assert_eq!(diversified.iter().filter(|r| r.session_id == "C").count(), 1);
}

#[test]
fn test_diversify_max_1() {
    let results = vec![
        make_result("A", 0, 1.0),
        make_result("A", 1, 0.9),
        make_result("B", 0, 0.8),
    ];
    let diversified = diversify_by_session(results, 1);
    assert_eq!(diversified.len(), 2);
    assert_eq!(diversified[0].session_id, "A");
    assert_eq!(diversified[0].turn_index, 0); // 최고 점수 턴 유지
    assert_eq!(diversified[1].session_id, "B");
}

#[test]
fn test_diversify_no_limit() {
    let results = vec![
        make_result("A", 0, 1.0),
        make_result("A", 1, 0.9),
        make_result("A", 2, 0.8),
    ];
    // max_per_session이 충분히 크면 모든 결과 유지
    let diversified = diversify_by_session(results.clone(), 100);
    assert_eq!(diversified.len(), 3);
}
```

## Dependencies

- 없음 (Task 01과 독립)
- 단, Task 01이 `search()` 메서드를 수정하므로, 두 작업을 동시에 머지할 때 `search()` 내부 코드 충돌에 주의

## Verification

```bash
# 1. 컴파일 확인
cargo check --all

# 2. 전체 테스트 통과
cargo test --all

# 3. hybrid 모듈 테스트 (diversify 포함)
cargo test -p secall-core -- search::hybrid --nocapture

# 4. clippy 경고 없음
cargo clippy --all -- -D warnings

# 5. SearchFilters Default가 깨지지 않는지 확인 (기존 호출자 호환)
cargo test -p secall-core -- search::bm25 --nocapture
```

## Risks

- **세션당 2개 제한이 너무 공격적**: 특정 세션에 정답이 집중된 경우 관련 턴을 놓칠 수 있음. 기본값 2는 gbrain의 page_cap(2)과 동일. `max_per_session: None`으로 비활성화 가능.
- **SearchFilters 필드 추가**: `Default` trait 유지하므로 기존 `SearchFilters { project: ..., ..Default::default() }` 패턴이 깨지지 않음.
- **MCP 서버 중복 로직**: `diversify_by_session()`을 `pub`으로 export하여 MCP에서도 재사용. 인라인 코드보다 일관성 확보.

## Scope boundary

다음 파일은 이 task에서 수정하지 않음:
- `crates/secall-core/src/search/vector.rs` — 벡터 검색 내부 로직 변경 없음
- `crates/secall-core/src/search/tokenizer.rs` — 토크나이저 변경 없음
- `crates/secall/src/commands/recall.rs` — CLI는 `search()` 호출만 하므로 자동 적용. 단, `max_per_session` CLI 옵션은 이 task 범위 밖 (향후 추가 가능)
- `crates/secall-core/src/search/chunker.rs` — 청킹 로직 변경 없음
