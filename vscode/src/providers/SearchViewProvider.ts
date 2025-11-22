import * as vscode from 'vscode';
import * as path from 'path';
import { query, ask } from '../utils/reflexClient';
import { SearchFilters, RfxQueryResult, ChatMessage } from '../types/search';
import { SecretsManager } from '../utils/secrets';

/**
 * Provider for the Reflex search webview panel
 */
export class SearchViewProvider implements vscode.WebviewViewProvider {
	public static readonly viewType = 'reflex.searchView';

	private _view?: vscode.WebviewView;
	private _currentSearch?: { query: string; filters: SearchFilters };
	private _chatHistory: ChatMessage[] = [];

	constructor(
		private readonly _extensionUri: vscode.Uri,
		private readonly _context: vscode.ExtensionContext,
		private readonly _secretsManager: SecretsManager
	) {
		// Load chat history from workspace state
		this._chatHistory = this._context.workspaceState.get('reflex.chatHistory', []);
	}

	public resolveWebviewView(
		webviewView: vscode.WebviewView,
		context: vscode.WebviewViewResolveContext,
		_token: vscode.CancellationToken
	) {
		this._view = webviewView;

		webviewView.webview.options = {
			// Allow scripts in the webview
			enableScripts: true,

			// Restrict the webview to only loading content from our extension's directory
			localResourceRoots: [this._extensionUri]
		};

		webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);

		// Handle messages from the webview
		webviewView.webview.onDidReceiveMessage(async (data) => {
			switch (data.type) {
				case 'search':
					await this._handleSearch(data.query, data.filters);
					break;
				case 'navigate':
					await this._handleNavigate(data.path, data.line);
					break;
				case 'reindex':
					await vscode.commands.executeCommand('reflex.reindex');
					break;
				case 'chat':
					await this._handleChat(data.message, data.provider);
					break;
				case 'getChatHistory':
					this._sendChatHistory();
					break;
				case 'clearChatHistory':
					this._clearChatHistory();
					break;
			}
		});
	}

	/**
	 * Send search results to the webview
	 */
	public sendResults(results: any) {
		if (this._view) {
			this._view.webview.postMessage({ type: 'results', data: results });
		}
	}

	/**
	 * Send error message to the webview
	 */
	public sendError(message: string) {
		if (this._view) {
			this._view.webview.postMessage({ type: 'error', message });
		}
	}

	/**
	 * Send loading state to the webview
	 */
	public sendLoading(isLoading: boolean) {
		if (this._view) {
			this._view.webview.postMessage({ type: 'loading', isLoading });
		}
	}

	/**
	 * Handle search request from webview
	 */
	private async _handleSearch(searchQuery: string, filters: SearchFilters) {
		// Don't search if query is empty
		if (!searchQuery.trim()) {
			return;
		}

		// Store current search for potential cancellation
		this._currentSearch = { query: searchQuery, filters };

		// Send loading state
		this.sendLoading(true);

		try {
			// Execute the search
			const result = await query(searchQuery, {
				language: filters.language,
				glob: filters.glob,
				symbolsOnly: filters.symbolsOnly,
				regex: filters.regex,
				kind: filters.kind,
				contains: filters.contains,
				limit: 100
			});

			// Check if this search was cancelled (new search started)
			if (
				this._currentSearch?.query !== searchQuery ||
				JSON.stringify(this._currentSearch?.filters) !== JSON.stringify(filters)
			) {
				return;
			}

			// Send loading state
			this.sendLoading(false);

			if (result.success) {
				// Parse JSON response
				try {
					const queryResult: RfxQueryResult = JSON.parse(result.stdout);
					this.sendResults(queryResult);
				} catch (parseError) {
					this.sendError(`Failed to parse search results: ${parseError}`);
				}
			} else {
				this.sendError(result.stderr || 'Search failed');
			}
		} catch (error) {
			this.sendLoading(false);
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			this.sendError(`Search error: ${errorMessage}`);
		}
	}

	/**
	 * Handle navigation request from webview
	 */
	private async _handleNavigate(filePath: string, line: number) {
		await vscode.commands.executeCommand('reflex.openFile', filePath, line);
	}

	/**
	 * Handle chat message from webview
	 */
	private async _handleChat(message: string, provider?: string) {
		// Send loading state
		this.sendChatLoading(true);

		try {
			// Get configured provider if not specified
			const config = vscode.workspace.getConfiguration('reflex');
			const selectedProvider = (provider || config.get<string>('aiProvider') || 'openai') as 'openai' | 'anthropic' | 'groq';

			// Get API key
			const apiKey = await this._secretsManager.getApiKey(selectedProvider);

			if (!apiKey) {
				const errorMessage: ChatMessage = {
					role: 'error',
					content: `No API key configured for ${selectedProvider}. Please run "Reflex: Configure AI Provider" command.`,
					timestamp: Date.now()
				};
				this.sendChatResponse(errorMessage);
				this._saveChatMessage(errorMessage);
				this.sendChatLoading(false);
				return;
			}

			// Execute rfx ask
			const result = await ask(message, {
				provider: selectedProvider,
				execute: true,
				json: true,
				answer: true,
				apiKey
			});

			this.sendChatLoading(false);

			if (!result.success) {
				const errorMessage: ChatMessage = {
					role: 'error',
					content: result.stderr || 'Failed to get response from AI',
					timestamp: Date.now()
				};
				this.sendChatResponse(errorMessage);
				this._saveChatMessage(errorMessage);
				return;
			}

			// Parse JSON response
			let data: any;
			try {
				data = JSON.parse(result.stdout);
			} catch (parseError) {
				const errorMessage: ChatMessage = {
					role: 'error',
					content: `Failed to parse response: ${result.stdout}`,
					timestamp: Date.now()
				};
				this.sendChatResponse(errorMessage);
				this._saveChatMessage(errorMessage);
				return;
			}

			// Build response message
			const responseMessage: ChatMessage = {
				role: 'assistant',
				content: data.answer || 'No answer provided',
				timestamp: Date.now(),
				queries: data.queries,
				results: data.results ? { ...data, pagination: data.pagination || { total: 0, count: 0, offset: 0, limit: 0, has_more: false } } : undefined
			};

			this.sendChatResponse(responseMessage);
			this._saveChatMessage(responseMessage);

		} catch (error) {
			this.sendChatLoading(false);
			const errorMessage: ChatMessage = {
				role: 'error',
				content: error instanceof Error ? error.message : 'Unknown error occurred',
				timestamp: Date.now()
			};
			this.sendChatResponse(errorMessage);
			this._saveChatMessage(errorMessage);
		}
	}

	/**
	 * Save chat message to history and persist to workspace state
	 */
	private _saveChatMessage(message: ChatMessage) {
		this._chatHistory.push(message);
		this._context.workspaceState.update('reflex.chatHistory', this._chatHistory);
	}

	/**
	 * Send chat history to webview
	 */
	private _sendChatHistory() {
		if (this._view) {
			this._view.webview.postMessage({ type: 'chatHistory', messages: this._chatHistory });
		}
	}

	/**
	 * Clear chat history
	 */
	private _clearChatHistory() {
		this._chatHistory = [];
		this._context.workspaceState.update('reflex.chatHistory', []);
		this._sendChatHistory();
	}

	/**
	 * Send chat response to webview
	 */
	public sendChatResponse(message: ChatMessage) {
		if (this._view) {
			this._view.webview.postMessage({ type: 'chatResponse', message });
		}
	}

	/**
	 * Send chat loading state to webview
	 */
	public sendChatLoading(isLoading: boolean) {
		if (this._view) {
			this._view.webview.postMessage({ type: 'chatLoading', isLoading });
		}
	}

	/**
	 * Generate HTML for the webview
	 */
	private _getHtmlForWebview(webview: vscode.Webview) {
		// Get the local path to main script run in the webview
		const scriptUri = webview.asWebviewUri(
			vscode.Uri.joinPath(this._extensionUri, 'webview-ui', 'dist', 'assets', 'index.js')
		);

		// Get the local path to css file
		const styleUri = webview.asWebviewUri(
			vscode.Uri.joinPath(this._extensionUri, 'webview-ui', 'dist', 'assets', 'index.css')
		);

		// Use a nonce to only allow specific scripts to be run
		const nonce = getNonce();

		return `<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<!--
		Use a content security policy to only allow loading images from https or from our extension directory,
		and only allow scripts that have a specific nonce.
	-->
	<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}';">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<link href="${styleUri}" rel="stylesheet">
	<title>Reflex Search</title>
</head>
<body>
	<div id="root"></div>
	<script type="module" nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
	}
}

function getNonce() {
	let text = '';
	const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
	for (let i = 0; i < 32; i++) {
		text += possible.charAt(Math.floor(Math.random() * possible.length));
	}
	return text;
}
