import * as vscode from 'vscode';
import { reindex } from '../utils/reflexClient';
import { promptToAddToGitignore } from '../utils/gitignore';

/**
 * Register the reflex.reindex command
 */
export function registerReindexCommand(context: vscode.ExtensionContext) {
	const disposable = vscode.commands.registerCommand('reflex.reindex', async () => {
		// Create status bar item
		const statusBarItem = vscode.window.createStatusBarItem(
			vscode.StatusBarAlignment.Left,
			100
		);
		statusBarItem.text = '$(sync~spin) Indexing...';
		statusBarItem.show();

		try {
			// Execute reindex
			const result = await reindex();

			// Hide status bar
			statusBarItem.hide();
			statusBarItem.dispose();

			if (result.success) {
				// Show success message
				vscode.window.showInformationMessage('✓ Reflex index updated successfully');

				// Prompt to add .reflex/ to .gitignore if needed
				await promptToAddToGitignore(context);
			} else {
				// Show error with details
				const errorMessage = result.stderr || 'Unknown error occurred';
				vscode.window.showErrorMessage(`Failed to reindex: ${errorMessage}`);

				// Log full output to console for debugging
				console.error('Reflex reindex failed:', {
					stdout: result.stdout,
					stderr: result.stderr,
					exitCode: result.exitCode
				});
			}
		} catch (error) {
			// Hide status bar
			statusBarItem.hide();
			statusBarItem.dispose();

			// Show error
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			vscode.window.showErrorMessage(`Failed to reindex: ${errorMessage}`);
			console.error('Reflex reindex error:', error);
		}
	});

	context.subscriptions.push(disposable);
}
