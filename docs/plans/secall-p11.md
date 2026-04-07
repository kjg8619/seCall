---
type: plan
status: draft
updated_at: 2026-04-07
version: 1
---

# seCall P11 — 임베딩 성능 최적화

## Description

M1 Max 64GB에서 1242 세션/3.1GB 임베딩에 49시간이 소요되는 심각한 성능 문제를 해결한다.
핵심 병목 5개를 단계적으로 제거하여 **2-4시간 수준**으로 단축한다 (10-20x 개선 목표).

기존 검색/인덱싱 동작에 사이드이펙트 없이, **중단 후 재시작 시 중복/누락이 발생하지 않아야 한다**.

## Root Cause Analysis

| # | 병목 | 위치 | 영향도 |
|---|------|------|--------|
| 1 | 세션 직렬 처리 | `embed.rs:35-63` — for loop 1개씩 | M1 Max 8 P-core 중 1개만 사용 |
| 2 | ONNX 가짜 batch | `embedding.rs:239-242` — embed_batch 내부 for loop | 텍스트별 개별 inference |
| 3 | Mutex 직렬화 | `embedding.rs:112` — `Arc<Mutex<Session>>` | 동시 inference 불가 |
| 4 | 청크별 개별 INSERT | `vector.rs:64-70` — INSERT per chunk | ~124K 개별 트랜잭션 |
| 5 | 세션마다 ANN 디스크 저장 | `vector.rs:94-101` — save per session | 1242번 I/O |

## 중단-재시작 안전성 설계

**현재 문제**: `find_sessions_without_vectors()` (db.rs:258-274)는 `turn_vectors`에 1행이라도 있으면 세션 완료로 간주 → 부분 임베딩 세션이 스킵됨.

**해결**: 세션 단위 트랜잭션 (BEGIN → DELETE existing → INSERT all → COMMIT). 실패/중단 시 자동 롤백 → 다음 실행에서 재처리.

## Expected Outcome

- 동일 1242 세션 임베딩이 **2-4시간** 이내 완료
- `secall embed` 중단 후 재시작 시 중복/누락 없음
- 기존 `recall`, `get`, `status`, `mcp` 동작 변경 없음

## Subtasks

| # | Title | depends_on | parallel_group |
|---|-------|------------|----------------|
| 01 | DB 트랜잭션 + 중단-재시작 안전성 | — | A |
| 02 | ORT 진짜 batch inference | — | A |
| 03 | 세션 병렬 처리 + ANN 저장 빈도 감소 | 01, 02 | — |
| 04 | batch_size CLI 연결 + 진행률 표시 개선 | 03 | — |

## Constraints

- 기존 `recall`, `get`, `status`, `mcp` 명령의 동작 변경 금지
- Ollama/OpenAI embedder의 기존 동작 보존 (ORT만 batch 개선)
- DB 스키마 변경 최소화 (turn_vectors 테이블 구조 유지)
- `secall embed --all` 재실행 시 중복 벡터 생성 금지
- 검색 품질(recall@10 등) 변경 없음

## Non-goals

- Embedder 백엔드 자동 전환 (Ollama → ORT 등)
- GPU 가속 (CoreML, Metal)
- 벡터 차원 축소 (384 → 128 등)
- 청크 전략 변경 (3600자/540 overlap 유지)
- 새 embedder 백엔드 추가
