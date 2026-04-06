---
type: plan
status: draft
updated_at: 2026-04-07
version: 1
---

# seCall P8 — 안정화 + 배포

## Description

에러 리포팅 개선과 GitHub Releases 바이너리 배포를 묶어 운영 안정성과 설치 편의를 높입니다.

## Expected Outcome

- ingest 실패 시 파일별/세션별 상세 에러 요약 출력
- `--format json`으로 구조화된 에러 리포트 출력
- GitHub tag push 시 macOS aarch64/x86_64 바이너리 자동 릴리스

## Subtasks

| # | Task | 파일 | depends_on | parallel_group |
|---|---|---|---|---|
| 01 | 에러 리포팅 개선 | error.rs, ingest.rs, output.rs | - | A |
| 02 | Release 바이너리 배포 | Cargo.toml, release.yml | - | A |

- 두 task는 독립적 → parallel_group A (병렬 가능)

## Constraints

- Linux 빌드는 lindera ko-dic embed 크기 문제로 스코프 밖 (macOS만 우선)
- 에러 리포팅은 기존 exit code 유지 (0: 성공, 1: 일부 실패)
- CI 기존 `ci.yml` 변경 금지 (별도 release.yml)

## Non-goals

- Windows 빌드
- Docker 이미지
- Homebrew formula
- 자동 업데이트 메커니즘
