# Review Report: P18 — 세션 분류 시스템 (config 기반) — Round 1

> Verdict: fail
> Reviewer: 
> Date: 2026-04-11 18:41
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall-core/src/vault/config.rs:86 — Task 02 계약은 `pattern`, `session_type`, `default`, `skip_embed_types` 기반 규칙을 요구하지만, 구현은 `label`, `cwd_pattern`, `project_pattern`, `agent_pattern`, `default_type`만 정의합니다. 따라서 계획서에 적힌 `.secall.toml` 예시가 역직렬화되지 않고, 임베딩 제외 설정 자체를 할 수 없습니다.
2. crates/secall/src/commands/ingest.rs:330 — Task 03 계약은 첫 번째 user turn 내용으로 세션을 분류해야 하는데, 구현은 `cwd/project/agent` 정규식으로 분류합니다. 자동화 세션 판별 기준이 달라져 계획된 분류 규칙이 그대로 동작하지 않습니다.
3. crates/secall/src/commands/ingest.rs:459 — Task 03 계약의 `skip_embed_types` 검사 없이 모든 세션을 `vector_tasks`에 넣고 있습니다. 그 결과 `automated`로 분류된 세션도 계속 임베딩되어 Expected Outcome의 “automated 세션 임베딩 skip 가능”을 만족하지 못합니다.
4. crates/secall-core/src/search/vector.rs:311 — Task 04에서 `exclude_session_types`를 검색 필터로 추가했지만, vector 검색의 `passes_filters()`는 project/agent/date만 검사합니다. 따라서 `crates/secall/src/commands/recall.rs:51`에서 automated 제외 필터를 넣어도 `--vec` 경로와 hybrid의 vector 결과에는 automated 세션이 계속 섞일 수 있습니다.
5. crates/secall/src/main.rs:121 — Task 05와 계획 Expected Outcome은 `secall classify --backfill`를 요구하지만, 구현된 CLI는 `Classify { dry_run }`뿐이라 `--backfill` 없이 바로 DB를 갱신합니다. 사용자가 dry-run 없이 실행하면 즉시 반영되는 현재 동작은 계획된 안전장치와 다릅니다.
6. crates/secall/src/commands/classify.rs:25 — backfill도 Task 05 계약과 달리 첫 번째 user turn 내용(`first_content`)을 사용하지 않고 `cwd/project/agent` 기준으로 분류합니다. 기존 세션 재분류 결과가 ingest 시점 분류 규칙과도 일치하지 않습니다.

## Recommendations

1. `ClassificationConfig`와 ingest/backfill 로직을 task 문서대로 다시 맞추고, `skip_embed_types`를 ingest 및 vector search 양쪽에 일관되게 적용하세요.
2. `classify` CLI는 `--backfill` 플래그를 명시적으로 요구하도록 바꾸고, `docs/plans/p18-config-result.md`에 task별 Verification 결과를 남기세요.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | DB 스키마 v4 | ✅ done |
| 2 | Config | ✅ done |
| 3 | Ingest | ✅ done |
| 4 | Search | ✅ done |
| 5 | Backfill 명령 | ✅ done |

