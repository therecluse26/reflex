import * as vscode from 'vscode';
import { ConfigManager } from '../utils/config';
import { getModelOptions } from '../utils/models';

/**
 * Register the reflex.configureAI command
 * Interactive wizard to configure AI provider and API key
 */
export function registerConfigureAICommand(
	context: vscode.ExtensionContext,
	configManager: ConfigManager
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
			const existingKey = await configManager.hasApiKey(providerValue);

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

				// Store API key in ~/.reflex/config.toml
				await configManager.setApiKey(providerValue, apiKey);
			}

			// Step 4: Select model
			const modelOptions = getModelOptions(providerValue);
			const selectedModel = await vscode.window.showQuickPick(
				[...modelOptions, { label: 'Custom model...', value: 'custom' }],
				{
					placeHolder: `Select model for ${provider.label}`,
					title: 'Reflex AI Configuration'
				}
			);

			if (!selectedModel) {
				return; // User cancelled
			}

			let modelValue: string;
			if (selectedModel.value === 'custom') {
				const customModel = await vscode.window.showInputBox({
					prompt: 'Enter custom model name',
					placeHolder: providerValue === 'openai' ? 'gpt-5.1' : providerValue === 'anthropic' ? 'claude-sonnet-4-5' : 'openai/gpt-oss-120b'
				});

				if (!customModel) {
					return; // User cancelled
				}

				modelValue = customModel;
			} else {
				modelValue = selectedModel.value;
			}

			// Save model to config
			await configManager.setModel(providerValue, modelValue);

			// Step 5: Update provider in ~/.reflex/config.toml
			await configManager.setProvider(providerValue);

			// Also update VS Code setting for UI consistency
			await vscode.workspace
				.getConfiguration('reflex')
				.update('aiProvider', providerValue, vscode.ConfigurationTarget.Global);

			// Step 6: Show success message
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
