---
type: plan
status: in_progress
slug: p18-config
updated_at: 2026-04-11
---

# Plan: P18 — 세션 분류 시스템 (config 기반)

## Description

ingest된 모든 세션이 동등하게 처리되어 자동화 세션(크론, wiki auto-update 등)이 검색 결과에 섞이는 문제를 해결한다. `secall.toml`에 사용자 정의 regex 규칙을 추가하면 첫 번째 user turn 내용을 기준으로 세션에 `session_type`을 자동 태깅하고, 검색·임베딩에서 필터링 가능하게 한다.

## Expected Outcome

- `secall recall` 이 기본적으로 `automated` 세션을 제외하고 검색
- `secall ingest` 가 분류 규칙에 따라 `automated` 세션의 임베딩 skip 가능
- `secall classify --backfill` 로 기존 세션을 재분류
- 규칙은 `.secall.toml` 에 사용자가 직접 정의, 하드코딩 없음

## Subtasks

| # | 제목 | 파일 | 의존성 |
|---|------|------|--------|
| 01 | DB 스키마 v4 — `session_type` 컬럼 | schema.rs, db.rs | 없음 |
| 02 | Config — `ClassificationConfig` 추가 | config.rs | 없음 |
| 03 | Ingest — 분류 적용 + 임베딩 skip | types.rs, ingest.rs, bm25.rs | Task 01, 02 |
| 04 | Search — `SearchFilters`에 `session_type` 필터 | bm25.rs, search_repo.rs, server.rs, recall.rs | Task 01 |
| 05 | Backfill — `secall classify --backfill` | classify.rs (신규), main.rs | Task 01, 02 |

## Constraints

- `regex` crate: workspace에 `tokenizers`가 `fancy-regex`를 이미 사용 중이므로 별도로 `regex = "1"` 추가 필요
- DB 스키마 v3 → v4 마이그레이션 (ALTER TABLE, 기존 데이터 유지)
- `INSERT OR IGNORE` 패턴 유지 — 기존 세션 중복 ingest 시 session_type 덮어쓰지 않음

## Non-goals

- ML 기반 세션 분류
- turn 단위 분류 (세션 단위로 충분)
- UI/대시보드
- wiki 파이프라인 직접 수정 (SearchFilters 필터링으로 간접 해결)
