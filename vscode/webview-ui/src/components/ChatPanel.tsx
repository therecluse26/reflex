import { useState, useEffect, useRef } from 'react';
import { ChatMessage, ExtensionToWebviewMessage, WebviewToExtensionMessage } from '../types/search';

interface ChatPanelProps {
	onConfigure: () => void;
	postMessage: (message: WebviewToExtensionMessage) => void;
	onMessage: (handler: (message: ExtensionToWebviewMessage) => void) => () => void;
}

export default function ChatPanel({ onConfigure, postMessage, onMessage }: ChatPanelProps) {
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [input, setInput] = useState('');
	const [loading, setLoading] = useState(false);
	const messagesEndRef = useRef<HTMLDivElement>(null);

	// Load chat history on mount
	useEffect(() => {
		postMessage({ type: 'getChatHistory' });
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

	return (
		<div className="flex flex-col h-full bg-[var(--vscode-editor-background)] text-[var(--vscode-editor-foreground)]">
			{/* Header */}
			<div className="p-3 border-b border-[var(--vscode-panel-border)] flex items-center justify-between">
				<div className="text-sm font-semibold">AI Chat</div>
				<div className="flex gap-2">
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

				{messages.map((message, index) => (
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
							<div className="text-sm whitespace-pre-wrap">{message.content}</div>

							{/* Queries if available */}
							{message.queries && message.queries.length > 0 && (
								<div className="mt-2 space-y-1">
									<div className="text-xs font-semibold">Generated Queries:</div>
									{message.queries.map((query, qIdx) => (
										<div key={qIdx} className="text-xs font-mono bg-[var(--vscode-textCodeBlock-background)] p-1 rounded">
											{query}
										</div>
									))}
								</div>
							)}

							{/* Timestamp */}
							<div className="text-xs text-[var(--vscode-descriptionForeground)] mt-1">
								{new Date(message.timestamp).toLocaleTimeString()}
							</div>
						</div>
					</div>
				))}

				{loading && (
					<div className="flex justify-start">
						<div className="max-w-[80%] px-3 py-2 rounded bg-[var(--vscode-editor-inactiveSelectionBackground)]">
							<div className="text-sm">Thinking...</div>
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
