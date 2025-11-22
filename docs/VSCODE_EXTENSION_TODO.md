# VS Code Extension Implementation TODO

**Status**: In Progress (Phase 2 Complete)
**Created**: 2025-01-21
**Target**: Monorepo integration in `vscode/` directory

---

## Overview

Build a VS Code extension for Reflex that provides:
1. Full `rfx query` support via sidebar search panel
2. Re-index capability from the panel
3. Full `rfx ask` support via Chat Participant API

**Repository Structure**: Monorepo approach with extension in `vscode/` directory

---

## Goals

- [ ] Provide seamless code search experience matching built-in VS Code search
- [ ] Integrate AI-powered chat using Chat Participant API
- [ ] Maintain config consistency with `~/.reflex/config.toml`
- [ ] Secure API key storage using VS Code SecretStorage API
- [ ] Professional UX with React-based webviews
- [ ] Automated CI/CD for testing and packaging

---

## Phase 1: Project Scaffolding

### Setup & Configuration
- [ ] Create `vscode/` directory structure
- [ ] Initialize Node.js project (`package.json`)
  - [ ] Add required dependencies (@types/vscode, @types/node, typescript, etc.)
  - [ ] Add dev dependencies (eslint, prettier, vsce, etc.)
- [ ] Create TypeScript configuration (`tsconfig.json`)
- [ ] Set up extension manifest (`package.json` - extension specific)
  - [ ] Define activation events
  - [ ] Configure publisher info
  - [ ] Set engine compatibility (VS Code version)
- [ ] Create `.vscode/launch.json` for debugging
- [ ] Create `.vscodeignore` for packaging

### Source Structure
- [ ] Create `vscode/src/` directory
- [ ] Create `vscode/src/extension.ts` (entry point)
- [ ] Create `vscode/assets/` for icons
  - [ ] Design/add Reflex icon (SVG format)
  - [ ] Add light/dark theme variants if needed

### Webview UI Setup
- [ ] Create `vscode/webview-ui/` directory
- [ ] Initialize React + Vite project
- [ ] Configure Vite for VS Code webview bundling
- [ ] Set up Tailwind CSS (optional but recommended)
- [ ] Create basic component structure

### Documentation
- [ ] Create `vscode/README.md` (user-facing)
- [ ] Create `vscode/DEVELOPMENT.md` (contributor guide)
- [ ] Update root `README.md` to mention VS Code extension

### Validation
- [ ] Test basic "Hello World" extension activation
- [ ] Verify extension loads in Extension Development Host
- [ ] Test debug configuration works

---

## Phase 2: Feature 1 - Re-Index Command (Warmup)

**Why first**: Simplest feature to validate CLI integration

### Implementation
- [ ] Create `vscode/src/reflexClient.ts`
  - [ ] Implement `executeRfx()` base function using `child_process.spawn()`
  - [ ] Add binary path detection (`which rfx` or configurable path)
  - [ ] Add error handling for missing binary
  - [ ] Add timeout protection
- [ ] Create `vscode/src/commands/reindex.ts`
  - [ ] Register `reflex.reindex` command
  - [ ] Show status bar item during indexing
  - [ ] Stream progress updates (if available)
  - [ ] Show success/error notifications
- [ ] Update `package.json` to register command
- [ ] Add command to Command Palette

### Testing
- [ ] Test with valid workspace
- [ ] Test with missing `rfx` binary
- [ ] Test with timeout scenarios
- [ ] Test cancellation (if implemented)

---

## Phase 3: Feature 2 - Search Panel UI

### Package.json Configuration
- [ ] Define `viewsContainers` contribution for Activity Bar icon
- [ ] Define `views` contribution for search panel
- [ ] Register view-related commands

### WebviewViewProvider
- [ ] Create `vscode/src/providers/searchViewProvider.ts`
- [ ] Implement `WebviewViewProvider` interface
- [ ] Implement `resolveWebviewView()` method
- [ ] Set up webview options (enableScripts, CSP, etc.)
- [ ] Register provider in `extension.ts`

### React Search UI
- [ ] Create `vscode/webview-ui/src/SearchView.tsx`
  - [ ] Search input field with debouncing
  - [ ] Filter controls:
    - [ ] Language filter dropdown
    - [ ] Glob pattern input
    - [ ] Symbols-only toggle
    - [ ] Case-insensitive toggle
  - [ ] Re-index button
  - [ ] Results container
- [ ] Create `SearchResult.tsx` component
  - [ ] Display file path (clickable)
  - [ ] Display line number
  - [ ] Display code preview with syntax highlighting
  - [ ] Display context (before/after lines)
- [ ] Create `EmptyState.tsx` component
- [ ] Create `ErrorState.tsx` component
- [ ] Create `LoadingState.tsx` component

### Communication Layer
- [ ] Implement extension → webview messaging
  - [ ] Send search results
  - [ ] Send error messages
  - [ ] Send loading states
- [ ] Implement webview → extension messaging
  - [ ] Receive search queries
  - [ ] Receive filter changes
  - [ ] Receive navigation requests (open file)

### Search Execution
- [ ] Update `reflexClient.ts` with `query()` method
  - [ ] Build args from search options
  - [ ] Add `--json` flag
  - [ ] Parse JSON output
  - [ ] Stream results as they arrive (optional optimization)
- [ ] Handle navigation to file:line in editor
  - [ ] Use `vscode.workspace.openTextDocument()`
  - [ ] Use `vscode.window.showTextDocument()`
  - [ ] Reveal specific line with highlighting

### Performance Optimizations
- [ ] Debounce search input (300ms)
- [ ] Virtualize large result lists (react-window or similar)
- [ ] Cache recent searches (optional)
- [ ] Cancel previous search when new one starts

### Testing
- [ ] Test basic text search
- [ ] Test with filters (language, glob, symbols)
- [ ] Test result navigation
- [ ] Test empty results
- [ ] Test error scenarios
- [ ] Test large result sets (1000+ matches)
- [ ] Test special characters in queries
- [ ] Test regex patterns

---

## Phase 4: Feature 3 - Chat Participant

### Package.json Configuration
- [ ] Define `chatParticipants` contribution
  - [ ] Set unique ID (`reflex.ask`)
  - [ ] Set name and full name
  - [ ] Set description
  - [ ] Set icon path
  - [ ] Configure `isSticky: true`
- [ ] Define slash commands (if needed)
- [ ] Register configuration command

### Chat Handler Implementation
- [ ] Create `vscode/src/providers/chatProvider.ts`
- [ ] Implement chat participant with `vscode.chat.createChatParticipant()`
- [ ] Implement request handler:
  - [ ] Extract user query from request
  - [ ] Load configuration (provider, API keys)
  - [ ] Execute `rfx ask` with appropriate env vars
  - [ ] Stream response to chat using `ChatResponseStream`
  - [ ] Handle errors gracefully
- [ ] Add message history support (optional)
- [ ] Add tool/command suggestions (optional)

### Testing
- [ ] Test basic chat queries
- [ ] Test with different AI providers (OpenAI, Anthropic, Groq)
- [ ] Test error handling (missing API key, network errors)
- [ ] Test streaming responses
- [ ] Test cancellation

---

## Phase 5: Configuration Management

### VS Code Settings
- [ ] Define `configuration` contribution in `package.json`
  - [ ] `reflex.binaryPath` (string, optional)
  - [ ] `reflex.aiProvider` (enum: openai, anthropic, groq)
  - [ ] `reflex.defaultLanguage` (string, optional)
  - [ ] `reflex.autoReindex` (boolean, default: false)
- [ ] Implement configuration reader in `vscode/src/config.ts`

### SecretStorage Integration
- [ ] Create `vscode/src/secrets.ts`
- [ ] Implement API key storage:
  - [ ] `storeApiKey(provider, key)`
  - [ ] `getApiKey(provider)`
  - [ ] `deleteApiKey(provider)`
- [ ] Support all providers:
  - [ ] OpenAI
  - [ ] Anthropic
  - [ ] Groq

### TOML Sync
- [ ] Add `@iarna/toml` dependency
- [ ] Create `vscode/src/tomlSync.ts`
- [ ] Implement `readReflexConfig()` from `~/.reflex/config.toml`
- [ ] Implement `writeReflexConfig()` to update TOML
- [ ] Handle missing config file gracefully
- [ ] Sync on extension activation
- [ ] Sync when settings change

### Configuration UI
- [ ] Create `reflex.configure` command
- [ ] Show Quick Pick for provider selection
- [ ] Show Input Box for API key entry
- [ ] Validate API keys (optional)
- [ ] Show configuration wizard on first use

### Testing
- [ ] Test reading existing config.toml
- [ ] Test writing to config.toml
- [ ] Test missing config file scenario
- [ ] Test SecretStorage on all platforms (Windows, macOS, Linux)
- [ ] Test configuration sync

---

## Phase 6: Polish & Documentation

### Error Handling
- [ ] Handle missing `rfx` binary gracefully
  - [ ] Show helpful error message
  - [ ] Provide installation instructions
- [ ] Handle invalid queries
- [ ] Handle timeout scenarios
- [ ] Handle network errors (for chat)
- [ ] Handle configuration errors

### User Experience
- [ ] Add loading indicators
- [ ] Add keyboard shortcuts
- [ ] Add context menus (right-click → Search in Reflex)
- [ ] Add codelens integration (optional)
- [ ] Polish webview styles (match VS Code theme)
- [ ] Add icons to all commands

### Documentation
- [ ] Update `vscode/README.md`
  - [ ] Installation instructions
  - [ ] Feature overview with screenshots
  - [ ] Configuration guide
  - [ ] Troubleshooting section
  - [ ] Known issues
- [ ] Update `vscode/DEVELOPMENT.md`
  - [ ] Build instructions
  - [ ] Development workflow
  - [ ] Testing guidelines
  - [ ] Release process
- [ ] Update root `README.md`
  - [ ] Add link to VS Code extension
  - [ ] Add installation instructions
- [ ] Create CHANGELOG.md
- [ ] Add inline code documentation (JSDoc)

### CI/CD
- [ ] Create `.github/workflows/vscode-extension.yml`
  - [ ] Run on changes to `vscode/**`
  - [ ] Lint TypeScript code
  - [ ] Run tests
  - [ ] Build extension
  - [ ] Package .vsix artifact
- [ ] Add automated release workflow (optional)
- [ ] Add marketplace publishing automation (optional)

### Testing
- [ ] Write unit tests for key components
  - [ ] `reflexClient.ts`
  - [ ] `config.ts`
  - [ ] `tomlSync.ts`
- [ ] Write integration tests
  - [ ] Extension activation
  - [ ] Command execution
  - [ ] Webview communication
- [ ] Manual testing checklist:
  - [ ] Fresh install workflow
  - [ ] Search functionality
  - [ ] Chat functionality
  - [ ] Configuration management
  - [ ] Cross-platform testing (Windows, macOS, Linux)

### Packaging & Publishing Prep
- [ ] Review `.vscodeignore` (exclude dev files)
- [ ] Add LICENSE file (if separate from root)
- [ ] Add icon and banner (marketplace assets)
- [ ] Test packaging: `vsce package`
- [ ] Test .vsix installation locally
- [ ] Create publisher account (if publishing)
- [ ] Prepare marketplace listing:
  - [ ] Extension title
  - [ ] Description
  - [ ] Categories
  - [ ] Tags/keywords
  - [ ] Screenshots/GIFs

---

## Key Technical Requirements

### Dependencies

**Production**:
- `@types/vscode` - VS Code API types
- `@iarna/toml` - TOML parsing
- Any webview UI dependencies (React, etc.)

**Development**:
- `typescript` - TypeScript compiler
- `@vscode/vsce` - Extension packaging tool
- `eslint` - Linting
- `prettier` - Code formatting
- `@types/node` - Node.js types

### VS Code Version Compatibility
- Target: VS Code 1.85+ (for Chat Participant API)
- Minimum: TBD based on API usage

### Platform Support
- Windows (x64, arm64)
- macOS (x64, arm64)
- Linux (x64, arm64)

---

## Milestones

### M1: Basic Infrastructure ✅
- Extension scaffolding complete
- Build system working
- Can load in Extension Development Host

### M2: Re-Index Working ✅
- CLI integration validated
- Basic command execution works
- User feedback mechanisms in place

### M3: Search Panel MVP ✅
- Webview UI rendering
- Basic search working
- Results displayed
- Navigation to files works

### M4: Search Panel Complete ✅
- All filters implemented
- Performance optimized
- Error handling complete
- UX polished

### M5: Chat Participant ✅
- Chat participant registered
- Basic queries working
- All providers supported

### M6: Production Ready ✅
- Configuration management complete
- Documentation complete
- Tests passing
- CI/CD configured
- Ready for initial release

---

## Open Questions

- [ ] Should we support regex search in the UI?
- [ ] Should we add AST query support in the UI?
- [ ] Should we implement file watching for auto-reindex?
- [ ] Should we bundle `rfx` binary with extension or require separate install?
- [ ] Should we support workspace-specific config in `.reflex/config.toml`?
- [ ] Should we add telemetry (with opt-in)?
- [ ] Should we support multi-root workspaces?

---

## Resources

### Official Documentation
- [VS Code Extension API](https://code.visualstudio.com/api)
- [Webview API Guide](https://code.visualstudio.com/api/extension-guides/webview)
- [Chat Participant API](https://code.visualstudio.com/api/extension-guides/ai/chat)
- [Publishing Extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)

### Example Repositories
- [React Sidebar Template](https://github.com/anubra266/vscode-sidebar-extension)
- [Chat Sample](https://github.com/microsoft/vscode-extension-samples/tree/main/chat-sample)
- [Webview View Sample](https://github.com/microsoft/vscode-extension-samples/tree/main/webview-view-sample)
- [Google Search Extension](https://github.com/adelphes/google-search-ext)

### Libraries
- [@iarna/toml](https://www.npmjs.com/package/@iarna/toml) - TOML parsing
- [@vscode/vsce](https://www.npmjs.com/package/@vscode/vsce) - Extension CLI

---

## Notes

- **Architecture Decision**: Webview-based search UI required because TreeView doesn't support input boxes
- **Security**: Use SecretStorage API for API keys, never store in settings.json
- **Performance**: Debounce search input, virtualize large result lists
- **Config Sync**: TOML file is source of truth, VS Code settings mirror it
- **Testing Priority**: Focus on CLI integration and webview communication
- **Release Strategy**: Start with manual releases, automate later

---

## Progress Tracking

**Phase 1**: ✅ Complete
**Phase 2**: ✅ Complete
**Phase 3**: ⬜ Not Started
**Phase 4**: ⬜ Not Started
**Phase 5**: ⬜ Not Started
**Phase 6**: ⬜ Not Started

**Overall Progress**: 2/6 phases complete

---

_Last Updated: 2025-01-21_
