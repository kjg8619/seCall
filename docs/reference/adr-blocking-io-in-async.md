---
type: reference
status: done
updated_at: 2026-04-15
---

# ADR: Async 함수 내 Blocking I/O 처리 — spawn_blocking 래핑 (A안)

## 배경

seCall 프로젝트 전반에서 async 함수 안에 `std::process::Command`와 `std::fs` 동기 호출이 직접 사용되고 있다.
Tokio async 런타임에서 blocking I/O는 런타임 스레드를 점유해 다른 task의 진행을 방해할 수 있다.

PR #29 (Codex wiki 백엔드) 머지 시점에 이 문제가 식별되었다.

## 선택지

### A안: `tokio::task::spawn_blocking` 래핑

- 기존 동기 코드를 `spawn_blocking(|| { ... })` 으로 감싸서 blocking 스레드풀에서 실행
- 동기 트레이트 시그니처 변경 불필요
- 변경량: 각 호출 지점에 `spawn_blocking` + `.await` 추가

### B안: 정석 async 전환

- `std::fs` → `tokio::fs`, `std::process::Command` → `tokio::process::Command`
- 트레이트 시그니처를 `async fn`으로 변경 → `async_trait` 매크로 필요
- 변경량: 5개 파일 단순 치환 + 트레이트 3개 async 전환 + 호출부 전체 수정

## 결정: A안 채택

## 근거

1. **오버헤드가 무시 가능**: `spawn_blocking` 호출당 ~1-2μs (스레드풀 디스패치). 실제 I/O 작업은 수~수백ms 단위이므로 비율 0.01% 미만.
2. **seCall은 CLI 도구**: 서버처럼 초당 수천 요청을 처리하지 않는다. 동시 I/O 호출이 많아야 수 건이므로 스레드풀 포화 우려 없음.
3. **변경 범위 최소화**: 동기 트레이트 시그니처를 유지하므로, 기존 `WikiBackend` trait 등의 구현체 전체를 async로 전환할 필요가 없다. B안 대비 변경 파일 수와 복잡도가 현저히 낮다.
4. **B안의 장점이 미미**: 서버 환경이었다면 B안이 정석이지만, CLI에서 얻는 실질적 성능 이점이 없다. 순수 async로 전환하면 코드 복잡도만 증가한다.

## 적용 범위

- `std::process::Command` 호출: `wiki.rs`, `codex.rs` 등 외부 프로세스 실행부
- `std::fs` 호출: `graph.rs`, `ingest.rs`, `sync.rs`, `log.rs` 등 파일 I/O부

## 예외

이미 동기 컨텍스트에서 실행되는 코드(예: 테스트, 순수 동기 함수)는 래핑 불필요.
