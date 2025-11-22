# Reflex Code Search for VS Code

Fast, local-first code search powered by [Reflex](https://github.com/your-org/reflex).

## Features

### Code Search
- **Fast full-text search** across your entire codebase
- **Symbol-aware filtering** to find definitions, not just usage
- **Advanced filters** for language, file patterns, and more
- **Click-to-navigate** to any search result

### Re-Indexing
- One-click re-indexing from the search panel
- Status notifications for indexing progress

### AI-Powered Chat (Coming Soon)
- Ask questions about your codebase using natural language
- Powered by your choice of AI provider (OpenAI, Anthropic, Groq)

## Requirements

- **Reflex CLI** must be installed and available in your PATH
  - Install from: [Reflex Releases](https://github.com/your-org/reflex/releases)
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

- `Reflex: Hello World` - Test command to verify extension is working

## Configuration

Currently, configuration is managed through `~/.reflex/config.toml`. VS Code-specific settings will be added in a future release.

## Known Issues

- This is an early preview release
- Search panel UI is in active development
- Chat participant feature is not yet implemented

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
