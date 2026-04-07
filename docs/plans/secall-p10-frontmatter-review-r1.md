# Review Report: seCall P10 — 세션 요약 frontmatter — Round 1

> Verdict: fail
> Reviewer: 
> Date: 2026-04-07 16:01
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall/src/commands/migrate.rs:134 — `status:` 라인이 없는 frontmatter fallback에서 `format!("{fm_str}{summary_line}")`로 이어붙여 마지막 기존 필드와 `summary:`가 같은 줄에 합쳐질 수 있습니다. 이 경우 YAML이 깨져 backfill 대상 파일의 frontmatter를 손상시킵니다.

## Recommendations

1. `docs/plans/secall-p10-frontmatter-result.md`에는 Task 01 검증 결과와 Task 02의 `cargo run -- migrate summary --dry-run` 실행 결과를 명시적으로 분리해 남기는 편이 좋습니다.
2. `insert_summary_into_frontmatter()`에 `status:` 없는 케이스 전용 테스트를 추가해 `...last_field\nsummary: ...` 형태를 보장하는 것이 안전합니다.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | 세션 summary frontmatter 추가 | ✅ done |
| 2 | 기존 세션 summary backfill | ✅ done |

