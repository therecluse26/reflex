import * as vscode from 'vscode';
import { SecretsManager } from '../utils/secrets';

/**
 * Register the reflex.configureAI command
 * Interactive wizard to configure AI provider and API key
 */
export function registerConfigureAICommand(
	context: vscode.ExtensionContext,
	secretsManager: SecretsManager
) {
	const disposable = vscode.commands.registerCommand('reflex.configureAI', async () => {
		try {
			// Step 1: Select provider
			const provider = await vscode.window.showQuickPick(
				[
					{ label: 'OpenAI', value: 'openai' },
					{ label: 'Anthropic (Claude)', value: 'anthropic' },
					{ label: 'Groq', value: 'groq' }
				],
				{
					placeHolder: 'Select AI provider',
					title: 'Reflex AI Configuration'
				}
			);

			if (!provider) {
				return; // User cancelled
			}

			const providerValue = provider.value as 'openai' | 'anthropic' | 'groq';

			// Step 2: Check if API key already exists for this provider
			const existingKey = await secretsManager.hasApiKey(providerValue);

			let shouldUpdateKey = true;

			if (existingKey) {
				// Ask if user wants to update the existing key
				const action = await vscode.window.showQuickPick(
					[
						{ label: 'Update API key', value: 'update' },
						{ label: 'Keep existing key', value: 'keep' }
					],
					{
						placeHolder: `API key already configured for ${provider.label}`,
						title: 'Reflex AI Configuration'
					}
				);

				if (!action) {
					return; // User cancelled
				}

				shouldUpdateKey = action.value === 'update';
			}

			// Step 3: Enter API key (only if updating or no existing key)
			if (shouldUpdateKey) {
				const apiKey = await vscode.window.showInputBox({
					prompt: `Enter your ${provider.label} API key`,
					password: true,
					placeHolder: 'sk-...',
					validateInput: (value) => {
						if (!value || value.trim().length === 0) {
							return 'API key cannot be empty';
						}
						return null;
					}
				});

				if (!apiKey) {
					return; // User cancelled
				}

				// Store API key in SecretStorage
				await secretsManager.setApiKey(providerValue, apiKey);
			}

			// Step 4: Update provider setting
			await vscode.workspace
				.getConfiguration('reflex')
				.update('aiProvider', providerValue, vscode.ConfigurationTarget.Global);

			// Step 5: Show success message
			const message = shouldUpdateKey
				? `✓ Updated ${provider.label} API key and set as default provider`
				: `✓ Set ${provider.label} as default provider (using existing API key)`;
			vscode.window.showInformationMessage(message);
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			vscode.window.showErrorMessage(`Failed to configure AI: ${errorMessage}`);
			console.error('Reflex configureAI error:', error);
		}
	});

	context.subscriptions.push(disposable);
}
