import { spawn, ChildProcess } from 'child_process';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Result from executing an rfx command
 */
export interface RfxCommandResult {
	success: boolean;
	stdout: string;
	stderr: string;
	exitCode: number | null;
}

/**
 * Options for executing rfx commands
 */
export interface RfxCommandOptions {
	args: string[];
	cwd?: string;
	timeout?: number; // milliseconds
	env?: Record<string, string>; // Environment variables
	onStdout?: (data: string) => void;
	onStderr?: (data: string) => void;
}

/**
 * Find the rfx binary path
 * Checks (in order):
 * 1. User configured path (reflex.binaryPath setting)
 * 2. Bundled binary from npm package (node_modules/.bin/rfx)
 * 3. System PATH
 */
export async function findRfxBinary(): Promise<string | null> {
	// 1. Check user configuration first
	const config = vscode.workspace.getConfiguration('reflex');
	const configuredPath = config.get<string>('binaryPath');

	if (configuredPath) {
		return configuredPath;
	}

	// 2. Check for bundled binary
	const extensionPath = vscode.extensions.getExtension('reflex.reflex')?.extensionPath;
	if (extensionPath) {
		const bundledBinaryPath = path.join(
			extensionPath,
			'node_modules',
			'.bin',
			process.platform === 'win32' ? 'rfx.cmd' : 'rfx'
		);

		if (fs.existsSync(bundledBinaryPath)) {
			return bundledBinaryPath;
		}
	}

	// 3. Try to find in system PATH
	return new Promise((resolve) => {
		const which = process.platform === 'win32' ? 'where' : 'which';
		const proc = spawn(which, ['rfx']);

		let output = '';
		proc.stdout.on('data', (data) => {
			output += data.toString();
		});

		proc.on('close', (code) => {
			if (code === 0 && output.trim()) {
				// Return first line (in case multiple paths found)
				resolve(output.trim().split('\n')[0]);
			} else {
				resolve(null);
			}
		});

		proc.on('error', () => {
			resolve(null);
		});
	});
}

/**
 * Execute an rfx command
 */
export async function executeRfx(options: RfxCommandOptions): Promise<RfxCommandResult> {
	const rfxPath = await findRfxBinary();

	if (!rfxPath) {
		return {
			success: false,
			stdout: '',
			stderr: 'rfx binary not found. Please install Reflex or configure the binary path in settings.',
			exitCode: null
		};
	}

	return new Promise((resolve) => {
		const cwd = options.cwd || vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
		const timeout = options.timeout || 60000; // 60 seconds default

		let stdout = '';
		let stderr = '';
		let timedOut = false;

		const proc: ChildProcess = spawn(rfxPath, options.args, {
			cwd,
			shell: false,
			env: { ...process.env, ...options.env } // Merge environment variables
		});

		// Set timeout
		const timeoutId = setTimeout(() => {
			timedOut = true;
			proc.kill();
		}, timeout);

		// Capture stdout
		proc.stdout?.on('data', (data: Buffer) => {
			const chunk = data.toString();
			stdout += chunk;
			if (options.onStdout) {
				options.onStdout(chunk);
			}
		});

		// Capture stderr
		proc.stderr?.on('data', (data: Buffer) => {
			const chunk = data.toString();
			stderr += chunk;
			if (options.onStderr) {
				options.onStderr(chunk);
			}
		});

		// Handle completion
		proc.on('close', (code: number | null) => {
			clearTimeout(timeoutId);

			if (timedOut) {
				resolve({
					success: false,
					stdout,
					stderr: stderr + '\nCommand timed out',
					exitCode: null
				});
			} else {
				resolve({
					success: code === 0,
					stdout,
					stderr,
					exitCode: code
				});
			}
		});

		// Handle spawn errors
		proc.on('error', (err: Error) => {
			clearTimeout(timeoutId);
			resolve({
				success: false,
				stdout,
				stderr: stderr + `\nFailed to spawn rfx: ${err.message}`,
				exitCode: null
			});
		});
	});
}

/**
 * Execute rfx index command
 */
export async function reindex(): Promise<RfxCommandResult> {
	return executeRfx({
		args: ['index'],
		timeout: 120000 // 2 minutes for indexing
	});
}

/**
 * Execute rfx query command
 */
export async function query(
	searchQuery: string,
	options?: {
		language?: string;
		glob?: string;
		symbolsOnly?: boolean;
		regex?: boolean;
		kind?: string;
		contains?: boolean;
		limit?: number;
	}
): Promise<RfxCommandResult> {
	const args = ['query', searchQuery, '--json'];

	if (options?.language) {
		args.push('--lang', options.language);
	}

	if (options?.glob) {
		args.push('--glob', options.glob);
	}

	if (options?.symbolsOnly) {
		args.push('--symbols');
	}

	if (options?.regex) {
		args.push('--regex');
	}

	if (options?.kind) {
		args.push('--kind', options.kind);
	}

	if (options?.contains) {
		args.push('--contains');
	}

	if (options?.limit) {
		args.push('--limit', String(options.limit));
	}

	return executeRfx({
		args,
		timeout: 30000 // 30 seconds for queries
	});
}

/**
 * Execute rfx ask command (AI-powered natural language search)
 */
export async function ask(
	question: string,
	options?: {
		provider?: string;
		execute?: boolean;
		json?: boolean;
		answer?: boolean;
		agentic?: boolean;
		apiKey?: string; // API key to pass via environment variable
	}
): Promise<RfxCommandResult> {
	const args = ['ask', question];

	if (options?.execute) {
		args.push('--execute');
	}

	if (options?.json) {
		args.push('--json');
	}

	if (options?.answer) {
		args.push('--answer');
	}

	if (options?.agentic) {
		args.push('--agentic');
	}

	if (options?.provider) {
		args.push('--provider', options.provider);
	}

	// Build environment variables for API key
	const env: Record<string, string> = {};
	if (options?.apiKey && options?.provider) {
		const envVarName = `${options.provider.toUpperCase()}_API_KEY`;
		env[envVarName] = options.apiKey;
	}

	return executeRfx({
		args,
		env,
		timeout: 60000 // 60 seconds for AI queries
	});
}

/**
 * Chat session API types and functions (HTTP API)
 */

export interface ChatSession {
	session_id: string;
	provider: string;
	model: string;
}

export interface ChatSessionMessage {
	role: string;
	content: string;
	timestamp: number;
	queries?: string[];
}

/**
 * Get the API server URL
 * If serverManager is provided, uses its URL (for auto-started server)
 * Otherwise falls back to default port 57878
 */
function getApiUrl(serverManager?: any): string {
	if (serverManager && typeof serverManager.getUrl === 'function') {
		return serverManager.getUrl();
	}
	// Default port for manually started servers
	return 'http://127.0.0.1:57878';
}

/**
 * Create a new chat session via HTTP API
 */
export async function createChatSession(
	provider: string,
	model?: string,
	serverManager?: any
): Promise<ChatSession> {
	const url = `${getApiUrl(serverManager)}/chat/sessions`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ provider, model })
	});

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to create chat session: ${error}`);
	}

	return response.json() as Promise<ChatSession>;
}

/**
 * Send a message to a chat session via HTTP API
 */
export async function sendChatMessage(
	sessionId: string,
	message: string,
	serverManager?: any
): Promise<ChatSessionMessage> {
	const url = `${getApiUrl(serverManager)}/chat/sessions/${sessionId}/messages`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ message })
	});

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to send message: ${error}`);
	}

	return response.json() as Promise<ChatSessionMessage>;
}

/**
 * Progress event from SSE stream
 */
export interface ProgressEvent {
	type: 'triaging' | 'answering_from_context' | 'thinking' | 'tools' | 'queries' | 'executing' | 'reindexing' | 'answer' | 'error' | 'done';
	reasoning?: string;
	needs_context?: boolean;
	content?: string;
	tool_calls?: string[];
	queries?: string[];
	results_count?: number;
	execution_time_ms?: number;
	current?: number;
	total?: number;
	message?: string;
	answer?: string;
	error?: string;
}

/**
 * Send a message to a chat session with SSE streaming progress updates
 */
export async function sendChatMessageStream(
	sessionId: string,
	message: string,
	onProgress: (event: ProgressEvent) => void,
	serverManager?: any
): Promise<void> {
	const url = `${getApiUrl(serverManager)}/chat/sessions/${sessionId}/messages/stream`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ message })
	});

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to send message: ${error}`);
	}

	// Read SSE stream
	const reader = response.body?.getReader();
	if (!reader) {
		throw new Error('No response body');
	}

	const decoder = new TextDecoder();
	let buffer = '';

	try {
		while (true) {
			const { done, value } = await reader.read();
			if (done) {
				break;
			}

			// Decode chunk and add to buffer
			buffer += decoder.decode(value, { stream: true });

			// Process complete SSE events (separated by double newline)
			const events = buffer.split('\n\n');
			buffer = events.pop() || ''; // Keep incomplete event in buffer

			for (const eventText of events) {
				if (!eventText.trim()) {
					continue;
				}

				// Parse SSE event format: "data: {json}\n"
				const lines = eventText.split('\n');
				for (const line of lines) {
					if (line.startsWith('data: ')) {
						const data = line.substring(6); // Remove "data: " prefix
						try {
							const event = JSON.parse(data) as ProgressEvent;
							onProgress(event);

							// Stop reading after done event
							if (event.type === 'done') {
								reader.cancel();
								return;
							}
						} catch (e) {
							console.error('Failed to parse SSE event:', data, e);
						}
					}
				}
			}
		}
	} finally {
		reader.releaseLock();
	}
}

/**
 * Get chat session info via HTTP API
 */
export async function getChatSessionInfo(
	sessionId: string,
	serverManager?: any
): Promise<{
	session_id: string;
	provider: string;
	model: string;
	total_tokens: number;
	context_limit: number;
	context_usage: number;
	message_count: number;
}> {
	const url = `${getApiUrl(serverManager)}/chat/sessions/${sessionId}`;
	const response = await fetch(url);

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to get session info: ${error}`);
	}

	return response.json() as Promise<{
		session_id: string;
		provider: string;
		model: string;
		total_tokens: number;
		context_limit: number;
		context_usage: number;
		message_count: number;
	}>;
}

/**
 * Delete a chat session via HTTP API
 */
export async function deleteChatSession(
	sessionId: string,
	serverManager?: any
): Promise<void> {
	const url = `${getApiUrl(serverManager)}/chat/sessions/${sessionId}`;
	const response = await fetch(url, { method: 'DELETE' });

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to delete session: ${error}`);
	}
}

/**
 * Check if the API server is running
 */
export async function isApiServerRunning(serverManager?: any): Promise<boolean> {
	try {
		const url = `${getApiUrl(serverManager)}/health`;
		const response = await fetch(url, {
			method: 'GET',
			signal: AbortSignal.timeout(2000) // 2 second timeout
		});
		return response.ok;
	} catch (error) {
		return false;
	}
}
