import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Check if .reflex/ is already in .gitignore
 */
function isReflexInGitignore(gitignorePath: string): boolean {
	if (!fs.existsSync(gitignorePath)) {
		return false;
	}

	const content = fs.readFileSync(gitignorePath, 'utf8');
	// Check for .reflex/ or .reflex (with or without trailing slash/comments)
	return /^\.reflex\/?(\s|$|#)/m.test(content);
}

/**
 * Add .reflex/ to .gitignore file
 */
function addReflexToGitignore(gitignorePath: string): void {
	const content = fs.readFileSync(gitignorePath, 'utf8');

	// Add with proper spacing
	const newContent = content.endsWith('\n')
		? content + '.reflex/\n'
		: content + '\n.reflex/\n';

	fs.writeFileSync(gitignorePath, newContent, 'utf8');
}

/**
 * Prompt user to add .reflex/ to .gitignore after first successful index
 */
export async function promptToAddToGitignore(context: vscode.ExtensionContext): Promise<void> {
	// Check if we should prompt
	const config = vscode.workspace.getConfiguration('reflex');
	const shouldPrompt = config.get<boolean>('promptGitignore', true);

	if (!shouldPrompt) {
		return;
	}

	// Check if we've already prompted this session
	const hasPromptedKey = 'reflex.hasPromptedGitignore';
	const hasPrompted = context.workspaceState.get<boolean>(hasPromptedKey, false);

	if (hasPrompted) {
		return;
	}

	// Get workspace folder
	const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
	if (!workspaceFolder) {
		return;
	}

	const gitignorePath = path.join(workspaceFolder.uri.fsPath, '.gitignore');

	// Check if .gitignore exists
	if (!fs.existsSync(gitignorePath)) {
		return;
	}

	// Check if .reflex/ already in .gitignore
	if (isReflexInGitignore(gitignorePath)) {
		return;
	}

	// Mark as prompted for this session
	await context.workspaceState.update(hasPromptedKey, true);

	// Prompt user
	const response = await vscode.window.showInformationMessage(
		'The .reflex/ directory should be added to .gitignore to avoid committing index files. Add it now?',
		'Yes',
		'No',
		'Don\'t ask again'
	);

	if (response === 'Yes') {
		try {
			addReflexToGitignore(gitignorePath);
			vscode.window.showInformationMessage('✓ Added .reflex/ to .gitignore');
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			vscode.window.showErrorMessage(`Failed to update .gitignore: ${errorMessage}`);
		}
	} else if (response === 'Don\'t ask again') {
		// Save preference globally
		await config.update('promptGitignore', false, vscode.ConfigurationTarget.Global);
		vscode.window.showInformationMessage('You can re-enable this prompt in settings (reflex.promptGitignore)');
	}
}
