---
type: plan
status: draft
updated_at: 2026-04-07
version: 1
---

# seCall P7 — 기능 고도화 + 검색 품질

## Description

빈 세션 필터링, 임베딩 갭 메우기, MCP wiki-search 도구, incremental wiki, wiki 프롬프트 효과 검증을 하나의 플랜으로 묶어 일상 사용 품질을 개선합니다.

## Expected Outcome

- `--min-turns` 옵션으로 2턴 이하 쓰레기 세션 자동 스킵
- 미임베딩 세션 100% 커버 → 벡터 검색 정확도 향상
- MCP `wiki_search` 도구 → 에이전트가 위키 지식도 활용
- `secall sync`에 incremental wiki 자동 생성 연동
- wiki 프롬프트 튜닝 효과 확인 및 재조정

## Subtasks

| # | Task | 파일 | depends_on | parallel_group |
|---|---|---|---|---|
| 01 | 빈 세션 자동 스킵 | ingest.rs, main.rs | - | A |
| 02 | 임베딩 갭 메우기 | embed.rs, vector.rs, db.rs | - | A |
| 03 | MCP wiki-search 도구 | server.rs, tools.rs | - | A |
| 04 | Incremental wiki in sync | sync.rs, wiki.rs | 01 | B |
| 05 | Wiki 프롬프트 효과 검증 | docs/prompts/*.md | 04 | C |

- Task 01, 02, 03은 독립적 → parallel_group A (병렬 가능)
- Task 04는 01에 의존 (min-turns 필터가 있어야 sync에서 쓸모없는 wiki 생성 방지)
- Task 05는 04 완료 후 실행 (incremental wiki로 생성된 결과 검증)

## Constraints

- embed 구현 시 ONNX 모델 미다운로드 상태면 graceful error + 안내 메시지
- wiki-search는 read-only, vault 파일 수정 금지
- incremental wiki는 claude CLI 의존 — 없으면 skip + 경고

## Non-goals

- wiki 전체 재생성 (기존 위키 유지, incremental만)
- 벡터 인덱스 교체 (usearch HNSW 유지)
- MCP HTTP 모드 인증 추가
- 새 에이전트 파서 추가 (P9 범위)
