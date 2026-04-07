# Implementation Result: seCall P11 — 임베딩 성능 최적화

> Developer: claude
> Branch: N/A
> Date: 2026-04-07 18:39
> Plan Revision: 0

---

## Summary

모든 Verification 통과.

---

Verification results for Rework:
✅ `cargo check --all` — exit 0
✅ `cargo test --all` — 137 passed, 0 failed
✅ `cargo clippy --all-targets -- -D warnings` — 0 warnings

---

## 수정 내용

`search_with_embedding()` ANN stale 판단을 **2단계 방어**로 교체:

**1단계 (크기 기반)**: `ann.size() < db_count` → 인터럽트 후 새 벡터가 ANN에 없는 경우 → BLOB 스캔 폴백

**2단계 (rowid 기반, 신규)**: `ann.size() >= db_count`를 통과해도 ANN 검색 결과의 각 rowid를 DB에서 조회. `get_vector_meta()` 실패(DB에 없는 rowid)가 하나라도 나오면 re-embed/`--all` 후 DELETE된 row의 잔재로 판단 → BLOB 스캔 폴백

```
Before: ann.size() >= db_count 통과 → ANN 결과에서 stale rowid 조용히 버림 → 결과 누락
After:  stale rowid 1개 발견 즉시 BLOB 스캔으로 재계산 → 모든 커밋된 벡터 포함
```

ANN이 완전히 최신인 정상 케이스(신규 임베딩, 중단 없음)에서는 rowid 검사가 모두 통과 → `return Ok(results)` 로 기존 성능 유지.

## Subtask Results

### 1. 모든 Verification 통과.

---

Verification results for Rework:
✅ `cargo check --all` — exit 0
✅ `cargo test --all` — 137 passed, 0 failed
✅ `cargo clippy --all-targets -- -D warnings` — 0 warnings

---

## 수정 내용

`search_with_embedding()` ANN stale 판단을 **2단계 방어**로 교체:

**1단계 (크기 기반)**: `ann.size() < db_count` → 인터럽트 후 새 벡터가 ANN에 없는 경우 → BLOB 스캔 폴백

**2단계 (rowid 기반, 신규)**: `ann.size() >= db_count`를 통과해도 ANN 검색 결과의 각 rowid를 DB에서 조회. `get_vector_meta()` 실패(DB에 없는 rowid)가 하나라도 나오면 re-embed/`--all` 후 DEL

