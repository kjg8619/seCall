---
type: task
plan: secall-p7
task_number: 3
status: draft
updated_at: 2026-04-07
depends_on: []
parallel_group: A
---

# Task 03: MCP wiki-search 도구

## Changed files

- `crates/secall-core/src/mcp/tools.rs` — `WikiSearchParams` 구조체 추가
- `crates/secall-core/src/mcp/server.rs` — `wiki_search()` 도구 메서드 추가 (line 230 이후)
- `crates/secall-core/src/mcp/server.rs:31-35` — `SeCallMcpServer`에 `vault_path: PathBuf` 필드 추가
- `crates/secall-core/src/mcp/instructions.rs` — 도구 설명에 wiki_search 추가

## Change description

### 1단계: WikiSearchParams 정의

`tools.rs`에 추가:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WikiSearchParams {
    /// Search query for wiki pages (matched against filename and content)
    pub query: String,
    /// Filter by wiki category: projects, topics, decisions (optional)
    pub category: Option<String>,
    /// Max results (default 5)
    pub limit: Option<usize>,
}
```

### 2단계: SeCallMcpServer에 vault_path 추가

서버 구조체에 `vault_path: PathBuf` 필드 추가. `new()` 시그니처 변경:

```rust
pub fn new(db: Arc<Mutex<Database>>, search: Arc<SearchEngine>, vault_path: PathBuf) -> Self
```

MCP 서버 생성 코드에서 vault_path 전달 (server.rs 하단의 `start_mcp_server` / `start_mcp_http_server`).

### 3단계: wiki_search 도구 구현

`server.rs`에 새 `#[tool]` 메서드 추가:

```rust
#[tool(description = "Search wiki knowledge pages. Returns matching wiki articles from projects, topics, and decisions.")]
async fn wiki_search(&self, #[tool(params)] params: WikiSearchParams) -> Result<CallToolResult, McpError> {
    let wiki_dir = self.vault_path.join("wiki");
    let limit = params.limit.unwrap_or(5);
    
    // 1. wiki/ 하위 MD 파일 수집 (category 필터 적용)
    // 2. 파일명 + 내용에서 query 매칭 (case-insensitive contains)
    // 3. 매칭된 파일의 frontmatter + 첫 N줄 반환
    // 4. limit 적용
}
```

검색 로직:
- `wiki/{category}/` 또는 `wiki/` 전체를 walkdir로 순회
- 파일명 매칭 (query가 파일명에 포함) → 높은 우선순위
- 내용 매칭 (query가 파일 본문에 포함) → 낮은 우선순위
- 결과: `{path, title (frontmatter), preview (첫 500자)}` 형태

### 4단계: instructions 업데이트

`instructions.rs`의 `build_instructions()`에 wiki_search 도구 설명 추가.

### 5단계: wiki.rs allowed tools 업데이트

`crates/secall/src/commands/wiki.rs:52`의 `--allowedTools`에 `mcp__secall__wiki_search` 추가. wiki 에이전트가 기존 wiki 페이지를 검색할 수 있도록.

## Dependencies

- 없음 (독립 task)
- vault_path는 Config에서 이미 사용 가능

## Verification

```bash
# 1. 빌드 확인
cargo build -p secall

# 2. 테스트 통과
cargo test --all

# 3. MCP 도구 목록에 wiki_search 포함 확인
# MCP inspector 또는 직접 JSON-RPC 호출:
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cargo run -p secall -- mcp 2>/dev/null | head -50

# 4. wiki_search 호출 테스트
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"wiki_search","arguments":{"query":"tunadish"}}}' | cargo run -p secall -- mcp 2>/dev/null
# 예상: tunadish.md 관련 결과 반환
```

## Risks

- `SeCallMcpServer::new()` 시그니처 변경 → 서버 생성 코드 (start_mcp_server, start_mcp_http_server) 수정 필요
- wiki/ 디렉토리가 없을 때 → 빈 결과 반환 (에러 아님)
- 대용량 wiki 파일 읽기 → 미리보기 500자로 제한
- vault_path를 서버에 넘기는 방식 → Config::load_or_default() 호출 위치 확인 필요

## Scope boundary

수정 금지 파일:
- `crates/secall-core/src/search/` — 기존 검색 로직 변경 불필요 (wiki-search는 단순 파일 검색)
- `crates/secall-core/src/store/` — DB 스키마 변경 불필요
- `crates/secall/src/commands/ingest.rs` — Task 01 범위
- `crates/secall/src/commands/embed.rs` — Task 02 범위
- `crates/secall/src/commands/sync.rs` — Task 04 범위
