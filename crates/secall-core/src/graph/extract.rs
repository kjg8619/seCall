use std::sync::OnceLock;

use regex::Regex;

use crate::ingest::markdown::SessionFrontmatter;

// ─── 컴파일된 정규식 (OnceLock) ───────────────────────────────────────────────

fn re_issue_ref() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)(?:fixes|closes|resolves|issue|re:)?\s*#(\d+)").unwrap())
}

fn re_file_edit() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r">\s*\[!tool\]-\s*Edit\s+`([^`]+)`").unwrap())
}

fn re_tool_path() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r">\s*\[!tool\]-\s*(?:Edit|Write|Bash)[^\n]*\n>\s*```\n>\s*([^\n`]+)").unwrap()
    })
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub meta: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: String,
    pub weight: f64,
}

#[derive(Debug, Default)]
pub struct ExtractionResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// 단일 세션 frontmatter에서 노드/엣지 추출
pub fn extract_from_frontmatter(fm: &SessionFrontmatter) -> ExtractionResult {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let session_id_str = &fm.session_id;
    let session_node_id = format!("session:{}", session_id_str);

    // 세션 메타 JSON
    let meta = serde_json::json!({
        "date": fm.date,
        "turns": fm.turns,
        "summary": fm.summary,
    });

    // 1. Session 노드
    nodes.push(GraphNode {
        id: session_node_id.clone(),
        node_type: "session".to_string(),
        label: session_id_str[..session_id_str.len().min(8)].to_string(),
        meta: Some(meta.to_string()),
    });

    // 2. Project 노드 + 엣지
    if let Some(project) = &fm.project {
        if !project.is_empty() {
            let project_id = format!("project:{}", project);
            nodes.push(GraphNode {
                id: project_id.clone(),
                node_type: "project".to_string(),
                label: project.clone(),
                meta: None,
            });
            edges.push(GraphEdge {
                source: session_node_id.clone(),
                target: project_id,
                relation: "belongs_to".to_string(),
                confidence: "EXTRACTED".to_string(),
                weight: 1.0,
            });
        }
    }

    // 3. Agent 노드 + 엣지
    if !fm.agent.is_empty() {
        let agent_id = format!("agent:{}", fm.agent);
        nodes.push(GraphNode {
            id: agent_id.clone(),
            node_type: "agent".to_string(),
            label: fm.agent.clone(),
            meta: None,
        });
        edges.push(GraphEdge {
            source: session_node_id.clone(),
            target: agent_id,
            relation: "by_agent".to_string(),
            confidence: "EXTRACTED".to_string(),
            weight: 1.0,
        });
    }

    // 4. Tool 노드 + 엣지
    if let Some(tools) = &fm.tools_used {
        for tool in tools {
            if tool.is_empty() {
                continue;
            }
            let tool_id = format!("tool:{}", tool);
            nodes.push(GraphNode {
                id: tool_id.clone(),
                node_type: "tool".to_string(),
                label: tool.clone(),
                meta: None,
            });
            edges.push(GraphEdge {
                source: session_node_id.clone(),
                target: tool_id,
                relation: "uses_tool".to_string(),
                confidence: "EXTRACTED".to_string(),
                weight: 1.0,
            });
        }
    }

    ExtractionResult { nodes, edges }
}

/// Rule-based 시맨틱 엣지 추출.
///
/// - `fixes_bug`: summary + body에서 `#N`, `fixes #N`, `closes #N` 등 이슈 참조 → `session → issue:N`
/// - `modifies_file`: body의 FileEdit/ToolUse 렌더링에서 파일 경로 추출 → `session → file:path`
///   단, frontmatter의 tools_used에 Edit 또는 Write가 없으면 modifies_file은 건너뜀.
pub fn extract_semantic_edges(fm: &SessionFrontmatter, body: &str) -> Vec<GraphEdge> {
    let mut edges = Vec::new();
    let session_node_id = format!("session:{}", fm.session_id);

    // ── fixes_bug ────────────────────────────────────────────────────────────
    // summary + body 전체를 합쳐서 이슈 번호 스캔
    let search_text = format!("{} {}", fm.summary.as_deref().unwrap_or(""), body);
    let mut seen_issues = std::collections::HashSet::new();
    for cap in re_issue_ref().captures_iter(&search_text) {
        let issue_num = &cap[1];
        if seen_issues.insert(issue_num.to_string()) {
            edges.push(GraphEdge {
                source: session_node_id.clone(),
                target: format!("issue:{}", issue_num),
                relation: "fixes_bug".to_string(),
                confidence: "INFERRED".to_string(),
                weight: 0.7,
            });
        }
    }

    // ── modifies_file ────────────────────────────────────────────────────────
    // tools_used에 Edit 또는 Write가 있을 때만 파일 경로 스캔
    let has_edit_write = fm
        .tools_used
        .as_ref()
        .map(|tools| {
            tools
                .iter()
                .any(|t| t.eq_ignore_ascii_case("edit") || t.eq_ignore_ascii_case("write"))
        })
        .unwrap_or(false);

    if has_edit_write {
        let mut seen_files = std::collections::HashSet::new();

        // FileEdit 렌더링: `> [!tool]- Edit \`path\``
        for cap in re_file_edit().captures_iter(body) {
            let path = cap[1].trim().to_string();
            if !path.is_empty() && seen_files.insert(path.clone()) {
                edges.push(GraphEdge {
                    source: session_node_id.clone(),
                    target: format!("file:{}", path),
                    relation: "modifies_file".to_string(),
                    confidence: "INFERRED".to_string(),
                    weight: 0.9,
                });
            }
        }

        // ToolUse 블록 input_summary 첫 줄: Edit/Write 뒤 코드블록 첫 줄
        for cap in re_tool_path().captures_iter(body) {
            let path = cap[1].trim().to_string();
            // 파일 경로처럼 보이는지 간단히 검증: '/' 또는 '\' 포함, 공백 없음
            if !path.is_empty()
                && (path.contains('/') || path.contains('\\'))
                && !path.contains(' ')
                && seen_files.insert(path.clone())
            {
                edges.push(GraphEdge {
                    source: session_node_id.clone(),
                    target: format!("file:{}", path),
                    relation: "modifies_file".to_string(),
                    confidence: "INFERRED".to_string(),
                    weight: 0.9,
                });
            }
        }
    }

    edges
}

/// 여러 세션 frontmatter 간 관계 추출 (same_project, same_day)
/// O(n²) 방지: 같은 project/day 내 시간순 인접 세션만 연결
pub fn extract_session_relations(sessions: &[SessionFrontmatter]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();

    // same_project: project별로 그룹핑 → 시간순 정렬 후 인접 연결
    let mut project_groups: std::collections::HashMap<String, Vec<&SessionFrontmatter>> =
        std::collections::HashMap::new();
    for fm in sessions {
        if let Some(project) = &fm.project {
            if !project.is_empty() {
                project_groups.entry(project.clone()).or_default().push(fm);
            }
        }
    }
    for (_, mut group) in project_groups {
        group.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        for window in group.windows(2) {
            let a = window[0];
            let b = window[1];
            edges.push(GraphEdge {
                source: format!("session:{}", a.session_id),
                target: format!("session:{}", b.session_id),
                relation: "same_project".to_string(),
                confidence: "EXTRACTED".to_string(),
                weight: 1.0,
            });
        }
    }

    // same_day: date별로 그룹핑 → 시간순 정렬 후 인접 연결
    let mut day_groups: std::collections::HashMap<String, Vec<&SessionFrontmatter>> =
        std::collections::HashMap::new();
    for fm in sessions {
        if !fm.date.is_empty() {
            day_groups.entry(fm.date.clone()).or_default().push(fm);
        }
    }
    for (_, mut group) in day_groups {
        if group.len() < 2 {
            continue;
        }
        group.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        for window in group.windows(2) {
            let a = window[0];
            let b = window[1];
            edges.push(GraphEdge {
                source: format!("session:{}", a.session_id),
                target: format!("session:{}", b.session_id),
                relation: "same_day".to_string(),
                confidence: "EXTRACTED".to_string(),
                weight: 0.5,
            });
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fm(
        id: &str,
        project: Option<&str>,
        agent: &str,
        tools: Option<Vec<&str>>,
        date: &str,
        start_time: &str,
    ) -> SessionFrontmatter {
        SessionFrontmatter {
            session_id: id.to_string(),
            agent: agent.to_string(),
            model: None,
            project: project.map(|s| s.to_string()),
            cwd: None,
            date: date.to_string(),
            start_time: start_time.to_string(),
            end_time: None,
            turns: Some(5),
            tokens_in: None,
            tokens_out: None,
            tools_used: tools.map(|t| t.iter().map(|s| s.to_string()).collect()),
            host: None,
            status: None,
            summary: Some("test summary".to_string()),
            session_type: None,
        }
    }

    #[test]
    fn test_extract_from_frontmatter() {
        let fm = make_fm(
            "abc12345",
            Some("tunaflow"),
            "claude-code",
            Some(vec!["Edit", "Read"]),
            "2026-04-10",
            "2026-04-10T00:00:00Z",
        );
        let result = extract_from_frontmatter(&fm);

        // session, project, agent, tool(Edit), tool(Read) = 5 nodes
        assert_eq!(result.nodes.len(), 5);
        assert!(result.nodes.iter().any(|n| n.node_type == "session"));
        assert!(result.nodes.iter().any(|n| n.node_type == "project"));
        assert!(result.nodes.iter().any(|n| n.node_type == "agent"));
        assert_eq!(
            result
                .nodes
                .iter()
                .filter(|n| n.node_type == "tool")
                .count(),
            2
        );

        // belongs_to, by_agent, uses_tool×2 = 4 edges
        assert_eq!(result.edges.len(), 4);
        assert!(result.edges.iter().any(|e| e.relation == "belongs_to"));
        assert!(result.edges.iter().any(|e| e.relation == "by_agent"));
        assert_eq!(
            result
                .edges
                .iter()
                .filter(|e| e.relation == "uses_tool")
                .count(),
            2
        );
    }

    #[test]
    fn test_extract_session_relations_same_project() {
        let sessions = vec![
            make_fm(
                "s1",
                Some("proj"),
                "claude-code",
                None,
                "2026-04-10",
                "2026-04-10T01:00:00Z",
            ),
            make_fm(
                "s2",
                Some("proj"),
                "claude-code",
                None,
                "2026-04-10",
                "2026-04-10T02:00:00Z",
            ),
            make_fm(
                "s3",
                Some("proj"),
                "claude-code",
                None,
                "2026-04-10",
                "2026-04-10T03:00:00Z",
            ),
        ];
        let edges = extract_session_relations(&sessions);
        let same_project: Vec<_> = edges
            .iter()
            .filter(|e| e.relation == "same_project")
            .collect();
        // 3개 세션 → 2개 인접 엣지
        assert_eq!(same_project.len(), 2);
    }

    #[test]
    fn test_extract_session_relations_same_day() {
        let sessions = vec![
            make_fm(
                "s1",
                None,
                "claude-code",
                None,
                "2026-04-10",
                "2026-04-10T01:00:00Z",
            ),
            make_fm(
                "s2",
                None,
                "claude-code",
                None,
                "2026-04-10",
                "2026-04-10T02:00:00Z",
            ),
            make_fm(
                "s3",
                None,
                "claude-code",
                None,
                "2026-04-11",
                "2026-04-11T01:00:00Z",
            ),
        ];
        let edges = extract_session_relations(&sessions);
        let same_day: Vec<_> = edges.iter().filter(|e| e.relation == "same_day").collect();
        // s1+s2만 같은 날 → 1개 엣지
        assert_eq!(same_day.len(), 1);
        assert!((same_day[0].weight - 0.5).abs() < f64::EPSILON);
    }

    // ─── extract_semantic_edges 테스트 ─────────────────────────────────────────

    fn make_fm_with_summary(
        id: &str,
        tools: Option<Vec<&str>>,
        summary: Option<&str>,
    ) -> SessionFrontmatter {
        SessionFrontmatter {
            session_id: id.to_string(),
            agent: "claude-code".to_string(),
            model: None,
            project: None,
            cwd: None,
            date: "2026-04-10".to_string(),
            start_time: "2026-04-10T00:00:00Z".to_string(),
            end_time: None,
            turns: None,
            tokens_in: None,
            tokens_out: None,
            tools_used: tools.map(|t| t.iter().map(|s| s.to_string()).collect()),
            host: None,
            status: None,
            summary: summary.map(|s| s.to_string()),
            session_type: None,
        }
    }

    #[test]
    fn test_semantic_fixes_bug_from_summary() {
        let fm = make_fm_with_summary("abc12345", None, Some("fixes #42 and closes #99"));
        let edges = extract_semantic_edges(&fm, "");
        let issues: Vec<_> = edges.iter().filter(|e| e.relation == "fixes_bug").collect();
        assert_eq!(issues.len(), 2, "summary에서 2개 이슈를 추출해야 함");
        assert!(issues.iter().any(|e| e.target == "issue:42"));
        assert!(issues.iter().any(|e| e.target == "issue:99"));
        assert_eq!(issues[0].confidence, "INFERRED");
        assert!((issues[0].weight - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_semantic_fixes_bug_from_body() {
        let fm = make_fm_with_summary("abc12345", None, None);
        let body = "## Turn 1 — User\n\nPlease resolve #7\n\n## Turn 2 — Assistant\n\nI will fix issue #7.";
        let edges = extract_semantic_edges(&fm, body);
        let issues: Vec<_> = edges.iter().filter(|e| e.relation == "fixes_bug").collect();
        // #7이 2번 언급되지만 중복 제거
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].target, "issue:7");
    }

    #[test]
    fn test_semantic_no_issues_if_empty() {
        let fm = make_fm_with_summary("abc12345", None, None);
        let edges = extract_semantic_edges(&fm, "some body without issues");
        assert!(edges.iter().all(|e| e.relation != "fixes_bug"));
    }

    #[test]
    fn test_semantic_modifies_file_from_file_edit() {
        let fm = make_fm_with_summary("abc12345", Some(vec!["Edit"]), None);
        let body = "> [!tool]- Edit `crates/secall-core/src/lib.rs`\n\n";
        let edges = extract_semantic_edges(&fm, body);
        let files: Vec<_> = edges
            .iter()
            .filter(|e| e.relation == "modifies_file")
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].target, "file:crates/secall-core/src/lib.rs");
        assert_eq!(files[0].confidence, "INFERRED");
        assert!((files[0].weight - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_semantic_modifies_file_dedup() {
        let fm = make_fm_with_summary("abc12345", Some(vec!["Edit"]), None);
        // 같은 파일을 두 번 편집
        let body = "> [!tool]- Edit `src/main.rs`\n\n> [!tool]- Edit `src/main.rs`\n\n";
        let edges = extract_semantic_edges(&fm, body);
        let files: Vec<_> = edges
            .iter()
            .filter(|e| e.relation == "modifies_file")
            .collect();
        assert_eq!(files.len(), 1, "동일 파일은 중복 제거");
    }

    #[test]
    fn test_semantic_modifies_file_skipped_without_edit_tool() {
        // tools_used에 Edit/Write 없으면 modifies_file 추출 안 함
        let fm = make_fm_with_summary("abc12345", Some(vec!["Read", "Bash"]), None);
        let body = "> [!tool]- Edit `src/main.rs`\n\n";
        let edges = extract_semantic_edges(&fm, body);
        assert!(edges.iter().all(|e| e.relation != "modifies_file"));
    }
}
