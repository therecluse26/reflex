//! Answer generation from search results
//!
//! This module provides functionality to synthesize conversational answers
//! from code search results using LLM providers.

use anyhow::Result;
use crate::models::FileGroupedResult;
use super::providers::LlmProvider;

/// Maximum number of matches to include in the prompt (to avoid token limits)
const MAX_MATCHES_IN_PROMPT: usize = 50;

/// Maximum preview length per match (characters)
const MAX_PREVIEW_LENGTH: usize = 200;

/// Generate a conversational answer based on search results
///
/// Takes the user's original question and search results, then calls the LLM
/// to synthesize a natural language answer that references specific files and
/// line numbers from the results.
///
/// # Arguments
///
/// * `question` - The original user question
/// * `results` - Search results grouped by file
/// * `total_count` - Total number of matches found
/// * `provider` - LLM provider to use for answer generation
///
/// # Returns
///
/// A conversational answer string that summarizes the findings
pub async fn generate_answer(
    question: &str,
    results: &[FileGroupedResult],
    total_count: usize,
    provider: &dyn LlmProvider,
) -> Result<String> {
    // Handle empty results
    if results.is_empty() {
        return Ok(format!("No results found for: {}", question));
    }

    // Build the prompt with search results
    let prompt = build_answer_prompt(question, results, total_count);

    log::debug!("Generating answer with prompt ({} chars)", prompt.len());

    // Call LLM to generate answer (json_mode: false for plain text output)
    let answer = provider.complete(&prompt, false).await?;

    // Clean up the response (remove markdown fences if present)
    let cleaned = strip_markdown_fences(&answer);

    Ok(cleaned.to_string())
}

/// Build the prompt for answer generation
fn build_answer_prompt(
    question: &str,
    results: &[FileGroupedResult],
    total_count: usize,
) -> String {
    let mut prompt = String::new();

    // Instructions
    prompt.push_str("You are analyzing code search results to answer a developer's question.\n\n");
    prompt.push_str("IMPORTANT: Provide ONLY the answer text, without any markdown formatting, code fences, or explanatory prefixes.\n\n");

    prompt.push_str(&format!("Question: {}\n\n", question));

    // Add search result summary
    prompt.push_str(&format!("Found {} total matches across {} files.\n\n", total_count, results.len()));

    prompt.push_str("Search Results:\n");
    prompt.push_str("==============\n\n");

    // Format results for the prompt (limit to avoid token overflow)
    let mut match_count = 0;
    for file_group in results {
        if match_count >= MAX_MATCHES_IN_PROMPT {
            prompt.push_str(&format!("\n... and {} more matches not shown\n", total_count - match_count));
            break;
        }

        prompt.push_str(&format!("File: {}\n", file_group.path));

        for match_result in &file_group.matches {
            if match_count >= MAX_MATCHES_IN_PROMPT {
                break;
            }

            // Truncate preview if too long
            let preview = if match_result.preview.len() > MAX_PREVIEW_LENGTH {
                format!("{}...", &match_result.preview[..MAX_PREVIEW_LENGTH])
            } else {
                match_result.preview.clone()
            };

            prompt.push_str(&format!(
                "  Line {}-{}: {}\n",
                match_result.span.start_line,
                match_result.span.end_line,
                preview.trim()
            ));

            match_count += 1;
        }

        prompt.push_str("\n");
    }

    // Instructions for answer format
    prompt.push_str("\nProvide a conversational answer that:\n");
    prompt.push_str("1. Directly answers the question based on the search results\n");
    prompt.push_str("2. References specific files and line numbers where relevant\n");
    prompt.push_str("3. Summarizes patterns or common approaches if multiple results are similar\n");
    prompt.push_str("4. Is concise but informative (typically 2-4 sentences)\n");
    prompt.push_str("5. Only mentions information that appears in the search results above\n\n");

    prompt.push_str("Answer (plain text only, no markdown):\n");

    prompt
}

/// Strip markdown code fences from LLM response
///
/// Some LLMs add markdown formatting even when instructed not to.
fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();

    // Check for markdown code fence pattern
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        // Remove opening fence (either ```markdown, ```text, or just ```)
        let without_start = if let Some(rest) = trimmed.strip_prefix("```markdown") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("```text") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("```") {
            rest
        } else {
            return trimmed;
        };

        // Remove closing fence
        let without_end = without_start.strip_suffix("```")
            .unwrap_or(without_start);

        without_end.trim()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_fences() {
        let input = "```\nThis is the answer\n```";
        assert_eq!(strip_markdown_fences(input), "This is the answer");
    }

    #[test]
    fn test_strip_markdown_fences_with_language() {
        let input = "```text\nThis is the answer\n```";
        assert_eq!(strip_markdown_fences(input), "This is the answer");
    }

    #[test]
    fn test_strip_markdown_fences_no_fences() {
        let input = "This is the answer";
        assert_eq!(strip_markdown_fences(input), "This is the answer");
    }

    #[test]
    fn test_build_answer_prompt_empty_results() {
        let results: Vec<FileGroupedResult> = vec![];
        let prompt = build_answer_prompt("Find TODOs", &results, 0);

        assert!(prompt.contains("Found 0 total matches"));
        assert!(prompt.contains("Question: Find TODOs"));
    }
}
