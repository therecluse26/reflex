import { useCallback, useRef } from 'react';
import { ExtensionToWebviewMessage, WebviewToExtensionMessage } from '../types/search';

type VSCodeAPI = {
	postMessage: (message: WebviewToExtensionMessage) => void;
	getState: () => any;
	setState: (state: any) => any;
};

declare global {
	interface Window {
		acquireVsCodeApi: () => VSCodeAPI;
	}
}

/**
 * Hook to interact with VS Code API
 */
export function useVSCodeAPI() {
	const vscodeRef = useRef<VSCodeAPI | undefined>();

	// Initialize VS Code API (only once)
	if (!vscodeRef.current) {
		vscodeRef.current = window.acquireVsCodeApi();
	}

	/**
	 * Send a message to the extension
	 */
	const postMessage = useCallback((message: WebviewToExtensionMessage) => {
		vscodeRef.current?.postMessage(message);
	}, []);

	/**
	 * Listen for messages from the extension
	 */
	const onMessage = useCallback(
		(handler: (message: ExtensionToWebviewMessage) => void) => {
			const listener = (event: MessageEvent<ExtensionToWebviewMessage>) => {
				handler(event.data);
			};

			window.addEventListener('message', listener);

			// Return cleanup function
			return () => {
				window.removeEventListener('message', listener);
			};
		},
		[]
	);

	return {
		postMessage,
		onMessage,
		vscode: vscodeRef.current
	};
}
