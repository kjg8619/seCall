---
type: task
plan: secall-p9-chatgpt
task_number: 1
status: draft
updated_at: 2026-04-07
depends_on: []
parallel_group: A
---

# Task 01: ChatGPT export 포맷 분석

## Changed files

- 코드 변경 없음
- 분석 결과를 `docs/plans/secall-p9-chatgpt.md`의 포맷 섹션에 업데이트

## Change description

### 1단계: export 데이터 확보

사용자에게 ChatGPT export ZIP 요청. "Settings → Data controls → Export data"로 다운로드. 이메일로 수신한 ZIP을 프로젝트 내 `desktop_conversation/chatgpt/`에 배치.

### 2단계: conversations.json 구조 분석

```bash
# ZIP 내용 확인
unzip -l chatgpt-export.zip

# conversations.json 추출
unzip -p chatgpt-export.zip conversations.json | python3 -m json.tool | head -100

# 전체 대화 수 확인
unzip -p chatgpt-export.zip conversations.json | python3 -c "
import json, sys
data = json.load(sys.stdin)
print(f'Total conversations: {len(data)}')
for c in data[:3]:
    print(f'  {c.get(\"title\", \"untitled\")} — {len(c.get(\"mapping\", {}))} nodes')
"
```

### 3단계: 필드 매핑 정리

실제 데이터에서 확인할 항목:

1. **mapping 트리 구조**
   - `parent`/`children`으로 트리 구성 확인
   - 분기(regeneration)가 있는 대화 식별
   - `current_node`가 가리키는 경로가 정상 선형화 가능한지 확인

2. **메시지 역할**
   - `author.role` 값 목록: `system`, `user`, `assistant`, `tool` 외 다른 값?
   - `tool` 역할의 `content.parts` 구조 (Code Interpreter 결과 등)

3. **컨텐츠 타입**
   - `content.content_type` 값 목록: `text`, `code`, `tether_browsing_display`, `tether_quote`, `multimodal_text`, `execution_output`?
   - `parts[]` 내 object 타입 (이미지 참조 등) → 텍스트만 추출

4. **타임스탬프 형식**
   - `create_time`: Unix epoch float (예: `1711234567.123`) 확인
   - null 가능성 확인

5. **모델 정보**
   - `default_model_slug`: `gpt-4`, `gpt-4o`, `o1-preview` 등
   - 메시지별 `metadata.model_slug` vs 대화별 `default_model_slug` 차이

6. **토큰 정보**
   - `metadata.finish_details` 존재 여부
   - 토큰 카운트 필드 존재 여부 (대부분 없음 예상)

### 4단계: claude.ai 파서와 구조 비교

| 항목 | claude.ai 파서 구현 | ChatGPT 대응 |
|---|---|---|
| `Conversation.uuid` → session_id | `conversation_id` |
| `Conversation.name` → project | `title` |
| `Conversation.created_at` (ISO) | `create_time` (epoch float) |
| `ChatMessage[]` (선형) | `mapping{}` (트리 → 선형화) |
| `ChatMessage.sender` | `message.author.role` |
| `ContentBlock` enum | `content.content_type` + `parts[]` |
| 모델 없음 | `default_model_slug` |

### 5단계: 선형화 알고리즘 설계

ChatGPT의 mapping은 트리 구조 (regeneration 분기 가능):

```
system → user → assistant (v1)
                 ↘ assistant (v2)  ← regeneration
```

선형화 전략:
1. `current_node`에서 시작
2. `parent`를 따라 root까지 역순 추적
3. 역순을 뒤집어 선형 배열 생성
4. `system` 역할 메시지는 skip (seCall에서 불필요)
5. `message`가 null인 노드는 skip (삭제된 메시지)

이 방식으로 사용자가 마지막으로 본 대화 경로만 추출.

## Dependencies

- ChatGPT export ZIP 파일 (사용자 제공)

## Verification

```bash
# 1. export ZIP 내 conversations.json 존재 확인
unzip -l desktop_conversation/chatgpt/*.zip | grep conversations.json

# 2. JSON 파싱 가능 확인
unzip -p desktop_conversation/chatgpt/*.zip conversations.json | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'{len(d)} conversations')"

# 3. 분석 결과가 plan 문서에 반영되었는지 확인
grep "mapping" docs/plans/secall-p9-chatgpt.md
```

## Risks

- ChatGPT export 포맷이 시간에 따라 변경될 수 있음 → 실제 데이터 기반 분석 필수
- 대용량 export (수천 대화) → 분석 시 jq/python 활용
- 트리 구조가 예상보다 복잡할 수 있음 (multi-branch, orphan nodes)

## Scope boundary

수정 금지 파일:
- `crates/` — 코드 변경 없음 (분석만)
- `docs/plans/secall-p7*.md` — P7 범위
- `docs/plans/secall-p8*.md` — P8 범위
