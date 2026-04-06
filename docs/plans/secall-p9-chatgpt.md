---
type: plan
status: draft
updated_at: 2026-04-07
version: 1
---

# seCall P9 — ChatGPT 파서

## Description

OpenAI ChatGPT export (conversations.json) 파서를 추가하여 에이전트 커버리지를 확대합니다.
ChatGPT "Export data" 기능으로 받는 ZIP에서 conversations.json을 추출하고 통합 Session 모델로 변환합니다.

## Expected Outcome

- `AgentKind::ChatGpt` 추가
- ChatGPT export ZIP/JSON 자동 감지 + 파싱 → vault MD 생성
- `secall ingest ~/Downloads/chatgpt-export.zip` 한 줄로 동작
- 기존 검색/위키/MCP에서 ChatGPT 세션도 통합 검색 가능

## Subtasks

| # | Task | 파일 | depends_on | parallel_group |
|---|---|---|---|---|
| 01 | ChatGPT export 포맷 분석 | (분석 문서만, 코드 없음) | - | A |
| 02 | ChatGPT 파서 구현 | chatgpt.rs, types.rs, detect.rs, mod.rs | 01 | B |
| 03 | 테스트 + E2E 검증 | chatgpt.rs (tests), detect.rs (tests) | 02 | C |

## ChatGPT Export 포맷 (알려진 구조)

ChatGPT "Settings → Data controls → Export data"로 받는 ZIP:

```
chatgpt-export.zip
├── conversations.json    ← 핵심
├── chat.html             ← HTML 렌더링 (무시)
├── message_feedback.json ← 피드백 (무시)
├── model_comparisons.json ← (무시)
├── shared_conversations.json ← (무시)
└── user.json             ← 사용자 정보 (무시)
```

### conversations.json 구조

```json
[
  {
    "title": "프로젝트 설계 논의",
    "create_time": 1711234567.123,
    "update_time": 1711234999.456,
    "mapping": {
      "msg-id-1": {
        "id": "msg-id-1",
        "message": {
          "id": "msg-id-1",
          "author": { "role": "system" },
          "content": { "content_type": "text", "parts": ["You are ChatGPT..."] },
          "create_time": 1711234567.123,
          "metadata": { "model_slug": "gpt-4" }
        },
        "parent": null,
        "children": ["msg-id-2"]
      },
      "msg-id-2": {
        "id": "msg-id-2",
        "message": {
          "author": { "role": "user" },
          "content": { "content_type": "text", "parts": ["설계 도와줘"] },
          "create_time": 1711234570.0
        },
        "parent": "msg-id-1",
        "children": ["msg-id-3"]
      }
    },
    "conversation_id": "conv-uuid-123",
    "default_model_slug": "gpt-4",
    "current_node": "msg-id-last"
  }
]
```

### 핵심 차이점 (vs claude.ai)

| 항목 | claude.ai | ChatGPT |
|---|---|---|
| 메시지 구조 | 선형 배열 `chat_messages[]` | 트리 `mapping{}` (parent/children) |
| 타임스탬프 | ISO 8601 문자열 | Unix epoch float |
| 역할 | `sender: "human"/"assistant"` | `author.role: "user"/"assistant"/"system"/"tool"` |
| 컨텐츠 | `content: [ContentBlock]` | `content.parts: [string|object]` |
| 모델 정보 | 없음 | `metadata.model_slug` |
| ID | `uuid` (UUID) | `conversation_id` (UUID) |

## Constraints

- ChatGPT export를 실제 보유하고 있어야 E2E 테스트 가능
- message tree → 선형화 필요 (parent/children 따라 DFS)
- 트리에 분기(regeneration)가 있을 수 있음 → current_node 경로만 추출

## Non-goals

- ChatGPT API 연동 (export 파일만)
- GPT 플러그인/Code Interpreter/DALL-E 결과물 별도 처리 (텍스트만 추출)
- Canvas/Artifacts 구조 파싱
- shared_conversations.json 파싱
