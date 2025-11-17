//! Tool execution system for agentic context gathering
//!
//! This module handles execution of tool calls from the LLM including:
//! - Running `rfx context` commands
//! - Executing exploratory queries
//! - Running codebase analysis (hotspots, unused files, etc.)

use anyhow::{Context as AnyhowContext, Result};
use std::collections::HashSet;
use crate::cache::CacheManager;
use crate::dependency::DependencyIndex;
use crate::query::{QueryEngine, QueryFilter};

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

    // If no specific flags, default to structure + file_types
    if !opts.structure && !opts.file_types && !opts.project_type
       && !opts.framework && !opts.entry_points && !opts.test_layout
       && !opts.config_files {
        opts.structure = true;
        opts.file_types = true;
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
            output.push(format!(
                "   Line {}: {}",
                match_result.span.start_line,
                match_result.preview.lines().next().unwrap_or("").trim()
            ));
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
