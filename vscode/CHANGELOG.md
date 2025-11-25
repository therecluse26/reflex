# Changelog

All notable changes to the Reflex Code Search extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-11-24

### Added
- **Code Search Panel**: Fast full-text search across your entire codebase with instant results
- **Symbol-Aware Filtering**: Find symbol definitions (functions, classes, etc.) using `--symbols` flag
- **Advanced Search Filters**:
  - Filter by programming language (Rust, Python, TypeScript, etc.)
  - Filter by file patterns using glob syntax
  - Regex pattern support for complex searches
- **AI-Powered Chat**: Ask natural language questions about your codebase
  - Supports OpenAI, Anthropic (Claude), and Groq providers
  - Streaming responses with real-time updates
  - Persistent chat history across sessions
  - Codebase-aware responses with source references
- **Click-to-Navigate**: Jump directly to any search result location in your editor
- **Automatic Indexing**: Extension automatically indexes your workspace on first activation
- **Re-Index Command**: One-click re-indexing from the search panel
- **Configurable Settings**:
  - Custom path to rfx binary
  - Choose preferred AI provider
  - Control .gitignore prompting behavior
- **Commands**:
  - `Reflex: Re-Index Project` - Rebuild the search index
  - `Reflex: Configure AI Provider` - Set up AI credentials
  - `Reflex: Clear Chat History` - Reset conversation history
  - `Reflex: Open File` - Internal command for result navigation

### Technical Details
- Built on Reflex's trigram-based search engine for sub-100ms queries
- Local-first architecture - all data stays on your machine
- React-based webview UI with Tailwind CSS
- Automatic server management for AI chat features
- TypeScript extension with comprehensive error handling

### Requirements
- VS Code 1.85.0 or later
- Reflex CLI (`rfx`) must be installed and available in PATH
- For AI chat: API key for your preferred provider (OpenAI, Anthropic, or Groq)

### Known Limitations
- Initial release - some features may need refinement
- Chat feature requires manual API key configuration
- Symbol extraction available for 18 languages (more coming soon)

---

## Future Releases

### Planned Features
- Dependency graph visualization
- Multi-workspace support
- Advanced AST-based queries from UI
- Code lens integration for quick searches
- Saved search queries and filters
- Export search results
- Performance metrics and indexing status display
