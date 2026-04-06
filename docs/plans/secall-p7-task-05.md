---
type: task
plan: secall-p7
task_number: 5
status: draft
updated_at: 2026-04-07
depends_on: [4]
parallel_group: C
---

# Task 05: Wiki 프롬프트 효과 검증

## Changed files

- `docs/prompts/wiki-update.md` — 필요시 프롬프트 재조정
- `docs/prompts/wiki-incremental.md` — 필요시 프롬프트 재조정

신규 파일 없음. 기존 프롬프트 MD만 수정 가능.

## Change description

### 1단계: 현재 wiki 상태 확인

Task 04 완료 후 incremental wiki로 생성된 결과물 확인:

```bash
# wiki 페이지 목록
find ~/Documents/Obsidian\ Vault/seCall/wiki -name "*.md" -exec wc -l {} \; | sort -n

# 최근 수정된 wiki 페이지
find ~/Documents/Obsidian\ Vault/seCall/wiki -name "*.md" -newer "기준일" -exec basename {} \;
```

### 2단계: 상세도 비교

프롬프트 튜닝 전후를 비교 기준:

| 기준 | 튜닝 전 (요약 톤) | 튜닝 후 (정리 톤) |
|---|---|---|
| 코드 스니펫 포함 여부 | ? | ? |
| 에러 메시지 보존 여부 | ? | ? |
| 기술 결정 근거 기록 | ? | ? |
| 수치/측정값 보존 | ? | ? |
| 페이지 평균 길이 (줄) | ? | ? |

### 3단계: 프롬프트 재조정 (필요시)

확인 결과에 따라:
- 여전히 과도한 요약 → "세션의 80% 이상 내용을 보존" 같은 정량 기준 추가
- 너무 장황 → "페이지당 최대 300줄" 같은 상한 추가
- 코드 스니펫 누락 → "코드 블록은 반드시 포함" 명시
- incremental 프롬프트가 batch와 톤 불일치 → 동기화

### 4단계: 재생성 테스트

조정 후 특정 세션으로 incremental wiki 테스트:

```bash
cargo run -p secall -- wiki update --session <test-session-id> --dry-run
# 프롬프트 내용 확인 후
cargo run -p secall -- wiki update --session <test-session-id>
# 생성된 wiki 페이지 Obsidian에서 확인
```

## Dependencies

- **Task 04** — incremental wiki가 동작해야 단일 세션 테스트 가능
- claude CLI — wiki 생성에 필요

## Verification

```bash
# 1. wiki 페이지 수 확인
find ~/Documents/Obsidian\ Vault/seCall/wiki -name "*.md" | wc -l

# 2. 프롬프트 파일 무결성 확인 (frontmatter 유효)
head -5 docs/prompts/wiki-update.md
head -5 docs/prompts/wiki-incremental.md

# 3. Manual: Obsidian에서 wiki 페이지 열어 상세도 확인
#    - wiki/projects/tunadish.md에 코드 스니펫이 있는가?
#    - wiki/decisions/에 트레이드오프 근거가 기록되었는가?
#    - wiki/topics/에 구체적 수치가 보존되었는가?
```

## Risks

- 프롬프트 변경이 기존 wiki 페이지와 톤 불일치 → 새 페이지만 적용, 기존 페이지는 수동 재생성 필요
- claude CLI 호출 비용 → 테스트는 소수 세션으로만
- 주관적 판단 영역 → 사용자 피드백 필수

## Scope boundary

수정 금지 파일:
- `crates/` — 코드 변경 없음 (프롬프트 MD만)
- `crates/secall/src/commands/wiki.rs` — wiki 실행 로직 변경 불필요
- `crates/secall-core/src/mcp/` — MCP 서버 변경 불필요
