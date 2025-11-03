//! MCP (Model Context Protocol) server implementation
//!
//! This module implements the MCP protocol directly over stdio using JSON-RPC 2.0.
//! It exposes RefLex's code search capabilities as MCP tools for AI coding assistants.

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
                "name": "search_code",
                "description": "Search code with full-text or symbol search. Returns results with file paths, line numbers, and context.",
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
                            "description": "Maximum number of results"
                        },
                        "expand": {
                            "type": "boolean",
                            "description": "Show full symbol body (not just signature)"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "search_regex",
                "description": "Search code using regex patterns with trigram optimization.",
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
                            "description": "Maximum number of results"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "search_ast",
                "description": "Structure-aware code search using Tree-sitter AST patterns (S-expressions).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Text pattern for trigram filtering"
                        },
                        "ast_pattern": {
                            "type": "string",
                            "description": "AST pattern (Tree-sitter S-expression)"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Language (required: rust, typescript, javascript, php)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter by file path"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results"
                        }
                    },
                    "required": ["pattern", "ast_pattern", "lang"]
                }
            },
            {
                "name": "index_project",
                "description": "Trigger reindexing of the project. Supports incremental or full rebuild.",
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

            let language = parse_language(lang);
            let parsed_kind = parse_symbol_kind(kind);
            let symbols_mode = symbols.unwrap_or(false) || parsed_kind.is_some();

            let filter = QueryFilter {
                language,
                kind: parsed_kind,
                use_ast: false,
                use_regex: false,
                limit,
                symbols_mode,
                expand: expand.unwrap_or(false),
                file_pattern: file,
                exact: exact.unwrap_or(false),
                timeout_secs: 30, // Default 30 second timeout for MCP queries
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let response = engine.search_with_metadata(&pattern, filter)?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&response)?
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

            let language = parse_language(lang);

            let filter = QueryFilter {
                language,
                kind: None,
                use_ast: false,
                use_regex: true,
                limit,
                symbols_mode: false,
                expand: false,
                file_pattern: file,
                exact: false,
                timeout_secs: 30, // Default 30 second timeout for MCP queries
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let response = engine.search_with_metadata(&pattern, filter)?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&response)?
                }]
            }))
        }
        "search_ast" => {
            let pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?
                .to_string();

            let ast_pattern = arguments["ast_pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing ast_pattern"))?
                .to_string();

            let lang_str = arguments["lang"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing lang"))?
                .to_string();

            let file = arguments["file"].as_str().map(|s| s.to_string());
            let limit = arguments["limit"].as_u64().map(|n| n as usize);

            let language = parse_language(Some(lang_str))
                .ok_or_else(|| anyhow::anyhow!("Invalid or unsupported language"))?;

            let filter = QueryFilter {
                language: Some(language),
                kind: None,
                use_ast: true,
                use_regex: false,
                limit,
                symbols_mode: false,
                expand: false,
                file_pattern: file,
                exact: false,
                timeout_secs: 30, // Default 30 second timeout for MCP queries
            };

            let cache = CacheManager::new(".");
            let engine = QueryEngine::new(cache);
            let results = engine.search_ast_with_text_filter(&pattern, &ast_pattern, filter)?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&results)?
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
                    "text": serde_json::to_string_pretty(&stats)?
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
    log::info!("Starting RefLex MCP server on stdio");

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

    log::info!("RefLex MCP server stopped");
    Ok(())
}
