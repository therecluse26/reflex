//! Progress reporting for agentic loop
//!
//! This module provides transparent "show your work" output for the agentic loop,
//! displaying the LLM's reasoning at each phase similar to Claude Code's thinking blocks.

use owo_colors::OwoColorize;

use super::schema_agentic::{ToolCall, EvaluationReport};
use super::tools::ToolResult;

/// Trait for reporting agentic loop progress
pub trait AgenticReporter: Send + Sync {
    /// Report assessment phase completion
    fn report_assessment(&self, reasoning: &str, needs_context: bool, tools: &[ToolCall]);

    /// Report start of tool execution
    fn report_tool_start(&self, idx: usize, tool: &ToolCall);

    /// Report tool execution completion
    fn report_tool_complete(&self, idx: usize, result: &ToolResult);

    /// Report query generation completion
    fn report_generation(&self, reasoning: Option<&str>, query_count: usize, confidence: f32);

    /// Report evaluation results
    fn report_evaluation(&self, evaluation: &EvaluationReport);

    /// Report refinement start
    fn report_refinement_start(&self);

    /// Report phase start
    fn report_phase(&self, phase_num: usize, phase_name: &str);
}

/// Console reporter with colored output
pub struct ConsoleReporter {
    /// Show LLM reasoning blocks
    show_reasoning: bool,

    /// Verbose output (show tool results, etc.)
    verbose: bool,
}

impl ConsoleReporter {
    /// Create a new console reporter
    pub fn new(show_reasoning: bool, verbose: bool) -> Self {
        Self {
            show_reasoning,
            verbose,
        }
    }

    /// Display formatted reasoning block with line prefix
    fn display_reasoning_block(&self, reasoning: &str) {
        for line in reasoning.lines() {
            if line.trim().is_empty() {
                println!();
            } else {
                println!("  {}", line.dimmed());
            }
        }
    }

    /// Describe a tool for display
    fn describe_tool(&self, tool: &ToolCall) -> String {
        match tool {
            ToolCall::GatherContext { params } => {
                let mut parts = Vec::new();
                if params.structure { parts.push("structure"); }
                if params.file_types { parts.push("file types"); }
                if params.project_type { parts.push("project type"); }
                if params.framework { parts.push("frameworks"); }
                if params.entry_points { parts.push("entry points"); }
                if params.test_layout { parts.push("test layout"); }
                if params.config_files { parts.push("config files"); }

                if parts.is_empty() {
                    "gather_context: General codebase context".to_string()
                } else {
                    format!("gather_context: {}", parts.join(", "))
                }
            }
            ToolCall::ExploreCodebase { description, command } => {
                format!("explore_codebase: {} ({})", description, command)
            }
            ToolCall::AnalyzeStructure { analysis_type } => {
                format!("analyze_structure: {:?}", analysis_type)
            }
        }
    }

    /// Truncate text for preview display
    fn truncate(&self, text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            return text.to_string();
        }

        let truncated = &text[..max_len];
        format!("{}...", truncated)
    }
}

impl AgenticReporter for ConsoleReporter {
    fn report_phase(&self, phase_num: usize, phase_name: &str) {
        println!("\n{}", format!("━━━ Phase {}: {} ━━━", phase_num, phase_name).bold().cyan());
    }

    fn report_assessment(&self, reasoning: &str, needs_context: bool, tools: &[ToolCall]) {
        self.report_phase(1, "Assessment");

        if self.show_reasoning && !reasoning.is_empty() {
            println!("\n{}", "💭 Reasoning:".dimmed());
            self.display_reasoning_block(reasoning);
        }

        println!();
        if needs_context && !tools.is_empty() {
            println!("{} {}", "→".bright_green(), "Needs additional context".bold());
            println!("  {} tool(s) to execute:", tools.len());
            for (i, tool) in tools.iter().enumerate() {
                println!("  {}. {}", (i + 1).to_string().bright_white(), self.describe_tool(tool).dimmed());
            }
        } else {
            println!("{} {}", "→".bright_green(), "Has sufficient context".bold());
            println!("  Proceeding directly to query generation");
        }
    }

    fn report_tool_start(&self, idx: usize, tool: &ToolCall) {
        if idx == 1 {
            self.report_phase(2, "Context Gathering");
            println!();
        }

        if self.verbose {
            println!("  {} Executing: {}", "⋯".dimmed(), self.describe_tool(tool).dimmed());
        }
    }

    fn report_tool_complete(&self, idx: usize, result: &ToolResult) {
        if result.success {
            println!("  {} {} {}",
                "✓".bright_green(),
                format!("[{}]", idx).dimmed(),
                result.description
            );

            if self.verbose && !result.output.is_empty() {
                // Show truncated output
                let preview = self.truncate(&result.output, 150);
                for line in preview.lines().take(3) {
                    println!("    {}", line.dimmed());
                }
                if result.output.lines().count() > 3 {
                    println!("    {}", "...".dimmed());
                }
            }
        } else {
            println!("  {} {} {} - {}",
                "✗".bright_red(),
                format!("[{}]", idx).dimmed(),
                result.description,
                "failed".red()
            );
        }
    }

    fn report_generation(&self, reasoning: Option<&str>, query_count: usize, confidence: f32) {
        self.report_phase(3, "Query Generation");

        if self.show_reasoning {
            if let Some(reasoning_text) = reasoning {
                if !reasoning_text.is_empty() {
                    println!("\n{}", "💭 Reasoning:".dimmed());
                    self.display_reasoning_block(reasoning_text);
                }
            }
        }

        println!();
        let confidence_pct = (confidence * 100.0) as u8;

        print!("{} Generated {} {} (confidence: ",
            "→".bright_green(),
            query_count,
            if query_count == 1 { "query" } else { "queries" }
        );

        if confidence >= 0.8 {
            println!("{}%)", confidence_pct.to_string().bright_green());
        } else if confidence >= 0.6 {
            println!("{}%)", confidence_pct.to_string().yellow());
        } else {
            println!("{}%)", confidence_pct.to_string().bright_red());
        }
    }

    fn report_evaluation(&self, evaluation: &EvaluationReport) {
        self.report_phase(5, "Evaluation");
        println!();

        if evaluation.success {
            println!("{} {} (score: {}/1.0)",
                "✓".bright_green(),
                "Success".bold().bright_green(),
                format!("{:.2}", evaluation.score).bright_white()
            );

            if self.verbose && !evaluation.issues.is_empty() {
                println!("\n  Minor issues noted:");
                for issue in &evaluation.issues {
                    println!("  - {} (severity: {:.2})",
                        issue.description.dimmed(),
                        issue.severity
                    );
                }
            }
        } else {
            println!("{} {} (score: {}/1.0)",
                "⚠".yellow(),
                "Results need refinement".bold().yellow(),
                format!("{:.2}", evaluation.score).bright_white()
            );

            if !evaluation.issues.is_empty() {
                println!("\n  Issues found:");
                for (idx, issue) in evaluation.issues.iter().enumerate().take(3) {
                    println!("  {}. {}",
                        (idx + 1).to_string().dimmed(),
                        issue.description
                    );
                }
            }

            if !evaluation.suggestions.is_empty() {
                println!("\n  Suggestions:");
                for (idx, suggestion) in evaluation.suggestions.iter().enumerate().take(3) {
                    println!("  {}. {}",
                        (idx + 1).to_string().dimmed(),
                        suggestion.dimmed()
                    );
                }
            }
        }
    }

    fn report_refinement_start(&self) {
        self.report_phase(6, "Refinement");
        println!();
        println!("{} Refining queries based on evaluation feedback...", "→".yellow());
    }
}

/// No-op reporter for quiet mode
pub struct QuietReporter;

impl AgenticReporter for QuietReporter {
    fn report_assessment(&self, _reasoning: &str, _needs_context: bool, _tools: &[ToolCall]) {}
    fn report_tool_start(&self, _idx: usize, _tool: &ToolCall) {}
    fn report_tool_complete(&self, _idx: usize, _result: &ToolResult) {}
    fn report_generation(&self, _reasoning: Option<&str>, _query_count: usize, _confidence: f32) {}
    fn report_evaluation(&self, _evaluation: &EvaluationReport) {}
    fn report_refinement_start(&self) {}
    fn report_phase(&self, _phase_num: usize, _phase_name: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::schema_agentic::*;

    #[test]
    fn test_console_reporter_creation() {
        let reporter = ConsoleReporter::new(true, false);
        assert!(reporter.show_reasoning);
        assert!(!reporter.verbose);
    }

    #[test]
    fn test_truncate() {
        let reporter = ConsoleReporter::new(false, false);
        let text = "a".repeat(300);
        let truncated = reporter.truncate(&text, 100);
        assert!(truncated.len() <= 103); // 100 + "..."
    }

    #[test]
    fn test_describe_gather_context_tool() {
        let reporter = ConsoleReporter::new(false, false);
        let tool = ToolCall::GatherContext {
            params: ContextGatheringParams {
                structure: true,
                file_types: true,
                ..Default::default()
            },
        };

        let desc = reporter.describe_tool(&tool);
        assert!(desc.contains("gather_context"));
        assert!(desc.contains("structure"));
    }
}
