import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import * as toml from '@iarna/toml';

/**
 * Manager for reading/writing Reflex configuration from ~/.reflex/config.toml
 * Stores API keys and provider settings in user's home directory
 */
export class ConfigManager {
	private readonly configPath: string;
	private readonly configDir: string;

	constructor() {
		// Cross-platform path to ~/.reflex/config.toml
		this.configDir = path.join(os.homedir(), '.reflex');
		this.configPath = path.join(this.configDir, 'config.toml');
	}

	/**
	 * Ensure ~/.reflex directory exists
	 */
	private ensureConfigDir(): void {
		if (!fs.existsSync(this.configDir)) {
			fs.mkdirSync(this.configDir, { recursive: true, mode: 0o700 });
		}
	}

	/**
	 * Read and parse config file
	 */
	private readConfig(): any {
		this.ensureConfigDir();

		if (!fs.existsSync(this.configPath)) {
			return {};
		}

		try {
			const content = fs.readFileSync(this.configPath, 'utf-8');
			return toml.parse(content);
		} catch (error) {
			console.error('Failed to parse config file:', error);
			return {};
		}
	}

	/**
	 * Write config to file
	 */
	private writeConfig(config: any): void {
		this.ensureConfigDir();

		try {
			const content = toml.stringify(config);
			fs.writeFileSync(this.configPath, content, { mode: 0o600 });
		} catch (error) {
			console.error('Failed to write config file:', error);
			throw error;
		}
	}

	/**
	 * Get API key for a specific provider
	 */
	async getApiKey(provider: 'openai' | 'anthropic' | 'groq'): Promise<string | undefined> {
		const config = this.readConfig();
		const credentials = config.credentials || {};

		const keyName = `${provider}_api_key`;
		return credentials[keyName];
	}

	/**
	 * Store API key for a specific provider
	 */
	async setApiKey(provider: 'openai' | 'anthropic' | 'groq', apiKey: string): Promise<void> {
		const config = this.readConfig();

		if (!config.credentials) {
			config.credentials = {};
		}

		const keyName = `${provider}_api_key`;
		config.credentials[keyName] = apiKey;

		this.writeConfig(config);
	}

	/**
	 * Delete API key for a specific provider
	 */
	async deleteApiKey(provider: 'openai' | 'anthropic' | 'groq'): Promise<void> {
		const config = this.readConfig();

		if (!config.credentials) {
			return;
		}

		const keyName = `${provider}_api_key`;
		delete config.credentials[keyName];

		this.writeConfig(config);
	}

	/**
	 * Check if API key exists for a provider
	 */
	async hasApiKey(provider: 'openai' | 'anthropic' | 'groq'): Promise<boolean> {
		const key = await this.getApiKey(provider);
		return key !== undefined && key.length > 0;
	}

	/**
	 * Get the configured AI provider
	 */
	async getProvider(): Promise<'openai' | 'anthropic' | 'groq' | undefined> {
		const config = this.readConfig();
		const semantic = config.semantic || {};
		return semantic.provider;
	}

	/**
	 * Set the configured AI provider
	 */
	async setProvider(provider: 'openai' | 'anthropic' | 'groq'): Promise<void> {
		const config = this.readConfig();

		if (!config.semantic) {
			config.semantic = {};
		}

		config.semantic.provider = provider;

		this.writeConfig(config);
	}

	/**
	 * Get the configured model for a specific provider
	 */
	async getModel(provider: 'openai' | 'anthropic' | 'groq'): Promise<string | undefined> {
		const config = this.readConfig();
		const credentials = config.credentials || {};

		const modelKey = `${provider}_model`;
		return credentials[modelKey];
	}

	/**
	 * Set the model for a specific provider
	 */
	async setModel(provider: 'openai' | 'anthropic' | 'groq', model: string): Promise<void> {
		const config = this.readConfig();

		if (!config.credentials) {
			config.credentials = {};
		}

		const modelKey = `${provider}_model`;
		config.credentials[modelKey] = model;

		this.writeConfig(config);
	}
}
