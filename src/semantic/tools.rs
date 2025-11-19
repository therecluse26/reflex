//! Tool execution system for agentic context gathering
//!
//! This module handles execution of tool calls from the LLM including:
//! - Running `rfx context` commands
//! - Executing exploratory queries
//! - Running codebase analysis (hotspots, unused files, etc.)

use anyhow::{Context as AnyhowContext, Result};
use crate::cache::CacheManager;
use crate::dependency::DependencyIndex;
use crate::query::QueryEngine;

use super::executor::parse_command;
use super::schema_agentic::{ToolCall, ContextGatheringParams, AnalysisType};

/// Result of executing a tool call
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Description of what this tool did
    pub description: String,

    /// The output/result from the tool
    pub output: String,

    /// Whether the tool execution was successful
    pub success: bool,
}

/// Execute a single tool call
pub async fn execute_tool(
    tool: &ToolCall,
    cache: &CacheManager,
) -> Result<ToolResult> {
    match tool {
        ToolCall::GatherContext { params } => {
            execute_gather_context(params, cache)
        }
        ToolCall::ExploreCodebase { description, command } => {
            execute_explore_codebase(description, command, cache).await
        }
        ToolCall::AnalyzeStructure { analysis_type } => {
            execute_analyze_structure(*analysis_type, cache)
        }
        ToolCall::SearchDocumentation { query, files } => {
            execute_search_documentation(query, files.as_deref(), cache)
        }
    }
}

/// Execute context gathering tool
fn execute_gather_context(
    params: &ContextGatheringParams,
    cache: &CacheManager,
) -> Result<ToolResult> {
    log::info!("Executing gather_context tool");

    // Build context options from params
    let mut opts = crate::context::ContextOptions {
        structure: params.structure,
        path: params.path.clone(),
        file_types: params.file_types,
        project_type: params.project_type,
        framework: params.framework,
        entry_points: params.entry_points,
        test_layout: params.test_layout,
        config_files: params.config_files,
        depth: params.depth,
        json: false, // Always use text format for LLM consumption
    };

    // If no specific flags, default to --full (all context types)
    if opts.is_empty() {
        opts.enable_all();
    }

    // Generate context
    let output = crate::context::generate_context(cache, &opts)
        .context("Failed to generate codebase context")?;

    // Build description of what was gathered
    let mut parts = Vec::new();
    if opts.structure { parts.push("structure"); }
    if opts.file_types { parts.push("file types"); }
    if opts.project_type { parts.push("project type"); }
    if opts.framework { parts.push("frameworks"); }
    if opts.entry_points { parts.push("entry points"); }
    if opts.test_layout { parts.push("test layout"); }
    if opts.config_files { parts.push("config files"); }

    let description = if parts.is_empty() {
        "Gathered general codebase context".to_string()
    } else {
        format!("Gathered codebase context: {}", parts.join(", "))
    };

    log::debug!("Context gathering successful: {} chars", output.len());

    Ok(ToolResult {
        description,
        output,
        success: true,
    })
}

/// Execute exploratory codebase query
async fn execute_explore_codebase(
    description: &str,
    command: &str,
    cache: &CacheManager,
) -> Result<ToolResult> {
    log::info!("Executing explore_codebase tool: {}", description);

    // Parse the command
    let parsed = parse_command(command)
        .with_context(|| format!("Failed to parse exploration command: {}", command))?;

    // Convert to QueryFilter
    let filter = parsed.to_query_filter()?;

    // Create query engine
    let engine = QueryEngine::new(CacheManager::new(cache.workspace_root()));

    // Execute query
    let response = engine.search_with_metadata(&parsed.pattern, filter)
        .with_context(|| format!("Failed to execute exploration query: {}", command))?;

    // Format results for LLM consumption
    let output = format_exploration_results(&response, &parsed.pattern);

    log::debug!("Exploration query found {} file groups", response.results.len());

    Ok(ToolResult {
        description: format!("Explored: {}", description),
        output,
        success: true,
    })
}

/// Execute structure analysis (hotspots, unused files, etc.)
fn execute_analyze_structure(
    analysis_type: AnalysisType,
    cache: &CacheManager,
) -> Result<ToolResult> {
    log::info!("Executing analyze_structure tool: {:?}", analysis_type);

    // Create dependency index
    let deps_index = DependencyIndex::new(CacheManager::new(cache.workspace_root()));

    let output = match analysis_type {
        AnalysisType::Hotspots => {
            // Get hotspots (returns file IDs and counts)
            let hotspot_ids = deps_index.find_hotspots(Some(10), 2)?; // top 10, min 2 dependents

            // Convert file IDs to paths
            let file_ids: Vec<i64> = hotspot_ids.iter().map(|(id, _)| *id).collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            // Convert to (String, usize) format
            let hotspots: Vec<(String, usize)> = hotspot_ids.iter()
                .filter_map(|(id, count)| {
                    paths.get(id).map(|path| (path.clone(), *count))
                })
                .collect();

            format_hotspots(&hotspots)
        }
        AnalysisType::Unused => {
            // Get unused files (returns file IDs)
            let unused_ids = deps_index.find_unused_files()?;

            // Convert file IDs to paths
            let paths = deps_index.get_file_paths(&unused_ids)?;
            let unused: Vec<String> = unused_ids.iter()
                .filter_map(|id| paths.get(id).cloned())
                .collect();

            format_unused_files(&unused)
        }
        AnalysisType::Circular => {
            // Get circular dependencies (returns vectors of file IDs)
            let circular_ids = deps_index.detect_circular_dependencies()?;

            // Collect all unique file IDs
            let all_ids: Vec<i64> = circular_ids.iter()
                .flat_map(|cycle| cycle.iter())
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            // Convert all IDs to paths
            let paths = deps_index.get_file_paths(&all_ids)?;

            // Convert cycles to path cycles
            let circular: Vec<Vec<String>> = circular_ids.iter()
                .map(|cycle| {
                    cycle.iter()
                        .filter_map(|id| paths.get(id).cloned())
                        .collect()
                })
                .collect();

            format_circular_deps(&circular)
        }
    };

    let description = match analysis_type {
        AnalysisType::Hotspots => "Analyzed dependency hotspots (most-imported files)",
        AnalysisType::Unused => "Analyzed unused files (no importers)",
        AnalysisType::Circular => "Analyzed circular dependencies",
    };

    log::debug!("Analysis complete: {} chars", output.len());

    Ok(ToolResult {
        description: description.to_string(),
        output,
        success: true,
    })
}

/// Execute documentation search tool
fn execute_search_documentation(
    query: &str,
    files: Option<&[String]>,
    cache: &CacheManager,
) -> Result<ToolResult> {
    log::info!("Executing search_documentation tool: query='{}'", query);

    let workspace_root = cache.workspace_root();

    // Default documentation files to search
    let default_files = vec!["CLAUDE.md".to_string(), "README.md".to_string()];
    let search_files = files.unwrap_or(&default_files);

    let mut found_sections = Vec::new();
    let mut searched_files = Vec::new();

    // Search specified documentation files
    for file in search_files {
        let file_path = workspace_root.join(file);

        if !file_path.exists() {
            log::debug!("Documentation file does not exist: {}", file);
            continue;
        }

        searched_files.push(file.clone());

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                // Search for query keywords in the content
                if let Some(sections) = search_documentation_content(&content, query, file) {
                    found_sections.push(sections);
                }
            }
            Err(e) => {
                log::warn!("Failed to read documentation file {}: {}", file, e);
            }
        }
    }

    // Also search .context/ directory for markdown files
    let context_dir = workspace_root.join(".context");
    if context_dir.exists() && context_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&context_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Some(sections) = search_documentation_content(
                                &content,
                                query,
                                &format!(".context/{}", file_name),
                            ) {
                                found_sections.push(sections);
                                searched_files.push(format!(".context/{}", file_name));
                            }
                        }
                    }
                }
            }
        }
    }

    // Format output
    let output = if found_sections.is_empty() {
        format!(
            "No relevant documentation found for query '{}' in files: {}\n\nTry:\n- Using different keywords\n- Searching the codebase directly with explore_codebase",
            query,
            searched_files.join(", ")
        )
    } else {
        format!(
            "Found documentation for '{}' in {} file(s):\n\n{}",
            query,
            found_sections.len(),
            found_sections.join("\n\n---\n\n")
        )
    };

    log::debug!("Documentation search found {} sections", found_sections.len());

    Ok(ToolResult {
        description: format!("Searched documentation for: {}", query),
        output,
        success: !found_sections.is_empty(),
    })
}

/// Search documentation content for query and extract relevant sections
fn search_documentation_content(content: &str, query: &str, file_name: &str) -> Option<String> {
    let query_lower = query.to_lowercase();
    let lines: Vec<&str> = content.lines().collect();

    let mut relevant_sections = Vec::new();
    let mut current_section = String::new();
    let mut current_section_title = String::new();
    let mut in_relevant_section = false;
    let mut relevance_score = 0;

    for line in lines.iter() {
        let line_lower = line.to_lowercase();

        // Check if this is a heading
        if line.starts_with('#') {
            // Save previous section if it was relevant
            if in_relevant_section && relevance_score > 0 {
                relevant_sections.push(format!(
                    "## {} ({})\n\n{}",
                    current_section_title,
                    file_name,
                    current_section.trim()
                ));
            }

            // Start new section
            current_section.clear();
            current_section_title = line.trim_start_matches('#').trim().to_string();
            relevance_score = 0;
            in_relevant_section = false;

            // Check if heading contains query keywords
            if line_lower.contains(&query_lower) {
                in_relevant_section = true;
                relevance_score += 10;
            }
        }

        // Check if content contains query keywords
        if line_lower.contains(&query_lower) {
            in_relevant_section = true;
            relevance_score += 1;
        }

        // Add line to current section (with some context)
        if in_relevant_section || relevance_score > 0 {
            current_section.push_str(line);
            current_section.push('\n');

            // Add a few lines of context after matches
            if relevance_score > 0 && !line_lower.contains(&query_lower) {
                // Keep adding context lines
                if current_section.lines().count() > 100 {
                    // Limit section size
                    break;
                }
            }
        }
    }

    // Save last section if relevant
    if in_relevant_section && relevance_score > 0 {
        relevant_sections.push(format!(
            "## {} ({})\n\n{}",
            current_section_title,
            file_name,
            current_section.trim()
        ));
    }

    if relevant_sections.is_empty() {
        None
    } else {
        Some(relevant_sections.join("\n\n"))
    }
}

/// Format exploration query results for LLM
fn format_exploration_results(
    response: &crate::models::QueryResponse,
    pattern: &str,
) -> String {
    if response.results.is_empty() {
        return format!("No results found for pattern: {}", pattern);
    }

    let mut output = Vec::new();
    output.push(format!(
        "Found {} total matches across {} files for pattern '{}':\n",
        response.pagination.total,
        response.results.len(),
        pattern
    ));

    // Show first 5 file groups
    for (idx, file_group) in response.results.iter().take(5).enumerate() {
        output.push(format!("\n{}. {}", idx + 1, file_group.path));

        // Show first 3 matches per file
        for match_result in file_group.matches.iter().take(3) {
            // Show context before the match
            for (idx, line) in match_result.context_before.iter().enumerate() {
                let line_num = match_result.span.start_line.saturating_sub(match_result.context_before.len() - idx);
                output.push(format!("   Line {}: {}", line_num, line.trim()));
            }

            // Show the match line itself
            output.push(format!(
                "   Line {}: {}",
                match_result.span.start_line,
                match_result.preview.lines().next().unwrap_or("").trim()
            ));

            // Show context after the match
            for (idx, line) in match_result.context_after.iter().enumerate() {
                let line_num = match_result.span.start_line + idx + 1;
                output.push(format!("   Line {}: {}", line_num, line.trim()));
            }
        }

        if file_group.matches.len() > 3 {
            output.push(format!("   ... and {} more matches", file_group.matches.len() - 3));
        }
    }

    if response.results.len() > 5 {
        output.push(format!("\n... and {} more files", response.results.len() - 5));
    }

    output.join("\n")
}

/// Format hotspot analysis results
fn format_hotspots(hotspots: &[(String, usize)]) -> String {
    if hotspots.is_empty() {
        return "No dependency hotspots found.".to_string();
    }

    let mut output = Vec::new();
    output.push(format!("Top {} most-imported files:\n", hotspots.len().min(10)));

    for (idx, (path, count)) in hotspots.iter().take(10).enumerate() {
        output.push(format!("{}. {} ({} importers)", idx + 1, path, count));
    }

    if hotspots.len() > 10 {
        output.push(format!("\n... and {} more hotspots", hotspots.len() - 10));
    }

    output.join("\n")
}

/// Format unused files analysis results
fn format_unused_files(unused: &[String]) -> String {
    if unused.is_empty() {
        return "No unused files found (all files are imported by others).".to_string();
    }

    let mut output = Vec::new();
    output.push(format!("Found {} unused files (no importers):\n", unused.len()));

    for (idx, path) in unused.iter().take(15).enumerate() {
        output.push(format!("{}. {}", idx + 1, path));
    }

    if unused.len() > 15 {
        output.push(format!("\n... and {} more unused files", unused.len() - 15));
    }

    output.join("\n")
}

/// Format circular dependency analysis results
fn format_circular_deps(circular: &[Vec<String>]) -> String {
    if circular.is_empty() {
        return "No circular dependencies found.".to_string();
    }

    let mut output = Vec::new();
    output.push(format!("Found {} circular dependency chains:\n", circular.len()));

    for (idx, cycle) in circular.iter().take(5).enumerate() {
        output.push(format!("\n{}. Cycle ({} files):", idx + 1, cycle.len()));
        output.push(format!("   {}", cycle.join(" → ")));
    }

    if circular.len() > 5 {
        output.push(format!("\n... and {} more circular dependencies", circular.len() - 5));
    }

    output.join("\n")
}

/// Format all tool results into a single context string for the next LLM call
pub fn format_tool_results(results: &[ToolResult]) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut output = Vec::new();
    output.push("## Tool Execution Results\n".to_string());

    for (idx, result) in results.iter().enumerate() {
        output.push(format!("\n### Tool {} - {}", idx + 1, result.description));
        output.push(String::new());
        output.push(result.output.clone());
        output.push(String::new());
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tool_results_empty() {
        let results = vec![];
        let output = format_tool_results(&results);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_tool_results_single() {
        let results = vec![ToolResult {
            description: "Test tool".to_string(),
            output: "Test output".to_string(),
            success: true,
        }];

        let output = format_tool_results(&results);
        assert!(output.contains("Tool Execution Results"));
        assert!(output.contains("Test tool"));
        assert!(output.contains("Test output"));
    }

    #[test]
    fn test_format_hotspots() {
        let hotspots = vec![
            ("src/main.rs".to_string(), 10),
            ("src/lib.rs".to_string(), 5),
        ];

        let output = format_hotspots(&hotspots);
        assert!(output.contains("most-imported files"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("10 importers"));
    }

    #[test]
    fn test_format_unused_files() {
        let unused = vec![
            "src/old.rs".to_string(),
            "tests/legacy.rs".to_string(),
        ];

        let output = format_unused_files(&unused);
        assert!(output.contains("unused files"));
        assert!(output.contains("src/old.rs"));
    }

    #[test]
    fn test_format_circular_deps() {
        let circular = vec![
            vec!["a.rs".to_string(), "b.rs".to_string(), "a.rs".to_string()],
        ];

        let output = format_circular_deps(&circular);
        assert!(output.contains("circular dependency"));
        assert!(output.contains("a.rs → b.rs → a.rs"));
    }
}
