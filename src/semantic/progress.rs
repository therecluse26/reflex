//! Progress event types for HTTP SSE streaming
//!
//! These types mirror the PhaseUpdate enum from chat_tui.rs but are designed
//! for JSON serialization over Server-Sent Events.

use serde::{Deserialize, Serialize};

/// Progress event sent via SSE to VSCode extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressEvent {
    /// Phase 0: Triage - deciding whether to search or answer directly
    Triaging,

    /// Fast path: Answering from conversation context
    AnsweringFromContext,

    /// Phase 1: Thinking/Assessment (agentic path)
    Thinking {
        reasoning: String,
        needs_context: bool,
    },

    /// Phase 2: Tool gathering (agentic path)
    Tools {
        content: String,
        tool_calls: Vec<String>,
    },

    /// Phase 3: Query generation (agentic path)
    Queries {
        queries: Vec<String>,
    },

    /// Phase 4: Execution status (agentic path)
    Executing {
        results_count: usize,
        execution_time_ms: u64,
    },

    /// Processing paginated results (smart pagination)
    ProcessingPage {
        current: usize,
        total: usize,
    },

    /// Generating summary for a page (smart pagination)
    GeneratingSummary {
        current: usize,
        total: usize,
    },

    /// Synthesizing final answer from summaries (smart pagination)
    SynthesizingAnswer {
        summary_count: usize,
    },

    /// Reindexing cache (schema mismatch detected)
    Reindexing {
        current: usize,
        total: usize,
        message: String,
    },

    /// Phase 5: Final answer (both paths)
    Answer {
        answer: String,
    },

    /// Error occurred
    Error {
        error: String,
    },

    /// Processing complete
    Done,
}

impl ProgressEvent {
    /// Convert to SSE event data (JSON string)
    pub fn to_sse_data(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"type":"error","error":"Failed to serialize progress event"}"#.to_string()
        })
    }

    /// Get a human-readable status message for this event
    pub fn status_message(&self) -> String {
        match self {
            ProgressEvent::Triaging => "Analyzing question...".to_string(),
            ProgressEvent::AnsweringFromContext => "Answering from conversation...".to_string(),
            ProgressEvent::Thinking { reasoning, .. } => {
                format!("Thinking... {}", reasoning)
            }
            ProgressEvent::Tools { tool_calls, .. } => {
                format!("Gathering context ({} tools)...", tool_calls.len())
            }
            ProgressEvent::Queries { queries } => {
                format!("Generated {} queries...", queries.len())
            }
            ProgressEvent::Executing { results_count, .. } => {
                format!("Found {} results...", results_count)
            }
            ProgressEvent::ProcessingPage { current, total } => {
                format!("Processing page {}/{}...", current, total)
            }
            ProgressEvent::GeneratingSummary { current, total } => {
                format!("Generating summary for page {}/{}...", current, total)
            }
            ProgressEvent::SynthesizingAnswer { summary_count } => {
                format!("Synthesizing final answer from {} summaries...", summary_count)
            }
            ProgressEvent::Reindexing { current, total, .. } => {
                let percentage = if *total > 0 {
                    (*current as f32 / *total as f32 * 100.0) as u8
                } else {
                    0
                };
                format!("Reindexing cache: {}/{}  ({}%)", current, total, percentage)
            }
            ProgressEvent::Answer { .. } => "Generating answer...".to_string(),
            ProgressEvent::Error { error } => format!("Error: {}", error),
            ProgressEvent::Done => "Complete".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_triaging() {
        let event = ProgressEvent::Triaging;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, r#"{"type":"triaging"}"#);
    }

    #[test]
    fn test_serialize_thinking() {
        let event = ProgressEvent::Thinking {
            reasoning: "Need to search for trigram implementation".to_string(),
            needs_context: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"thinking""#));
        assert!(json.contains(r#""reasoning":"Need to search for trigram implementation""#));
        assert!(json.contains(r#""needs_context":true"#));
    }

    #[test]
    fn test_serialize_queries() {
        let event = ProgressEvent::Queries {
            queries: vec!["query trigram".to_string(), "query index".to_string()],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"queries""#));
        assert!(json.contains(r#""queries":["query trigram","query index"]"#));
    }

    #[test]
    fn test_status_message() {
        let event = ProgressEvent::Executing {
            results_count: 247,
            execution_time_ms: 150,
        };
        assert_eq!(event.status_message(), "Found 247 results...");
    }
}
