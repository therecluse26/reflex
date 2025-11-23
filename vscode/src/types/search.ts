/**
 * Type definitions for search functionality
 */

/**
 * Search filters for rfx query
 */
export interface SearchFilters {
	language?: string;
	glob?: string;
	symbolsOnly: boolean;
	regex: boolean;
	kind?: string;
	contains: boolean;
}

/**
 * Match result from rfx query
 */
export interface SearchMatch {
	span: {
		start_line: number;
		end_line: number;
	};
	preview: string;
	context_before: string[];
	context_after: string[];
}

/**
 * File result with matches from rfx query
 */
export interface SearchFileResult {
	path: string;
	matches: SearchMatch[];
}

/**
 * Pagination info from rfx query
 */
export interface SearchPagination {
	total: number;
	count: number;
	offset: number;
	limit: number;
	has_more: boolean;
}

/**
 * Complete response from rfx query --json
 */
export interface RfxQueryResult {
	status: string;
	can_trust_results: boolean;
	warning?: {
		reason: string;
		action_required: string;
		details?: any;
	};
	pagination: SearchPagination;
	results: SearchFileResult[];
}

/**
 * Chat message in conversation history
 */
export interface ChatMessage {
	role: 'user' | 'assistant' | 'error';
	content: string;
	timestamp: number;
	queries?: string[];
	results?: RfxQueryResult;
}

/**
 * Messages from webview to extension
 */
export type WebviewToExtensionMessage =
	| { type: 'search'; query: string; filters: SearchFilters }
	| { type: 'navigate'; path: string; line: number }
	| { type: 'reindex' }
	| { type: 'chat'; message: string; provider?: string }
	| { type: 'getChatHistory' }
	| { type: 'clearChatHistory' }
	| { type: 'configure' }
	| { type: 'getModelInfo' }
	| { type: 'getAvailableModels' }
	| { type: 'selectModel'; provider: string; model: string };

/**
 * Messages from extension to webview
 */
export type ExtensionToWebviewMessage =
	| { type: 'results'; data: RfxQueryResult }
	| { type: 'error'; message: string }
	| { type: 'loading'; isLoading: boolean }
	| { type: 'chatResponse'; message: ChatMessage }
	| { type: 'chatHistory'; messages: ChatMessage[] }
	| { type: 'chatLoading'; isLoading: boolean }
	| { type: 'modelInfo'; provider: string; model: string }
	| { type: 'availableModels'; models: Record<string, string[]>; currentProvider: string; currentModel: string };
