# Review Report: seCall P7 — 기능 고도화 + 검색 품질 — Round 1

> Verdict: fail
> Reviewer: 
> Date: 2026-04-07 07:16
> Plan Revision: 0

---

## Verdict

**fail**

## Findings

1. crates/secall-core/src/mcp/server.rs:255 — `params.category`를 검증 없이 `wiki_dir.join(cat)`에 사용해 `../../...` 또는 절대경로 입력으로 `wiki/` 바깥의 임의 디렉터리를 순회할 수 있습니다. `wiki_search`가 위키 페이지만 읽어야 한다는 task 범위를 깨고, MCP를 통해 의도치 않은 파일 노출이 가능합니다.

## Recommendations

1. `category`는 `projects|topics|decisions` enum으로 제한하거나, canonicalize 후 최종 경로가 `wiki_dir` 하위인지 검사하세요.
2. /Users/d9ng/privateProject/seCall/crates/secall/src/commands/embed.rs:32 의 `batch_size`는 현재 저장만 되고 실제 처리에 반영되지 않으므로, 배치 처리에 연결하거나 옵션을 제거해 의미를 명확히 하는 편이 좋습니다.

## Subtask Verification

| # | Subtask | Status |
|---|---------|--------|
| 1 | 빈 세션 자동 스킵 | ✅ done |
| 2 | 임베딩 갭 메우기 | ✅ done |
| 3 | MCP wiki-search 도구 | ✅ done |
| 4 | Incremental wiki in sync | ✅ done |
| 5 | Wiki 프롬프트 효과 검증 | ✅ done |

