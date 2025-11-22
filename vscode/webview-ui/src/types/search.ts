/**
 * Type definitions for search functionality (webview side)
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
 * Messages from webview to extension
 */
export type WebviewToExtensionMessage =
	| { type: 'search'; query: string; filters: SearchFilters }
	| { type: 'navigate'; path: string; line: number }
	| { type: 'reindex' };

/**
 * Messages from extension to webview
 */
export type ExtensionToWebviewMessage =
	| { type: 'results'; data: RfxQueryResult }
	| { type: 'error'; message: string }
	| { type: 'loading'; isLoading: boolean };
