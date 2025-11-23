import * as vscode from 'vscode';
import { registerReindexCommand } from './commands/reindex';
import { registerOpenFileCommand } from './commands/openFile';
import { registerConfigureAICommand } from './commands/configureAI';
import { SearchViewProvider } from './providers/SearchViewProvider';
import { ConfigManager } from './utils/config';

/**
 * This method is called when the extension is activated
 * Activation happens when VS Code starts up (onStartupFinished)
 */
export function activate(context: vscode.ExtensionContext) {
	console.log('Reflex extension is now active');

	// Register a simple command to test the extension works
	const disposable = vscode.commands.registerCommand('reflex.helloWorld', () => {
		vscode.window.showInformationMessage('Hello from Reflex!');
	});
	context.subscriptions.push(disposable);

	// Create config manager for API keys and settings
	const configManager = new ConfigManager();

	// Register reindex command
	registerReindexCommand(context);

	// Register openFile command
	registerOpenFileCommand(context);

	// Register AI configuration command
	registerConfigureAICommand(context, configManager);

	// Register command to clear chat history (useful for recovering from corruption)
	const clearChatCommand = vscode.commands.registerCommand('reflex.clearChatHistory', async () => {
		await context.workspaceState.update('reflex.chatHistory', []);
		vscode.window.showInformationMessage('Reflex chat history cleared');
	});
	context.subscriptions.push(clearChatCommand);

	// Register search view provider (now with chat functionality)
	const searchViewProvider = new SearchViewProvider(
		context.extensionUri,
		context,
		configManager
	);
	context.subscriptions.push(
		vscode.window.registerWebviewViewProvider(
			SearchViewProvider.viewType,
			searchViewProvider
		)
	);
}

/**
 * This method is called when the extension is deactivated
 */
export function deactivate() {
	console.log('Reflex extension is now deactivated');
}
