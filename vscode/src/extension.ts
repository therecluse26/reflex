import * as vscode from 'vscode';
import { registerReindexCommand } from './commands/reindex';
import { registerOpenFileCommand } from './commands/openFile';
import { registerConfigureAICommand } from './commands/configureAI';
import { SearchViewProvider } from './providers/SearchViewProvider';
import { SecretsManager } from './utils/secrets';

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

	// Create secrets manager for API keys
	const secretsManager = new SecretsManager(context.secrets);

	// Register reindex command
	registerReindexCommand(context);

	// Register openFile command
	registerOpenFileCommand(context);

	// Register AI configuration command
	registerConfigureAICommand(context, secretsManager);

	// Register search view provider (now with chat functionality)
	const searchViewProvider = new SearchViewProvider(
		context.extensionUri,
		context,
		secretsManager
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
