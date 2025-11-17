//! Prompt template generation with language injection

use crate::cache::CacheManager;
use anyhow::{Context, Result};
use rusqlite::Connection;

/// Embed SEMANTIC_QUERY_PROMPT.md at compile time
const PROMPT_TEMPLATE: &str = include_str!("../../.context/SEMANTIC_QUERY_PROMPT.md");

/// Build the complete prompt for the LLM
///
/// Injects detected languages from the cache and adds JSON schema instructions
pub fn build_prompt(question: &str, cache: &CacheManager) -> Result<String> {
    // Detect languages from cache
    let languages = detect_languages(cache)?;
    let lang_list = if languages.is_empty() {
        "No languages detected (empty codebase)".to_string()
    } else {
        languages.join(", ")
    };

    // Inject languages into template
    let prompt = PROMPT_TEMPLATE.replace("{DETECTED_LANGUAGES}", &lang_list);

    // Build final prompt with JSON schema
    Ok(format!(
        r#"{prompt}

## Response Format

You MUST respond with valid JSON matching this exact schema:

```json
{schema}
```

## User Question

{question}

**IMPORTANT:** Return ONLY valid JSON. No markdown code blocks, no explanations outside the JSON structure. Just pure JSON.
"#,
        prompt = prompt,
        schema = crate::semantic::schema::RESPONSE_SCHEMA,
        question = question
    ))
}

/// Detect languages present in the indexed codebase
fn detect_languages(cache: &CacheManager) -> Result<Vec<String>> {
    let db_path = cache.path().join("meta.db");
    let conn = Connection::open(&db_path)
        .context("Failed to open database")?;

    // Query for distinct language extensions
    let mut stmt = conn.prepare(
        "SELECT DISTINCT language FROM files
         WHERE language IS NOT NULL
         ORDER BY language"
    )?;

    let languages: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(languages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_prompt_contains_schema() {
        let temp_dir = TempDir::new().unwrap();
        let cache = CacheManager::create(temp_dir.path()).unwrap();

        let prompt = build_prompt("find todos", &cache).unwrap();

        assert!(prompt.contains("RESPONSE_SCHEMA"));
        assert!(prompt.contains("find todos"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_prompt_injects_empty_languages() {
        let temp_dir = TempDir::new().unwrap();
        let cache = CacheManager::create(temp_dir.path()).unwrap();

        let prompt = build_prompt("test", &cache).unwrap();

        // Should handle empty codebase gracefully
        assert!(prompt.contains("No languages detected") || prompt.contains("Languages in this codebase"));
    }
}
