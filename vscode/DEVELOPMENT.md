# Reflex VS Code Extension - Development Guide

This guide covers development setup, architecture, and contribution guidelines for the Reflex VS Code extension.

## Prerequisites

- [Node.js](https://nodejs.org/) (v20 or later)
- [npm](https://www.npmjs.com/) (comes with Node.js)
- [VS Code](https://code.visualstudio.com/) (latest version)
- [Reflex CLI](../README.md) installed and available in PATH

## Project Structure

```
vscode/
├── src/                    # Extension source code (TypeScript)
│   ├── extension.ts        # Main entry point
│   ├── providers/          # Webview and chat providers
│   ├── commands/           # Command implementations
│   └── utils/              # Utility functions
├── webview-ui/             # React-based webview UI
│   ├── src/                # React components
│   ├── dist/               # Built assets (generated)
│   └── package.json        # Webview dependencies
├── assets/                 # Icons and static assets
├── out/                    # Compiled TypeScript (generated)
├── package.json            # Extension manifest and dependencies
├── tsconfig.json           # TypeScript configuration
└── .vscode/                # VS Code debug configuration
```

## Development Setup

### 1. Install Dependencies

```bash
cd vscode

# Install extension dependencies
npm install

# Install webview UI dependencies
cd webview-ui
npm install
cd ..
```

### 2. Build the Extension

```bash
# Compile TypeScript
npm run compile

# Or watch for changes
npm run watch
```

### 3. Build the Webview UI

```bash
cd webview-ui

# Build once
npm run build

# Or watch for changes (in a separate terminal)
npm run dev
```

## Running & Debugging

### Method 1: VS Code Debug

1. Open the `vscode/` directory in VS Code
2. Press `F5` or select "Run Extension" from the debug panel
3. A new VS Code window will open with the extension loaded
4. Test the extension in the Extension Development Host window

### Method 2: Manual Testing

1. Build the extension: `npm run compile`
2. Build the webview: `cd webview-ui && npm run build`
3. Press `F5` in VS Code

## Testing

```bash
# Run linter
npm run lint

# Run tests (when implemented)
npm test
```

## Architecture

### Extension Host (src/)

The main extension runs in the VS Code Extension Host process:
- **extension.ts**: Entry point with `activate()` and `deactivate()` functions
- **providers/**: WebviewViewProvider for search panel, ChatParticipant for AI chat
- **commands/**: Command implementations (re-index, configure, etc.)
- **utils/reflexClient.ts**: Wrapper for executing `rfx` CLI commands

### Webview UI (webview-ui/)

The search panel runs in an isolated webview with React:
- **React + TypeScript**: Modern UI development
- **Vite**: Fast build tooling
- **Tailwind CSS**: Styling that respects VS Code themes
- **Message passing**: Communication with extension host via `postMessage()`

### Communication Flow

```
User Action
    ↓
React Webview (postMessage)
    ↓
Extension Host (receives message)
    ↓
Spawn rfx CLI (child_process)
    ↓
Parse JSON output
    ↓
Send results to Webview (postMessage)
    ↓
React updates UI
```

## Development Workflow

### Adding a New Command

1. Define command in `package.json`:
   ```json
   "contributes": {
     "commands": [{
       "command": "reflex.myCommand",
       "title": "Reflex: My Command"
     }]
   }
   ```

2. Implement command in `src/commands/myCommand.ts`:
   ```typescript
   import * as vscode from 'vscode';

   export function registerMyCommand(context: vscode.ExtensionContext) {
     const disposable = vscode.commands.registerCommand('reflex.myCommand', () => {
       vscode.window.showInformationMessage('My command executed!');
     });
     context.subscriptions.push(disposable);
   }
   ```

3. Register in `src/extension.ts`:
   ```typescript
   import { registerMyCommand } from './commands/myCommand';

   export function activate(context: vscode.ExtensionContext) {
     registerMyCommand(context);
   }
   ```

### Adding a Webview Component

1. Create component in `webview-ui/src/components/MyComponent.tsx`
2. Import and use in `webview-ui/src/App.tsx`
3. Build: `cd webview-ui && npm run build`
4. Test in Extension Development Host

### Working with CLI Integration

The extension calls `rfx` commands via `child_process.spawn()`:

```typescript
import { spawn } from 'child_process';

const rfx = spawn('rfx', ['query', 'pattern', '--json']);
rfx.stdout.on('data', (data) => {
  const results = JSON.parse(data.toString());
  // Process results...
});
```

Always use `--json` flag for machine-readable output.

## Packaging

```bash
# Install vsce if needed
npm install -g @vscode/vsce

# Package extension
npm run package

# Creates reflex-0.1.0.vsix
```

Install the .vsix file locally for testing:
```bash
code --install-extension reflex-0.1.0.vsix
```

## Publishing

(Instructions will be added when ready for marketplace publication)

## Troubleshooting

### Extension Not Loading
- Check VS Code Developer Tools: Help > Toggle Developer Tools
- Look for errors in the console
- Verify TypeScript compiled without errors: `npm run compile`

### Webview Not Showing
- Ensure webview UI is built: `cd webview-ui && npm run build`
- Check that `dist/` directory exists in `webview-ui/`
- Check webview console for JavaScript errors

### CLI Integration Issues
- Verify `rfx` is in PATH: `which rfx` or `where rfx`
- Check rfx version: `rfx --version`
- Test rfx commands manually: `rfx query test --json`

## Code Style

- **TypeScript**: Strict mode enabled
- **ESLint**: Run `npm run lint` before committing
- **Formatting**: Use Prettier (configuration coming soon)

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and test thoroughly
4. Run linter: `npm run lint`
5. Commit with descriptive messages
6. Push and create a Pull Request

## Resources

- [VS Code Extension API](https://code.visualstudio.com/api)
- [Webview API Guide](https://code.visualstudio.com/api/extension-guides/webview)
- [Chat Participant API](https://code.visualstudio.com/api/extension-guides/ai/chat)
- [Reflex Documentation](../docs/)

## Questions?

- Open an issue in the main Reflex repository
- Check the [TODO document](../docs/VSCODE_EXTENSION_TODO.md) for planned features
