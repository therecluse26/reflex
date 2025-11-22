import * as vscode from 'vscode';
import { registerReindexCommand } from './commands/reindex';

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

	// Register reindex command
	registerReindexCommand(context);
}

/**
 * This method is called when the extension is deactivated
 */
export function deactivate() {
	console.log('Reflex extension is now deactivated');
}
