import * as vscode from 'vscode';
import * as path from 'path';

/**
 * Register the reflex.openFile command
 * Opens a file at a specific line number and highlights it
 */
export function registerOpenFileCommand(context: vscode.ExtensionContext) {
	const disposable = vscode.commands.registerCommand(
		'reflex.openFile',
		async (filePath: string, lineNumber: number) => {
			try {
				// Get the workspace folder
				const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
				if (!workspaceFolder) {
					vscode.window.showErrorMessage('No workspace folder open');
					return;
				}

				// Resolve the file path (handle relative paths)
				let absolutePath: string;
				if (path.isAbsolute(filePath)) {
					absolutePath = filePath;
				} else {
					// Remove leading ./ if present
					const cleanPath = filePath.replace(/^\.\//, '');
					absolutePath = path.join(workspaceFolder.uri.fsPath, cleanPath);
				}

				// Open the document
				const uri = vscode.Uri.file(absolutePath);
				const document = await vscode.workspace.openTextDocument(uri);

				// Show the document and navigate to the line
				// Line numbers from rfx are 1-based, VS Code uses 0-based
				const line = Math.max(0, lineNumber - 1);
				const range = new vscode.Range(line, 0, line, 0);

				await vscode.window.showTextDocument(document, {
					selection: range,
					viewColumn: vscode.ViewColumn.One
				});

				// Reveal the line and highlight it
				const editor = vscode.window.activeTextEditor;
				if (editor) {
					editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
				}
			} catch (error) {
				const errorMessage = error instanceof Error ? error.message : 'Unknown error';
				vscode.window.showErrorMessage(`Failed to open file: ${errorMessage}`);
				console.error('Reflex openFile error:', error);
			}
		}
	);

	context.subscriptions.push(disposable);
}
