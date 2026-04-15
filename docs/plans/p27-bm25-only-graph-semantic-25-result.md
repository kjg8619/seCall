# Implementation Result: P27 — BM25-only 선택 시 graph semantic 자동 비활성화 (#25)

> Developer: claude
> Branch: N/A
> Date: 2026-04-15 13:51
> Plan Revision: 0

---

## Summary

Verification results for Task 02 (Rework):
- ✅ `cargo check -p secall` — Finished, exit 0
- ✅ `cargo test` — 254 passed, 0 failed, 10 ignored

---

**수정 내용**: `ingest.rs:365` — `semantic_enabled` 조건에 `config.embedding.backend != "none"` 체크를 추가했습니다.

이제 사용자가 수동으로 설정 파일을 편집해 `embedding.backend = "none"`인데 `graph.semantic = true`로 남긴 불일치 상태에서도 semantic extraction이 시작되지 않습니다.

## Subtask Results

### 1. Verification results for Task 02 (Rework):
- ✅ `cargo check -p secall` — Finished, exit 0
- ✅ `cargo test` — 254 passed, 0 failed, 10 ignored

---

**수정 내용**: `ingest.rs:365` — `semantic_enabled` 조건에 `config.embedding.backend != "none"` 체크를 추가했습니다.

이제 사용자가 수동으로 설정 파일을 편집해 `embedding.backend = "none"`인데 `graph.semantic = true`로 남긴 불일치 상태에서도 semantic extraction이 시작되지 않습니다.

