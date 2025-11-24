import { useState, useEffect, useRef } from 'react';
import { ChatMessage, ExtensionToWebviewMessage, WebviewToExtensionMessage, ProgressEvent } from '../types/search';

interface ChatPanelProps {
	onConfigure: () => void;
	postMessage: (message: WebviewToExtensionMessage) => void;
	onMessage: (handler: (message: ExtensionToWebviewMessage) => void) => () => void;
}

// Convert ProgressEvent to user-friendly status message
function getProgressMessage(event: ProgressEvent): string {
	switch (event.type) {
		case 'triaging':
			return 'Analyzing question...';
		case 'answering_from_context':
			return 'Answering from conversation history...';
		case 'thinking':
			return event.reasoning ? `Thinking... ${event.reasoning}` : 'Thinking...';
		case 'tools':
			const toolCount = event.tool_calls?.length || 0;
			return `Gathering context (${toolCount} tools)...`;
		case 'queries':
			const queryCount = event.queries?.length || 0;
			return `Generated ${queryCount} ${queryCount === 1 ? 'query' : 'queries'}...`;
		case 'executing':
			const resultCount = event.results_count || 0;
			return `Found ${resultCount} ${resultCount === 1 ? 'result' : 'results'}...`;
		case 'processing_page':
			if (event.current && event.total) {
				return `Processing page ${event.current}/${event.total}...`;
			}
			return 'Processing results...';
		case 'generating_summary':
			if (event.current && event.total) {
				return `Generating summary for page ${event.current}/${event.total}...`;
			}
			return 'Generating summary...';
		case 'synthesizing_answer':
			if (event.summary_count) {
				return `Synthesizing final answer from ${event.summary_count} ${event.summary_count === 1 ? 'summary' : 'summaries'}...`;
			}
			return 'Synthesizing answer...';
		case 'reindexing':
			if (event.current && event.total) {
				return `Reindexing (${event.current}/${event.total})...`;
			}
			return 'Reindexing...';
		case 'answer':
			return 'Generating answer...';
		case 'error':
			return event.error || 'An error occurred';
		case 'done':
			return 'Done';
		default:
			return 'Processing...';
	}
}

export default function ChatPanel({ onConfigure, postMessage, onMessage }: ChatPanelProps) {
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [input, setInput] = useState('');
	const [loading, setLoading] = useState(false);
	const [progressMessage, setProgressMessage] = useState<string>('Thinking...');
	const [modelInfo, setModelInfo] = useState<{ provider: string; model: string } | null>(null);
	const [availableModels, setAvailableModels] = useState<Record<string, string[]>>({});
	const messagesEndRef = useRef<HTMLDivElement>(null);

	// Load chat history and available models on mount
	useEffect(() => {
		postMessage({ type: 'getChatHistory' });
		postMessage({ type: 'getAvailableModels' });
	}, [postMessage]);

	// Listen for messages from extension
	useEffect(() => {
		return onMessage((message) => {
			switch (message.type) {
				case 'chatResponse':
					setMessages((prev) => [...prev, message.message]);
					setLoading(false);
					break;
				case 'chatHistory':
					setMessages(message.messages);
					break;
				case 'chatLoading':
					setLoading(message.isLoading);
					break;
				case 'chatProgress':
					// Update progress message based on event type
					if (message.event) {
						const statusMsg = getProgressMessage(message.event);
						setProgressMessage(statusMsg);
					}
					break;
				case 'modelInfo':
					setModelInfo({ provider: message.provider, model: message.model });
					break;
				case 'availableModels':
					setAvailableModels(message.models);
					setModelInfo({ provider: message.currentProvider, model: message.currentModel });
					break;
			}
		});
	}, [onMessage]);

	// Auto-scroll to bottom
	useEffect(() => {
		messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
	}, [messages]);

	const handleSend = () => {
		if (!input.trim() || loading) {
			return;
		}

		// Add user message immediately
		const userMessage: ChatMessage = {
			role: 'user',
			content: input,
			timestamp: Date.now()
		};
		setMessages((prev) => [...prev, userMessage]);

		// Send to extension
		postMessage({ type: 'chat', message: input });

		// Clear input
		setInput('');
		setLoading(true);
	};

	const handleClearHistory = () => {
		postMessage({ type: 'clearChatHistory' });
		setMessages([]);
	};

	const handleKeyPress = (e: React.KeyboardEvent) => {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
	};

	const handleModelChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
		const selectedValue = e.target.value;
		// Parse the value (format: "provider:model")
		const [provider, model] = selectedValue.split(':');
		if (provider && model) {
			postMessage({ type: 'selectModel', provider, model });
		}
	};

	return (
		<div className="flex flex-col h-full bg-[var(--vscode-editor-background)] text-[var(--vscode-editor-foreground)]">
			{/* Header */}
			<div className="p-3 border-b border-[var(--vscode-panel-border)] flex flex-col gap-2">
				<div className="flex flex-col gap-1">
					<div className="text-sm font-semibold">AI Chat</div>
					{modelInfo && Object.keys(availableModels).length > 0 && (
						<select
							value={`${modelInfo.provider}:${modelInfo.model}`}
							onChange={handleModelChange}
							className="text-xs bg-[var(--vscode-input-background)] text-[var(--vscode-input-foreground)] border border-[var(--vscode-input-border)] rounded px-1 py-0.5 max-w-xs"
						>
							{Object.entries(availableModels).map(([provider, models]) => (
								<optgroup key={provider} label={provider.charAt(0).toUpperCase() + provider.slice(1)}>
									{models.map(model => (
										<option key={`${provider}:${model}`} value={`${provider}:${model}`}>
											{model}
										</option>
									))}
								</optgroup>
							))}
						</select>
					)}
				</div>
				<div className="flex gap-2 justify-end">
					<button
						onClick={onConfigure}
						className="px-2 py-1 text-xs bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)] rounded hover:bg-[var(--vscode-button-hoverBackground)]"
					>
						⚙️ Configure
					</button>
					{messages.length > 0 && (
						<button
							onClick={handleClearHistory}
							className="px-2 py-1 text-xs bg-[var(--vscode-button-secondaryBackground)] text-[var(--vscode-button-secondaryForeground)] rounded hover:bg-[var(--vscode-button-secondaryHoverBackground)]"
						>
							🗑️ Clear
						</button>
					)}
				</div>
			</div>

			{/* Messages */}
			<div className="flex-1 overflow-y-auto p-3 space-y-3">
				{messages.length === 0 && !loading && (
					<div className="text-center text-sm text-[var(--vscode-descriptionForeground)] mt-8">
						<div className="mb-2">💬 Start a conversation</div>
						<div className="text-xs">Ask questions about your code in natural language</div>
					</div>
				)}

				{messages.map((message, index) => {
					// Ensure content is always a string for rendering
					const safeContent = typeof message.content === 'string'
						? message.content
						: JSON.stringify(message.content);

					return (
					<div key={index} className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}>
						<div
							className={`max-w-[80%] px-3 py-2 rounded ${
								message.role === 'user'
									? 'bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)]'
									: message.role === 'error'
									? 'bg-[var(--vscode-inputValidation-errorBackground)] text-[var(--vscode-inputValidation-errorForeground)]'
									: 'bg-[var(--vscode-editor-inactiveSelectionBackground)] text-[var(--vscode-editor-foreground)]'
							}`}
						>
							{/* Message content */}
							<div className="text-sm whitespace-pre-wrap">{safeContent}</div>

							{/* Queries if available */}
							{message.queries && message.queries.length > 0 && (
								<div className="mt-2 space-y-1">
									<div className="text-xs font-semibold">Generated Queries:</div>
									{message.queries.map((query, qIdx) => {
										// Handle query objects from rfx ask --json
										const queryText = typeof query === 'string'
											? query
											: (query as any)?.command || JSON.stringify(query);

										return (
											<div key={qIdx} className="text-xs font-mono bg-[var(--vscode-textCodeBlock-background)] p-1 rounded">
												{queryText}
											</div>
										);
									})}
								</div>
							)}

							{/* Timestamp */}
							<div className="text-xs text-[var(--vscode-descriptionForeground)] mt-1">
								{new Date(message.timestamp).toLocaleTimeString()}
							</div>
						</div>
					</div>
					);
				})}

				{loading && (
					<div className="flex justify-start">
						<div className="max-w-[80%] px-3 py-2 rounded bg-[var(--vscode-editor-inactiveSelectionBackground)]">
							<div className="text-sm">{progressMessage}</div>
						</div>
					</div>
				)}

				<div ref={messagesEndRef} />
			</div>

			{/* Input */}
			<div className="p-3 border-t border-[var(--vscode-panel-border)]">
				<div className="flex gap-2">
					<textarea
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyPress={handleKeyPress}
						placeholder="Ask a question about your code..."
						disabled={loading}
						className="flex-1 px-3 py-2 bg-[var(--vscode-input-background)] text-[var(--vscode-input-foreground)] border border-[var(--vscode-input-border)] rounded focus:outline-none focus:border-[var(--vscode-focusBorder)] resize-none"
						rows={2}
					/>
					<button
						onClick={handleSend}
						disabled={!input.trim() || loading}
						className="px-4 py-2 bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)] rounded hover:bg-[var(--vscode-button-hoverBackground)] disabled:opacity-50 disabled:cursor-not-allowed"
					>
						Send
					</button>
				</div>
				<div className="text-xs text-[var(--vscode-descriptionForeground)] mt-1">
					Press Enter to send, Shift+Enter for new line
				</div>
			</div>
		</div>
	);
}
