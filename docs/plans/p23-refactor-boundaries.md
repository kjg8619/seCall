---
type: plan
status: draft
updated_at: 2026-04-14
slug: p23-refactor-boundaries
---

# P23 — 모듈 경계 고정 리팩터링 (1차)

## Background

seCall은 검색 기능이 동작하고 테스트도 268개 통과 상태이지만,
책임 경계가 불명확하여 기능 추가 시 파일 위치 결정이 어렵다.

이번 리팩터링의 목표는 **"구조를 바꿔도 기능은 그대로"**이다.

### 현재 문제

| 파일 | 줄 수 | 문제 |
|------|-------|------|
| `store/db.rs` | 1155 | trait 밖 메서드 40+개가 session/통계/캐시/classify 뒤섞임 |
| `search/bm25.rs` | 631 | `impl SessionRepo for Database` + `impl SearchRepo for Database`가 검색 모듈 안에 있음 |
| `search/vector.rs` | 753 | `impl VectorRepo for Database`가 검색 모듈 안에 있음 |
| `commands/ingest.rs` | 808 | 도메인 로직과 CLI 로직 혼재 |
| `commands/wiki.rs` | 808 | 도메인 로직과 CLI 로직 혼재 |

### 이미 존재하는 기반

- `store/session_repo.rs` — trait 정의 (16줄, impl 없음)
- `store/search_repo.rs` — trait 정의 (12줄, impl 없음)
- `store/vector_repo.rs` — trait 정의 (22줄, impl 없음)
- `store/graph_repo.rs` — trait + impl 완성 (342줄) ← 목표 패턴

## 원칙

1. 공개 API(CLI 명령) 동작을 바꾸지 않는다
2. 검색 알고리즘(BM25 공식, ANN, RRF)은 건드리지 않는다
3. 테스트 통과 상태를 매 단계 유지한다
4. 파일 분리만 하지 말고 책임도 같이 옮긴다
5. `graph_repo.rs` 패턴(trait + impl 한 파일)을 따른다

## 이번 라운드에서 하지 않는 것

- BM25 공식 변경
- tokenizer 정책 변경
- ANN 교체 / sqlite-vec 재도입 (현재 macOS arm64 C 컴파일 이슈로 보류 중)
- query expansion 전략 변경
- CLI UX 변경
- 스키마 전면 수정
- ingest/wiki 서비스 계층 도입 (2차 라운드에서 판단)
- vector.rs 4개 파일 분해 (repo 이동 후 재평가)

## Subtasks

| # | 제목 | 파일 | 의존성 |
|---|------|------|--------|
| 01 | bm25.rs에서 repo impl을 store/로 이동 | bm25.rs, session_repo.rs, search_repo.rs | 없음 |
| 02 | vector.rs에서 repo impl을 store/로 이동 | vector.rs, vector_repo.rs | 없음 |
| 03 | db.rs 메서드를 카테고리별 repo trait에 편입 | db.rs, session_repo.rs, search_repo.rs, store/mod.rs | 01, 02 |

Task 01, 02는 독립적이므로 병렬 진행 가능.
Task 03은 01, 02 완료 후 진행.

## 성공 기준

1. `cargo test` 268개 전부 통과 (0 regression)
2. `db.rs`가 현저히 줄어듦 (목표: 연결/마이그레이션/트랜잭션만 잔류, ~200줄 이하)
3. `bm25.rs`에 SQL 없음, `vector.rs`에 SQL 없음
4. search 모듈은 trait만 의존, SQL은 store에만 존재
5. 새 기능 추가 시 파일 위치가 바로 결정됨

## 테스트 기준선 (2026-04-14)

```
secall:      16 passed, 0 failed
secall-core: 248 passed, 0 failed, 10 ignored
doc-tests:   4 passed
총:          268 passed
```
