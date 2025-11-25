# Reflex Code Search for VS Code

Fast, local-first code search powered by [Reflex](https://github.com/reflex-search/reflex).

## Features

### Code Search
- **Fast full-text search** across your entire codebase
- **Symbol-aware filtering** to find definitions, not just usage
- **Advanced filters** for language, file patterns, and more
- **Click-to-navigate** to any search result

### Re-Indexing
- One-click re-indexing from the search panel
- Status notifications for indexing progress

### AI-Powered Chat
- Ask questions about your codebase using natural language
- Powered by your choice of AI provider (OpenAI, Anthropic, Groq)
- Streaming responses with real-time progress updates
- Persistent conversation history
- Codebase-aware responses with automatic search integration

## Requirements

- **Reflex CLI** must be installed and available in your PATH
  - Install from: [Reflex Releases](https://github.com/reflex-search/reflex/releases)
  - Or build from source: `cargo install --path .`

## Installation

1. Install the Reflex CLI (see Requirements above)
2. Install this extension from the VS Code Marketplace
3. Open a workspace/folder in VS Code
4. The extension will activate automatically

## Usage

### Search Panel

1. Click the Reflex icon in the Activity Bar (sidebar)
2. Enter your search query in the search box
3. Use filters to refine results:
   - **Language**: Filter by programming language (rust, python, etc.)
   - **Glob**: Filter by file patterns (src/**/*.rs)
   - **Symbols Only**: Find only symbol definitions
4. Click any result to jump to that file and line

### Re-Indexing

Click the "Re-Index" button in the search panel to rebuild the Reflex index for your workspace.

### Commands

- `Reflex: Re-Index Project` - Rebuild the search index for your workspace
- `Reflex: Configure AI Provider` - Set up your AI provider and API key for chat
- `Reflex: Clear Chat History` - Reset your conversation history
- `Reflex: Hello World` - Test command to verify extension is working

## Configuration

### Extension Settings

- **Reflex: Binary Path** - Custom path to rfx binary (leave empty to use PATH)
- **Reflex: AI Provider** - Choose between OpenAI, Anthropic, or Groq for chat
- **Reflex: Prompt Gitignore** - Automatically prompt to add `.reflex/` to `.gitignore`

### AI Chat Setup

To use the AI-powered chat feature:

1. Run the command `Reflex: Configure AI Provider` from the Command Palette
2. Choose your preferred provider (OpenAI, Anthropic, or Groq)
3. Enter your API key when prompted
4. Start chatting in the search panel's Chat tab

Alternatively, you can manually configure via `~/.reflex/config.toml`.

## Known Issues

- This is an early preview release
- API keys must be configured manually via command palette or config file
- Chat history is stored in VS Code workspace state (not synced across machines)
- Some advanced search features (AST queries, dependency analysis) not yet exposed in UI

## Release Notes

### 0.1.0

Initial preview release:
- Basic extension scaffolding
- Hello World command for testing
- Foundation for search panel and chat features

## Contributing

See [DEVELOPMENT.md](./DEVELOPMENT.md) for development setup and contribution guidelines.

## License

This project is part of the Reflex monorepo. See the root LICENSE file for details.
