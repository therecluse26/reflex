//! MCP (Model Context Protocol) server implementation
//!
//! This module implements the MCP protocol directly over stdio using JSON-RPC 2.0.
//! It exposes Reflex's code search capabilities as MCP tools for AI coding assistants.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::cache::CacheManager;
use crate::indexer::Indexer;
use crate::models::{IndexConfig, Language, SymbolKind};
use crate::query::{QueryEngine, QueryFilter};

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Parse language string to Language enum
fn parse_language(lang: Option<String>) -> Option<Language> {
    lang.as_deref().and_then(|s| match s.to_lowercase().as_str() {
        "rust" | "rs" => Some(Language::Rust),
        "javascript" | "js" => Some(Language::JavaScript),
        "typescript" | "ts" => Some(Language::TypeScript),
        "vue" => Some(Language::Vue),
        "svelte" => Some(Language::Svelte),
        "php" => Some(Language::PHP),
        "python" | "py" => Some(Language::Python),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "c" => Some(Language::C),
        "cpp" | "c++" => Some(Language::Cpp),
        _ => None,
    })
}

/// Parse symbol kind string to SymbolKind enum
fn parse_symbol_kind(kind: Option<String>) -> Option<SymbolKind> {
    kind.as_deref().and_then(|s| {
        let capitalized = {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.flat_map(|c| c.to_lowercase()))
                    .collect(),
            }
        };

        capitalized
            .parse::<SymbolKind>()
            .ok()
            .or_else(|| Some(SymbolKind::Unknown(s.to_string())))
    })
}

/// Handle initialize request
fn handle_initialize(_params: Option<Value>) -> Result<Value> {
    Ok(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "reflex",
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

/// Handle tools/list request
fn handle_list_tools(_params: Option<Value>) -> Result<Value> {
    Ok(json!({
        "tools": [
            {
                "name": "list_locations",
                "description": "Fast location discovery with minimal token usage.\n\n**Purpose:** Find where a pattern occurs (file + line) without loading previews or detailed context.\n\n**Returns:** Array of {path, line} objects - one per match location.\n\n**Use this when:**\n- Starting exploration (\"where is X used?\")\n- Counting affected locations\n- Building a list for targeted Read operations\n- You need locations only, not code content\n\n**Workflow:**\n1. Use list_locations to discover (cheap, returns locations only)\n2. Use Read tool or search_code on specific files if you need content (targeted)\n\n**Supports:** lang, file, glob, exclude filters\n**No limit:** Returns ALL matching locations\n\n**Example:** Pattern \"CourtCase\" → [{\"path\": \"app/Models/CourtCase.php\", \"line\": 15}, {\"path\": \"app/Http/Controllers/CourtController.php\", \"line\": 42}]",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (text to find)"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Filter by language (php, rust, typescript, python, etc.)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter by file path substring (e.g., 'Controllers')"
                        },
                        "glob": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Include files matching patterns (e.g., ['app/**/*.php'])"
                        },
                        "exclude": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Exclude files matching patterns (e.g., ['vendor/**', 'tests/**'])"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force execution of potentially expensive queries (bypasses broad query detection)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "count_occurrences",
                "description": "Quick statistics - count how many times a pattern occurs.\n\n**Purpose:** Get total occurrence count and file count without loading any content.\n\n**Use this when:**\n- You need quick stats (\"how many times is X used?\")\n- Checking impact before refactoring\n- Validating search scope\n\n**Returns:** {total: count, files: count, pattern: string}\n\n**Supports:** All filters (lang, file, glob, exclude, symbols)\n\n**Example output:** {\"total\": 87, \"files\": 12, \"pattern\": \"CourtCase\"}",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (text to find)"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Filter by language"
                        },
                        "symbols": {
                            "type": "boolean",
                            "description": "Count symbol definitions only (not usages)"
                        },
                        "kind": {
                            "type": "string",
                            "description": "Filter by symbol kind (function, class, etc.)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter by file path substring"
                        },
                        "glob": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Include files matching patterns"
                        },
                        "exclude": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Exclude files matching patterns"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force execution of potentially expensive queries (bypasses broad query detection)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "search_code",
                "description": "Full-text or symbol-only code search with detailed results.\n\n**When to use search_regex instead:**\n- Patterns with special characters: -> :: () [] {} . * + ? \\\\ | ^ $\n- Complex pattern matching: wildcards, alternation, anchors\n- Examples: '->with(', '::new', 'function*', '[derive]', 'fn (get|set)_.*'\n\n**Search modes:**\n- Full-text (default): Finds ALL occurrences - definitions + usages\n- Symbol-only (symbols=true): Finds ONLY definitions where symbols are declared\n\n**Use this for:**\n- Simple text patterns (alphanumeric, underscores, hyphens)\n- Detailed analysis with line numbers and code previews\n- Symbol definition searches\n\n**Pagination:** Check response.pagination.has_more. If true, use offset parameter to fetch next page.\n\n**Note:** If results seem outdated, run index_project first.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (text to find)"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Filter by language (rust, typescript, python, etc.)"
                        },
                        "kind": {
                            "type": "string",
                            "description": "Filter by symbol kind (function, class, struct, etc.)"
                        },
                        "symbols": {
                            "type": "boolean",
                            "description": "Symbol-only search (definitions, not usage)"
                        },
                        "exact": {
                            "type": "boolean",
                            "description": "Exact match (no substring matching)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter by file path (substring)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results per page (default: 100). IMPORTANT: If response.pagination.has_more is true, you MUST fetch more pages using offset parameter."
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Pagination offset (skip first N results). ALWAYS paginate when has_more=true. Example: First call offset=0, second call offset=100, third offset=200, etc."
                        },
                        "expand": {
                            "type": "boolean",
                            "description": "Show full symbol body (not just signature)"
                        },
                        "glob": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Include files matching glob patterns (e.g., 'src/**/*.rs')"
                        },
                        "exclude": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Exclude files matching glob patterns (e.g., 'target/**')"
                        },
                        "paths": {
                            "type": "boolean",
                            "description": "Return only unique file paths (not full results)"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force execution of potentially expensive queries (bypasses broad query detection)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "search_regex",
                "description": "Regex-based code search for complex pattern matching (e.g., 'fn (get|set)_\\\\w+').\n\n**Use for:**\n- Patterns with special characters: -> :: () [] {} . * + ? \\\\ | ^ $\n- Pattern matching: wildcards (.*), alternation (a|b), anchors (^$)\n- Complex searches: case-insensitive variants, word boundaries\n\n**Common examples:**\n- Method calls: '->with\\\\(', '->map\\\\(', '::new\\\\('\n- Operators: '->', '::', '||', '&&'\n- Functions: 'fn (get|set)_\\\\\\\\w+' (getter/setter functions)\n- Attributes: '\\\\\\\\[(derive|test)\\\\\\\\]' (Rust attributes)\n\n**Escaping rules:**\n- Must escape: ( ) [ ] { } . * + ? \\\\ | ^ $\n- No escaping needed: -> :: - _ / = < >\n- Use double backslash in JSON: \\\\\\\\( \\\\\\\\) \\\\\\\\[ \\\\\\\\]\n\n**Don't use for:**\n- Simple text searches (use search_code instead - faster)\n- Symbol definitions (use search_code with symbols=true instead)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Filter by language"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter by file path"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results (use with offset for pagination)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Pagination offset (skip first N results after sorting)"
                        },
                        "glob": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Include files matching glob patterns"
                        },
                        "exclude": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Exclude files matching glob patterns"
                        },
                        "paths": {
                            "type": "boolean",
                            "description": "Return only unique file paths"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force execution of potentially expensive queries (bypasses broad query detection)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "search_ast",
                "description": "⚠️ ADVANCED USERS ONLY - DO NOT USE UNLESS ABSOLUTELY NECESSARY ⚠️\n\nStructure-aware code search using Tree-sitter AST patterns (S-expressions).\n\n**PERFORMANCE WARNING:** AST queries bypass trigram optimization and scan the ENTIRE codebase (500ms-10s+).\n\n**WHEN TO USE (RARE):**\n- You need to match code structure, not just text (e.g., \"all async functions with try/catch blocks\")\n- --symbols search is insufficient (e.g., need to match specific AST node types)\n- You have a very specific structural pattern that cannot be expressed as text\n\n**IN 95% OF CASES, USE search_code with symbols=true INSTEAD** (10-100x faster).\n\n**REQUIRED:** You MUST use glob patterns to limit scope (e.g., glob=['src/**/*.rs']) to avoid scanning thousands of files.\n\n**Token efficiency:** Previews are auto-truncated to ~100 chars. Use limit parameter to control result count.\n\n**Example AST patterns:**\n- Rust: '(function_item) @fn' (all functions)\n- Python: '(function_definition) @fn' (all functions)\n- TypeScript: '(class_declaration) @class' (all classes)\n\nRefer to Tree-sitter documentation for each language's grammar.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "AST pattern (Tree-sitter S-expression, e.g., '(function_item) @fn')"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Language (REQUIRED: rust, typescript, javascript, python, go, java, c, cpp, csharp, php, ruby, kotlin, zig)"
                        },
                        "glob": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Include files matching glob patterns (STRONGLY RECOMMENDED to limit scope, e.g., ['src/**/*.rs'])"
                        },
                        "exclude": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Exclude files matching glob patterns (e.g., ['target/**', 'node_modules/**'])"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter by file path (substring)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results (use with offset for pagination)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Pagination offset (skip first N results after sorting)"
                        },
                        "paths": {
                            "type": "boolean",
                            "description": "Return only unique file paths"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force execution of potentially expensive queries (bypasses broad query detection)"
                        }
                    },
                    "required": ["pattern", "lang"]
                }
            },
            {
                "name": "index_project",
                "description": "Rebuild or update the code search index. Run this when:\n\n- After code changes (user edits, git operations, file creation/deletion)\n- Search results seem stale or missing new files\n- Empty/error results (may indicate missing/corrupt index)\n\n**Modes:**\n- Incremental (default): Only re-indexes changed files (fast)\n- Full rebuild (force=true): Re-indexes everything (use if index seems corrupted)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "force": {
                            "type": "boolean",
                            "description": "Force full rebuild (ignore incremental)"
                        },
                        "languages": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Languages to include (empty = all)"
                        }
                    }
                }
            }
        ]
    }))
}

/// Handle tools/call request
fn handle_call_tool(params: Option<Value>) -> Result<Value> {
    let params = params.ok_or_else(|| anyhow::anyhow!("Missing params for tools/call"))?;

    let name = params["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

    let arguments = params["arguments"].clone();

    match name {
        "list_locations" => {
            // Location discovery tool (minimal token usage)
            let pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?
                .to_string();

            let lang = arguments["lang"].as_str().map(|s| s.to_string());
            let file = arguments["file"].as_str().map(|s| s.to_string());
            let glob_patterns = arguments["glob"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let exclude_patterns = arguments["exclude"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let force = arguments["force"].as_bool().unwrap_or(false);

            let language = parse_language(lang);

            let filter = QueryFilter {
                language,
                kind: None,
                use_ast: false,
                use_regex: false,
                limit: None,  // No limit for paths-only mode
                symbols_mode: false,
                expand: false,
                file_pattern: file,
                exact: false,
                use_contains: false,
                timeout_secs: 30,
                glob_patterns,
                exclude_patterns,
                paths_only: true,  // KEY: Enable paths-only mode
                offset: None,
                force,
                suppress_output: true,  // MCP always returns JSON
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let response = engine.search_with_metadata(&pattern, filter)?;

            // Extract locations (path + line) for each match
            let locations: Vec<serde_json::Value> = response.results.iter()
                .map(|r| json!({
                    "path": r.path,
                    "line": r.span.start_line
                }))
                .collect();

            // Return compact response (just locations + count)
            let compact_response = json!({
                "status": response.status,
                "total_locations": locations.len(),
                "locations": locations
            });

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&compact_response)?
                }]
            }))
        }
        "count_occurrences" => {
            // Quick stats tool (minimal token usage)
            let pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?
                .to_string();

            let lang = arguments["lang"].as_str().map(|s| s.to_string());
            let kind = arguments["kind"].as_str().map(|s| s.to_string());
            let symbols = arguments["symbols"].as_bool();
            let file = arguments["file"].as_str().map(|s| s.to_string());
            let glob_patterns = arguments["glob"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let exclude_patterns = arguments["exclude"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let force = arguments["force"].as_bool().unwrap_or(false);

            let language = parse_language(lang);
            let parsed_kind = parse_symbol_kind(kind);
            let symbols_mode = symbols.unwrap_or(false) || parsed_kind.is_some();

            let filter = QueryFilter {
                language,
                kind: parsed_kind,
                use_ast: false,
                use_regex: false,
                limit: None,  // No limit for counting
                symbols_mode,
                expand: false,
                file_pattern: file,
                exact: false,
                use_contains: false,
                timeout_secs: 30,
                glob_patterns,
                exclude_patterns,
                paths_only: false,  // Need to count all occurrences
                offset: None,
                force,
                suppress_output: true,  // MCP always returns JSON
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let response = engine.search_with_metadata(&pattern, filter)?;

            // Count unique files
            use std::collections::HashSet;
            let unique_files: HashSet<String> = response.results.iter()
                .map(|r| r.path.clone())
                .collect();

            // Return minimal stats
            let stats = json!({
                "status": response.status,
                "pattern": pattern,
                "total": response.pagination.total,
                "files": unique_files.len()
            });

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&stats)?
                }]
            }))
        }
        "search_code" => {
            let pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?
                .to_string();

            let lang = arguments["lang"].as_str().map(|s| s.to_string());
            let kind = arguments["kind"].as_str().map(|s| s.to_string());
            let symbols = arguments["symbols"].as_bool();
            let exact = arguments["exact"].as_bool();
            let file = arguments["file"].as_str().map(|s| s.to_string());
            let limit = arguments["limit"].as_u64().map(|n| n as usize);
            let expand = arguments["expand"].as_bool();
            let glob_patterns: Vec<String> = arguments["glob"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let exclude_patterns = arguments["exclude"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let paths_only = arguments["paths"].as_bool().unwrap_or(false);
            let force = arguments["force"].as_bool().unwrap_or(false);

            let language = parse_language(lang);
            let parsed_kind = parse_symbol_kind(kind);
            let symbols_mode = symbols.unwrap_or(false) || parsed_kind.is_some();

            let offset = arguments["offset"].as_u64().map(|n| n as usize);

            // Smart limit handling:
            // 1. If --paths is set and user didn't specify limit: no limit (None)
            // 2. If user specified limit: use that value
            // 3. Otherwise: use default limit of 100
            let final_limit = if paths_only && limit.is_none() {
                None  // --paths without explicit limit means no limit
            } else if let Some(user_limit) = limit {
                Some(user_limit)  // Use user-specified limit
            } else {
                Some(100)  // Default: limit to 100 results for token efficiency
            };

            let filter = QueryFilter {
                language,
                kind: parsed_kind,
                use_ast: false,
                use_regex: false,
                limit: final_limit,
                symbols_mode,
                expand: expand.unwrap_or(false),
                file_pattern: file,
                exact: exact.unwrap_or(false),
                use_contains: false, // Default to word-boundary matching for MCP
                timeout_secs: 30, // Default 30 second timeout for MCP queries
                glob_patterns: glob_patterns.clone(),
                exclude_patterns,
                paths_only,
                offset,
                force,
                suppress_output: true,  // MCP always returns JSON
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let mut response = engine.search_with_metadata(&pattern, filter)?;

            // Apply preview truncation for token efficiency (100 chars max)
            const MAX_PREVIEW_LENGTH: usize = 100;
            for result in &mut response.results {
                result.preview = crate::cli::truncate_preview(&result.preview, MAX_PREVIEW_LENGTH);
            }

            // Generate AI instruction (MCP always uses AI mode)
            response.ai_instruction = crate::query::generate_ai_instruction(
                response.results.len(),
                response.pagination.total,
                response.pagination.has_more,
                symbols_mode,
                paths_only,
                false,  // use_ast
                false,  // use_regex
                language.is_some(),
                !glob_patterns.is_empty(),
                exact.unwrap_or(false),
            );

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&response)?
                }]
            }))
        }
        "search_regex" => {
            let pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?
                .to_string();

            let lang = arguments["lang"].as_str().map(|s| s.to_string());
            let file = arguments["file"].as_str().map(|s| s.to_string());
            let limit = arguments["limit"].as_u64().map(|n| n as usize);
            let glob_patterns: Vec<String> = arguments["glob"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let exclude_patterns = arguments["exclude"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let paths_only = arguments["paths"].as_bool().unwrap_or(false);
            let force = arguments["force"].as_bool().unwrap_or(false);

            let language = parse_language(lang);
            let offset = arguments["offset"].as_u64().map(|n| n as usize);

            // Smart limit handling (same as search_code)
            let final_limit = if paths_only && limit.is_none() {
                None  // --paths without explicit limit means no limit
            } else if let Some(user_limit) = limit {
                Some(user_limit)  // Use user-specified limit
            } else {
                Some(100)  // Default: limit to 100 results for token efficiency
            };

            let filter = QueryFilter {
                language,
                kind: None,
                use_ast: false,
                use_regex: true,
                limit: final_limit,
                symbols_mode: false,
                expand: false,
                file_pattern: file,
                exact: false,
                use_contains: false, // Regex mode uses substring matching via use_regex flag
                timeout_secs: 30, // Default 30 second timeout for MCP queries
                glob_patterns: glob_patterns.clone(),
                exclude_patterns,
                paths_only,
                offset,
                force,
                suppress_output: true,  // MCP always returns JSON
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let mut response = engine.search_with_metadata(&pattern, filter)?;

            // Apply preview truncation for token efficiency (100 chars max)
            const MAX_PREVIEW_LENGTH: usize = 100;
            for result in &mut response.results {
                result.preview = crate::cli::truncate_preview(&result.preview, MAX_PREVIEW_LENGTH);
            }

            // Generate AI instruction (MCP always uses AI mode)
            response.ai_instruction = crate::query::generate_ai_instruction(
                response.results.len(),
                response.pagination.total,
                response.pagination.has_more,
                false,  // symbols_mode
                paths_only,
                false,  // use_ast
                true,   // use_regex
                language.is_some(),
                !glob_patterns.is_empty(),
                false,  // exact
            );

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&response)?
                }]
            }))
        }
        "search_ast" => {
            // AST pattern (Tree-sitter S-expression)
            let ast_pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pattern (AST S-expression)"))?
                .to_string();

            let lang_str = arguments["lang"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing lang (required for AST queries)"))?
                .to_string();

            let file = arguments["file"].as_str().map(|s| s.to_string());
            let limit = arguments["limit"].as_u64().map(|n| n as usize);
            let glob_patterns: Vec<String> = arguments["glob"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let exclude_patterns: Vec<String> = arguments["exclude"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let paths_only = arguments["paths"].as_bool().unwrap_or(false);
            let force = arguments["force"].as_bool().unwrap_or(false);

            let language = parse_language(Some(lang_str))
                .ok_or_else(|| anyhow::anyhow!("Invalid or unsupported language for AST queries"))?;

            // Warn if glob patterns are not provided (performance issue)
            if glob_patterns.is_empty() && exclude_patterns.is_empty() {
                log::warn!("⚠️  AST query without glob patterns will scan the ENTIRE codebase. This may take 2-10+ seconds.");
                log::warn!("    Strongly recommend using glob patterns, e.g., glob=['src/**/*.rs']");
            }

            let offset = arguments["offset"].as_u64().map(|n| n as usize);

            // Smart limit handling (same as search_code)
            let final_limit = if paths_only && limit.is_none() {
                None  // --paths without explicit limit means no limit
            } else if let Some(user_limit) = limit {
                Some(user_limit)  // Use user-specified limit
            } else {
                Some(100)  // Default: limit to 100 results for token efficiency
            };

            let filter = QueryFilter {
                language: Some(language),
                kind: None,
                use_ast: true,
                use_regex: false,
                limit: final_limit,
                symbols_mode: false,
                expand: false,
                file_pattern: file,
                exact: false,
                use_contains: false,
                timeout_secs: 60, // Longer timeout for AST queries (they're slow)
                glob_patterns,
                exclude_patterns,
                paths_only,
                offset,
                force,
                suppress_output: true,  // MCP always returns JSON
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);

            // Use the new search_ast_all_files method (no trigram filtering)
            let mut results = engine.search_ast_all_files(&ast_pattern, filter)?;

            // Apply preview truncation for token efficiency (100 chars max)
            const MAX_PREVIEW_LENGTH: usize = 100;
            for result in &mut results {
                result.preview = crate::cli::truncate_preview(&result.preview, MAX_PREVIEW_LENGTH);
            }

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&results)?
                }]
            }))
        }
        "index_project" => {
            let force = arguments["force"].as_bool();
            let languages = arguments["languages"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                });

            let cache = CacheManager::new(".");

            if force.unwrap_or(false) {
                log::info!("Force rebuild requested, clearing existing cache");
                cache.clear()?;
            }

            let lang_filters: Vec<Language> = languages
                .unwrap_or_default()
                .iter()
                .filter_map(|s| parse_language(Some(s.clone())))
                .collect();

            let config = IndexConfig {
                languages: lang_filters,
                ..Default::default()
            };

            let indexer = Indexer::new(cache, config);
            let path = PathBuf::from(".");
            let stats = indexer.index(&path, false)?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&stats)?
                }]
            }))
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}

/// Process a single JSON-RPC request
fn process_request(request: JsonRpcRequest) -> JsonRpcResponse {
    log::debug!("MCP request: method={}", request.method);

    let result = match request.method.as_str() {
        "initialize" => handle_initialize(request.params),
        "tools/list" => handle_list_tools(request.params),
        "tools/call" => handle_call_tool(request.params),
        _ => Err(anyhow::anyhow!("Unknown method: {}", request.method)),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(e) => {
            log::error!("MCP error: {}", e);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
    }
}

/// Run the MCP server on stdio
pub fn run_mcp_server() -> Result<()> {
    log::info!("Starting Reflex MCP server on stdio");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        log::debug!("MCP input: {}", line);

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                log::error!("Failed to parse JSON-RPC request: {}", e);
                continue;
            }
        };

        // Process request
        let response = process_request(request);

        // Send response
        let response_json = serde_json::to_string(&response)?;
        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;

        log::debug!("MCP output: {}", response_json);
    }

    log::info!("Reflex MCP server stopped");
    Ok(())
}
