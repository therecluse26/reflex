//! Answer generation from search results
//!
//! This module provides functionality to synthesize conversational answers
//! from code search results using LLM providers.

use anyhow::Result;
use crate::models::FileGroupedResult;
use crate::cache::CacheManager;
use super::providers::LlmProvider;
use super::schema::QueryCommand;
use std::sync::Arc;

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
/// * `gathered_context` - Optional context gathered from tools (documentation, codebase structure)
/// * `codebase_context` - Optional codebase metadata (always available, language distribution, directories)
/// * `conversation_history` - Optional conversation history from chat session
/// * `provider` - LLM provider to use for answer generation
///
/// # Returns
///
/// A conversational answer string that summarizes the findings
pub async fn generate_answer(
    question: &str,
    results: &[FileGroupedResult],
    total_count: usize,
    gathered_context: Option<&str>,
    codebase_context: Option<&str>,
    conversation_history: Option<&str>,
    provider: &dyn LlmProvider,
) -> Result<String> {
    // Handle empty results - use gathered context if available, then codebase context
    if results.is_empty() {
        // Try gathered context first (from tools like search_documentation, gather_context)
        if let Some(context) = gathered_context {
            if !context.is_empty() {
                // Generate answer from documentation/context alone
                let prompt = build_context_only_prompt(question, context, conversation_history);
                log::debug!("Generating answer from gathered context ({} chars)", prompt.len());
                let answer = provider.complete(&prompt, false).await?;
                let cleaned = strip_markdown_fences(&answer);
                return Ok(cleaned.to_string());
            }
        }

        // Try codebase context (language distribution, file counts, directories)
        if let Some(context) = codebase_context {
            if !context.is_empty() {
                // Generate answer from codebase metadata alone
                let prompt = build_codebase_context_prompt(question, context, conversation_history);
                log::debug!("Generating answer from codebase context ({} chars)", prompt.len());
                let answer = provider.complete(&prompt, false).await?;
                let cleaned = strip_markdown_fences(&answer);
                return Ok(cleaned.to_string());
            }
        }

        return Ok(format!("No results found for: {}", question));
    }

    // Build the prompt with search results (and optional gathered context and conversation history)
    let prompt = build_answer_prompt(question, results, total_count, gathered_context, conversation_history);

    log::debug!("Generating answer with prompt ({} chars)", prompt.len());

    // Call LLM to generate answer (json_mode: false for plain text output)
    let answer = provider.complete(&prompt, false).await?;

    // Clean up the response (remove markdown fences if present)
    let cleaned = strip_markdown_fences(&answer);

    Ok(cleaned.to_string())
}

/// Build the prompt for answer generation (with optional gathered context and conversation history)
fn build_answer_prompt(
    question: &str,
    results: &[FileGroupedResult],
    total_count: usize,
    gathered_context: Option<&str>,
    conversation_history: Option<&str>,
) -> String {
    let mut prompt = String::new();

    // Instructions
    prompt.push_str("You are analyzing code search results to answer a developer's question.\n\n");
    prompt.push_str("IMPORTANT: Provide ONLY the answer text, without any markdown formatting, code fences, or explanatory prefixes.\n\n");

    // Include conversation history if available
    if let Some(history) = conversation_history {
        if !history.is_empty() {
            prompt.push_str(history);
            prompt.push_str("\n");
        }
    }

    prompt.push_str(&format!("Question: {}\n\n", question));

    // Add gathered context if available (documentation, codebase structure)
    if let Some(context) = gathered_context {
        if !context.is_empty() {
            prompt.push_str("Additional Context (from documentation and codebase analysis):\n");
            prompt.push_str("====================================================================\n\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }
    }

    // Add search result summary
    prompt.push_str(&format!("Found {} total matches across {} files.\n\n", total_count, results.len()));

    prompt.push_str("Code Search Results:\n");
    prompt.push_str("====================\n\n");

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

            log::debug!("Formatting match at {}:{} - context_before: {}, context_after: {}",
                file_group.path, match_result.span.start_line,
                match_result.context_before.len(), match_result.context_after.len());

            // Show context before the match
            for (idx, line) in match_result.context_before.iter().enumerate() {
                let line_num = match_result.span.start_line.saturating_sub(match_result.context_before.len() - idx);
                // Truncate long lines
                let truncated = if line.len() > MAX_PREVIEW_LENGTH {
                    format!("{}...", &line[..MAX_PREVIEW_LENGTH])
                } else {
                    line.clone()
                };
                prompt.push_str(&format!("  Line {}: {}\n", line_num, truncated.trim()));
            }

            // Show the match line itself
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

            // Show context after the match
            for (idx, line) in match_result.context_after.iter().enumerate() {
                let line_num = match_result.span.start_line + idx + 1;
                // Truncate long lines
                let truncated = if line.len() > MAX_PREVIEW_LENGTH {
                    format!("{}...", &line[..MAX_PREVIEW_LENGTH])
                } else {
                    line.clone()
                };
                prompt.push_str(&format!("  Line {}: {}\n", line_num, truncated.trim()));
            }

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

/// Build prompt for answering from context alone (no code search results)
fn build_context_only_prompt(question: &str, gathered_context: &str, conversation_history: Option<&str>) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are answering a developer's question using documentation and codebase context.\n\n");
    prompt.push_str("IMPORTANT: Provide ONLY the answer text, without any markdown formatting, code fences, or explanatory prefixes.\n\n");

    // Include conversation history if available
    if let Some(history) = conversation_history {
        if !history.is_empty() {
            prompt.push_str(history);
            prompt.push_str("\n");
        }
    }

    prompt.push_str(&format!("Question: {}\n\n", question));

    prompt.push_str("Available Context (from documentation and codebase analysis):\n");
    prompt.push_str("================================================================\n\n");
    prompt.push_str(gathered_context);
    prompt.push_str("\n\n");

    prompt.push_str("Provide a conversational answer that:\n");
    prompt.push_str("1. Directly answers the question based on the context above\n");
    prompt.push_str("2. References documentation sections or files where relevant\n");
    prompt.push_str("3. Is concise but informative (typically 2-4 sentences)\n");
    prompt.push_str("4. Only mentions information that appears in the context above\n\n");

    prompt.push_str("Answer (plain text only, no markdown):\n");

    prompt
}

/// Build prompt for answering from codebase metadata alone (file counts, languages, directories)
fn build_codebase_context_prompt(question: &str, codebase_context: &str, conversation_history: Option<&str>) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are answering a developer's question using codebase metadata.\n\n");
    prompt.push_str("IMPORTANT: Provide ONLY the answer text, without any markdown formatting, code fences, or explanatory prefixes.\n\n");

    // Include conversation history if available
    if let Some(history) = conversation_history {
        if !history.is_empty() {
            prompt.push_str(history);
            prompt.push_str("\n");
        }
    }

    prompt.push_str(&format!("Question: {}\n\n", question));

    prompt.push_str("Codebase Metadata:\n");
    prompt.push_str("==================\n\n");
    prompt.push_str(codebase_context);
    prompt.push_str("\n\n");

    prompt.push_str("Provide a conversational answer that:\n");
    prompt.push_str("1. Directly answers the question using the metadata above\n");
    prompt.push_str("2. Uses specific numbers and percentages from the metadata\n");
    prompt.push_str("3. Is concise but informative (typically 1-2 sentences)\n");
    prompt.push_str("4. Only mentions information that appears in the metadata above\n\n");

    prompt.push_str("Answer (plain text only, no markdown):\n");

    prompt
}

/// Generate a concise summary of findings from one page of results
///
/// Extracts key information (file paths, line numbers, code patterns) for hybrid pagination.
/// Optimized for fast/cheap models (GPT-5-mini, Claude Haiku-4.5).
///
/// Returns a 300-500 token summary of key findings from this page.
pub async fn generate_page_summary(
    question: &str,
    page_results: &[FileGroupedResult],
    page_num: usize,
    total_pages: usize,
    provider: &dyn LlmProvider,
) -> Result<String> {
    let prompt = build_summary_prompt(question, page_results, page_num, total_pages);

    log::debug!("Generating summary for page {}/{} ({} chars)", page_num, total_pages, prompt.len());

    let summary = provider.complete(&prompt, false).await?;
    let cleaned = strip_markdown_fences(&summary);

    Ok(cleaned.to_string())
}

/// Build prompt for page summary generation
fn build_summary_prompt(
    question: &str,
    results: &[FileGroupedResult],
    page_num: usize,
    total_pages: usize,
) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "Extract key findings from code search results (page {}/{}).\n\n",
        page_num, total_pages
    ));

    prompt.push_str("IMPORTANT: Provide ONLY a structured summary, no markdown formatting.\n\n");
    prompt.push_str(&format!("Question: {}\n\n", question));

    prompt.push_str("Search Results:\n");
    prompt.push_str("===============\n\n");

    // Format results (similar to build_answer_prompt but more concise)
    for file_group in results {
        prompt.push_str(&format!("File: {}\n", file_group.path));

        for match_result in &file_group.matches {
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
        }

        prompt.push_str("\n");
    }

    prompt.push_str("\nExtract key findings in this format:\n");
    prompt.push_str("• File paths and line numbers where relevant code exists\n");
    prompt.push_str("• Important function/class names mentioned\n");
    prompt.push_str("• Key code patterns or snippets (max 1-2 lines each)\n");
    prompt.push_str("• Notable observations relevant to the question\n\n");

    prompt.push_str("Summary (plain text, bullet points):\n");

    prompt
}

/// Generate final answer from page summaries
///
/// Synthesizes findings from multiple page summaries into a coherent answer.
/// Uses user's premium model for high-quality final response.
pub async fn generate_final_answer_from_summaries(
    question: &str,
    summaries: &[String],
    total_results: usize,
    gathered_context: Option<&str>,
    codebase_context: Option<&str>,
    conversation_history: Option<&str>,
    provider: &dyn LlmProvider,
) -> Result<String> {
    let prompt = build_final_answer_from_summaries_prompt(
        question,
        summaries,
        total_results,
        gathered_context,
        codebase_context,
        conversation_history,
    );

    log::debug!("Generating final answer from {} summaries ({} chars)", summaries.len(), prompt.len());

    let answer = provider.complete(&prompt, false).await?;
    let cleaned = strip_markdown_fences(&answer);

    Ok(cleaned.to_string())
}

/// Build prompt for final answer generation from summaries
fn build_final_answer_from_summaries_prompt(
    question: &str,
    summaries: &[String],
    total_results: usize,
    gathered_context: Option<&str>,
    codebase_context: Option<&str>,
    conversation_history: Option<&str>,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are answering a developer's question based on code search findings.\n\n");
    prompt.push_str("IMPORTANT: Provide ONLY the answer text, without markdown formatting or prefixes.\n\n");

    // Include conversation history
    if let Some(history) = conversation_history {
        if !history.is_empty() {
            prompt.push_str(history);
            prompt.push_str("\n");
        }
    }

    prompt.push_str(&format!("Question: {}\n\n", question));

    // Add gathered context (documentation, codebase structure)
    if let Some(context) = gathered_context {
        if !context.is_empty() {
            prompt.push_str("Additional Context:\n");
            prompt.push_str("==================\n\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }
    }

    // Add codebase metadata
    if let Some(context) = codebase_context {
        if !context.is_empty() {
            prompt.push_str("Codebase Overview:\n");
            prompt.push_str("=================\n\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str(&format!("Found {} total matches across the codebase.\n\n", total_results));

    prompt.push_str("Key Findings (extracted from search results):\n");
    prompt.push_str("===========================================\n\n");

    for (idx, summary) in summaries.iter().enumerate() {
        prompt.push_str(&format!("Page {}:\n{}\n\n", idx + 1, summary));
    }

    prompt.push_str("\nProvide a conversational answer that:\n");
    prompt.push_str("1. Directly answers the question based on the findings\n");
    prompt.push_str("2. References specific files and line numbers where relevant\n");
    prompt.push_str("3. Synthesizes patterns across findings\n");
    prompt.push_str("4. Is concise but informative (typically 2-4 sentences)\n\n");

    prompt.push_str("Answer (plain text only):\n");

    prompt
}

/// Generate answer with smart pagination based on result count
///
/// Three-tier approach:
/// - Tier 1 (0 results): Answer from context only
/// - Tier 2 (1-20 results): Single call with all results (fast path, matches TUI)
/// - Tier 3 (21+ results): Hybrid pagination with summaries (handles TPM limits)
///
/// This ensures identical behavior between TUI and VSCode while handling rate limits.
///
/// Optional progress callback receives ProgressEvent updates during pagination.
pub async fn generate_answer_with_smart_pagination(
    question: &str,
    queries: &[QueryCommand],
    results: &[FileGroupedResult],
    total_count: usize,
    gathered_context: Option<&str>,
    codebase_context: Option<&str>,
    conversation_history: Option<&str>,
    cache: &CacheManager,
    provider_name: &str,
    user_model: Option<&str>,
    api_key: String,
    progress_callback: Option<Box<dyn Fn(super::progress::ProgressEvent) + Send + Sync>>,
) -> Result<String> {
    const PAGE_SIZE: usize = 20;

    log::info!("Smart pagination: {} results, {} queries", total_count, queries.len());

    let user_model_name = user_model.unwrap_or("default");

    // TIER 1: No results (documentation-only)
    if total_count == 0 {
        log::info!("Tier 1: Answering from context (0 results)");
        let provider = super::providers::create_provider(
            provider_name,
            api_key,
            user_model.map(|s| s.to_string()),
        )?;

        return generate_answer(
            question,
            &[],
            0,
            gathered_context,
            codebase_context,
            conversation_history,
            &*provider,
        ).await;
    }

    // TIER 2: Small results (fast path - matches TUI behavior)
    if total_count <= PAGE_SIZE {
        log::info!("Tier 2: Single call ({} results)", total_count);
        let provider = super::providers::create_provider(
            provider_name,
            api_key,
            user_model.map(|s| s.to_string()),
        )?;

        return generate_answer(
            question,
            results,
            total_count,
            gathered_context,
            codebase_context,
            conversation_history,
            &*provider,
        ).await;
    }

    // TIER 3: Large results (hybrid pagination with summaries)
    let total_pages = (total_count + PAGE_SIZE - 1) / PAGE_SIZE;
    log::info!("Tier 3: Hybrid pagination ({} results, {} pages)", total_count, total_pages);

    // Send initial pagination progress event
    if let Some(ref cb) = progress_callback {
        cb(super::progress::ProgressEvent::ProcessingPage {
            current: 1,
            total: total_pages,
        });
    }

    // Determine summary model
    let summary_model = super::providers::get_summary_model(provider_name, user_model_name);

    if let Some(ref sm) = summary_model {
        log::info!("Using {} for summaries, {} for final answer", sm, user_model_name);
    } else {
        log::info!("Using {} for both (high TPM)", user_model_name);
    }

    // Create summary provider (cheap/high-TPM model)
    let summary_provider = if let Some(sm) = summary_model {
        super::providers::create_provider(
            provider_name,
            api_key.clone(),
            Some(sm),
        )?
    } else {
        super::providers::create_provider(
            provider_name,
            api_key.clone(),
            user_model.map(|s| s.to_string()),
        )?
    };

    // Wrap provider in Arc for sharing across tasks
    let summary_provider = Arc::new(summary_provider);

    // Wrap progress callback in Arc if it exists
    let progress_callback = progress_callback.map(Arc::new);

    // Collect page offsets to process (with safety limit of 20 pages)
    let max_pages = total_pages.min(20);
    let page_offsets: Vec<usize> = (0..max_pages).map(|i| i * PAGE_SIZE).collect();

    if max_pages < total_pages {
        log::warn!("Limiting to {} pages (max 20), skipping {} pages", max_pages, total_pages - max_pages);
    }

    // Process pages in batches of 5 for parallel summarization
    const BATCH_SIZE: usize = 5;
    let mut all_summaries = Vec::with_capacity(max_pages);

    for batch_start in (0..max_pages).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(max_pages);
        let batch_offsets = &page_offsets[batch_start..batch_end];

        log::info!("Processing batch {}-{} of {} pages", batch_start + 1, batch_end, max_pages);

        // Spawn parallel summary tasks for this batch
        let mut tasks = Vec::new();

        for (_batch_idx, &offset) in batch_offsets.iter().enumerate() {
            let page_num = (offset / PAGE_SIZE) + 1;
            let queries = queries.to_vec();
            let cache = cache.clone();
            let question = question.to_string();
            let provider = Arc::clone(&summary_provider);
            let progress_cb = progress_callback.clone();

            let task = tokio::spawn(async move {
                // Fetch page results
                let (page_results, _, _, _) = super::executor::execute_queries(
                    queries,
                    &cache,
                    Some(PAGE_SIZE),
                    Some(offset),
                ).await?;

                log::debug!("Page {}/{}: {} results", page_num, total_pages, page_results.len());

                // Send progress event for summary generation
                if let Some(ref cb) = progress_cb {
                    cb(super::progress::ProgressEvent::GeneratingSummary {
                        current: page_num,
                        total: total_pages,
                    });
                }

                // Generate summary
                let summary = generate_page_summary(
                    &question,
                    &page_results,
                    page_num,
                    total_pages,
                    &**provider,
                ).await?;

                Ok::<(usize, String), anyhow::Error>((page_num, summary))
            });

            tasks.push(task);
        }

        // Wait for all tasks in this batch to complete
        let batch_results = futures::future::join_all(tasks).await;

        // Collect results and handle errors
        let mut batch_summaries: Vec<(usize, String)> = Vec::new();
        for result in batch_results {
            match result {
                Ok(Ok((page_num, summary))) => batch_summaries.push((page_num, summary)),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(anyhow::anyhow!("Task panicked: {}", e)),
            }
        }

        // Sort by page_num to maintain order
        batch_summaries.sort_by_key(|(page_num, _)| *page_num);

        // Add to all_summaries
        for (_, summary) in batch_summaries {
            all_summaries.push(summary);
        }
    }

    let summaries = all_summaries;

    log::info!("Generated {} summaries, creating final answer", summaries.len());

    // Send progress event for final answer synthesis
    if let Some(ref cb) = progress_callback {
        cb(super::progress::ProgressEvent::SynthesizingAnswer {
            summary_count: summaries.len(),
        });
    }

    // Create final answer provider (user's premium model)
    let final_provider = super::providers::create_provider(
        provider_name,
        api_key,
        user_model.map(|s| s.to_string()),
    )?;

    generate_final_answer_from_summaries(
        question,
        &summaries,
        total_count,
        gathered_context,
        codebase_context,
        conversation_history,
        &*final_provider,
    ).await
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
        let prompt = build_answer_prompt("Find TODOs", &results, 0, None);

        assert!(prompt.contains("Found 0 total matches"));
        assert!(prompt.contains("Question: Find TODOs"));
    }
}
