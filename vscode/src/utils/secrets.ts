import * as vscode from 'vscode';

/**
 * Manager for securely storing and retrieving API keys
 * Uses VS Code's SecretStorage API (platform-native encrypted storage)
 */
export class SecretsManager {
	constructor(private secrets: vscode.SecretStorage) {}

	/**
	 * Get API key for a specific provider
	 */
	async getApiKey(provider: 'openai' | 'anthropic' | 'groq'): Promise<string | undefined> {
		return this.secrets.get(`reflex.${provider}.apiKey`);
	}

	/**
	 * Store API key for a specific provider
	 */
	async setApiKey(provider: 'openai' | 'anthropic' | 'groq', apiKey: string): Promise<void> {
		await this.secrets.store(`reflex.${provider}.apiKey`, apiKey);
	}

	/**
	 * Delete API key for a specific provider
	 */
	async deleteApiKey(provider: 'openai' | 'anthropic' | 'groq'): Promise<void> {
		await this.secrets.delete(`reflex.${provider}.apiKey`);
	}

	/**
	 * Check if API key exists for a provider
	 */
	async hasApiKey(provider: 'openai' | 'anthropic' | 'groq'): Promise<boolean> {
		const key = await this.getApiKey(provider);
		return key !== undefined && key.length > 0;
	}
}
