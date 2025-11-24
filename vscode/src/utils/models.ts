/**
 * Model options for AI providers
 * Matches the models defined in src/semantic/configure.rs
 */

export interface ModelOption {
	label: string;
	value: string;
}

/**
 * Get model options for a specific provider
 */
export function getModelOptions(provider: 'openai' | 'anthropic' | 'groq'): ModelOption[] {
	switch (provider) {
		case 'openai':
			return [
				{ label: 'GPT-5.1', value: 'gpt-5.1' },
				{ label: 'GPT-5 Mini (recommended)', value: 'gpt-5-mini' },
				{ label: 'GPT-5 Nano', value: 'gpt-5-nano' }
			];
		case 'anthropic':
			return [
				{ label: 'Claude Sonnet 4.5 (recommended)', value: 'claude-sonnet-4-5' },
				{ label: 'Claude Haiku 4.5', value: 'claude-haiku-4-5' },
				{ label: 'Claude Sonnet 4', value: 'claude-sonnet-4' }
			];
		case 'groq':
			return [
				{ label: 'GPT-OSS 120B (recommended)', value: 'openai/gpt-oss-120b' },
				{ label: 'GPT-OSS 20B', value: 'openai/gpt-oss-20b' },
				{ label: 'Llama 4 Maverick 17B', value: 'meta-llama/llama-4-maverick-17b-128e-instruct' },
				{ label: 'Llama 4 Scout 17B', value: 'meta-llama/llama-4-scout-17b-16e-instruct' },
				{ label: 'Qwen 3 32B', value: 'qwen/qwen3-32b' },
				{ label: 'Kimi K2 Instruct', value: 'moonshotai/kimi-k2-instruct-0905' }
			];
	}
}

/**
 * Get all model values (without labels) for a provider
 */
export function getModelValues(provider: 'openai' | 'anthropic' | 'groq'): string[] {
	return getModelOptions(provider).map(opt => opt.value);
}

/**
 * Provider display names
 */
export const PROVIDER_NAMES: Record<'openai' | 'anthropic' | 'groq', string> = {
	openai: 'OpenAI',
	anthropic: 'Anthropic',
	groq: 'Groq'
};
