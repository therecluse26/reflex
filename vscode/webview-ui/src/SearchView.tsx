import { useState, useEffect, useCallback } from 'react';
import { useVSCodeAPI } from './hooks/useVSCodeAPI';
import {
	SearchFilters,
	RfxQueryResult,
	ExtensionToWebviewMessage
} from './types/search';

// Supported languages from rfx --help
const LANGUAGES = [
	'rust', 'python', 'javascript', 'typescript', 'vue', 'svelte',
	'go', 'java', 'php', 'c', 'c++', 'c#', 'ruby', 'kotlin', 'zig'
];

// Supported symbol kinds from rfx --help
const SYMBOL_KINDS = [
	'function', 'class', 'struct', 'enum', 'interface', 'trait',
	'constant', 'variable', 'method', 'module', 'namespace', 'type',
	'macro', 'property', 'event', 'import', 'export', 'attribute'
];

export default function SearchView() {
	const { postMessage, onMessage } = useVSCodeAPI();

	// Search state
	const [query, setQuery] = useState('');
	const [filters, setFilters] = useState<SearchFilters>({
		symbolsOnly: false,
		regex: false,
		contains: false
	});

	// Results state
	const [results, setResults] = useState<RfxQueryResult | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	// Listen for messages from extension
	useEffect(() => {
		return onMessage((message: ExtensionToWebviewMessage) => {
			switch (message.type) {
				case 'results':
					setResults(message.data);
					setError(null);
					break;
				case 'error':
					setError(message.message);
					setResults(null);
					break;
				case 'loading':
					setLoading(message.isLoading);
					break;
			}
		});
	}, [onMessage]);

	// Debounced search
	useEffect(() => {
		if (!query.trim()) {
			setResults(null);
			setError(null);
			return;
		}

		const timeoutId = setTimeout(() => {
			postMessage({ type: 'search', query, filters });
		}, 300);

		return () => clearTimeout(timeoutId);
	}, [query, filters, postMessage]);

	// Handle navigation
	const handleNavigate = useCallback(
		(path: string, line: number) => {
			postMessage({ type: 'navigate', path, line });
		},
		[postMessage]
	);

	// Handle reindex
	const handleReindex = useCallback(() => {
		postMessage({ type: 'reindex' });
	}, [postMessage]);

	return (
		<div className="flex flex-col h-full bg-[var(--vscode-editor-background)] text-[var(--vscode-editor-foreground)]">
			{/* Search Input */}
			<div className="p-3 border-b border-[var(--vscode-panel-border)]">
				<input
					type="text"
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					placeholder="Search code..."
					className="w-full px-3 py-2 bg-[var(--vscode-input-background)] text-[var(--vscode-input-foreground)] border border-[var(--vscode-input-border)] rounded focus:outline-none focus:border-[var(--vscode-focusBorder)]"
				/>

				{/* Filters */}
				<div className="mt-2 flex flex-col gap-2 text-sm">
					{/* Row 1: Language and Glob */}
					<div className="flex gap-2">
						<select
							value={filters.language || ''}
							onChange={(e) => setFilters({ ...filters, language: e.target.value || undefined })}
							className="flex-1 px-2 py-1 bg-[var(--vscode-input-background)] text-[var(--vscode-input-foreground)] border border-[var(--vscode-input-border)] rounded text-xs"
						>
							<option value="">All languages</option>
							{LANGUAGES.map(lang => (
								<option key={lang} value={lang}>{lang}</option>
							))}
						</select>
						<input
							type="text"
							value={filters.glob || ''}
							onChange={(e) => setFilters({ ...filters, glob: e.target.value || undefined })}
							placeholder="Glob (e.g., src/**)"
							className="flex-1 px-2 py-1 bg-[var(--vscode-input-background)] text-[var(--vscode-input-foreground)] border border-[var(--vscode-input-border)] rounded text-xs"
						/>
					</div>

					{/* Row 2: Checkboxes */}
					<div className="flex items-center gap-3">
						<label className="flex items-center gap-1 cursor-pointer">
							<input
								type="checkbox"
								checked={filters.symbolsOnly}
								onChange={(e) => setFilters({ ...filters, symbolsOnly: e.target.checked })}
								className="cursor-pointer"
							/>
							<span className="text-xs">Symbols</span>
						</label>

						<label className="flex items-center gap-1 cursor-pointer">
							<input
								type="checkbox"
								checked={filters.regex}
								onChange={(e) => {
									const newRegex = e.target.checked;
									setFilters({
										...filters,
										regex: newRegex,
										contains: newRegex ? false : filters.contains
									});
								}}
								disabled={filters.contains}
								className="cursor-pointer"
							/>
							<span className="text-xs">Regex</span>
						</label>

						<label className="flex items-center gap-1 cursor-pointer">
							<input
								type="checkbox"
								checked={filters.contains}
								onChange={(e) => {
									const newContains = e.target.checked;
									setFilters({
										...filters,
										contains: newContains,
										regex: newContains ? false : filters.regex
									});
								}}
								disabled={filters.regex}
								className="cursor-pointer"
							/>
							<span className="text-xs">Contains</span>
						</label>
					</div>

					{/* Row 3: Kind and Re-index */}
					<div className="flex gap-2">
						<select
							value={filters.kind || ''}
							onChange={(e) => setFilters({ ...filters, kind: e.target.value || undefined })}
							className="flex-1 px-2 py-1 bg-[var(--vscode-input-background)] text-[var(--vscode-input-foreground)] border border-[var(--vscode-input-border)] rounded text-xs"
						>
							<option value="">All kinds</option>
							{SYMBOL_KINDS.map(kind => (
								<option key={kind} value={kind}>{kind}</option>
							))}
						</select>
						<button
							onClick={handleReindex}
							className="px-3 py-1 text-xs bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)] rounded hover:bg-[var(--vscode-button-hoverBackground)]"
						>
							Re-index
						</button>
					</div>
				</div>
			</div>

			{/* Results */}
			<div className="flex-1 overflow-y-auto">
				{loading && (
					<div className="p-4 text-center text-sm text-[var(--vscode-descriptionForeground)]">
						Searching...
					</div>
				)}

				{error && (
					<div className="p-4 text-sm text-[var(--vscode-errorForeground)]">
						<div className="font-semibold">Error:</div>
						<div className="mt-1">{error}</div>
					</div>
				)}

				{!loading && !error && results && (
					<div>
						{/* Summary */}
						<div className="p-3 text-xs text-[var(--vscode-descriptionForeground)] border-b border-[var(--vscode-panel-border)]">
							Found {results.pagination.total} {results.pagination.total === 1 ? 'result' : 'results'}
							{results.warning && (
								<div className="mt-1 text-[var(--vscode-editorWarning-foreground)]">
									⚠ {results.warning.reason}
								</div>
							)}
						</div>

						{/* Results */}
						{results.results.map((fileResult, fileIdx) => (
							<div key={fileIdx} className="border-b border-[var(--vscode-panel-border)]">
								<div className="px-3 py-2 text-sm font-semibold bg-[var(--vscode-sideBar-background)]">
									{fileResult.path}
								</div>

								{fileResult.matches.map((match, matchIdx) => (
									<div
										key={matchIdx}
										onClick={() => handleNavigate(fileResult.path, match.span.start_line)}
										className="px-3 py-2 hover:bg-[var(--vscode-list-hoverBackground)] cursor-pointer"
									>
										<div className="flex items-start gap-2 text-xs">
											<span className="text-[var(--vscode-descriptionForeground)] font-mono">
												{match.span.start_line}
											</span>
											<code className="flex-1 font-mono text-[var(--vscode-editor-foreground)]">
												{match.preview}
											</code>
										</div>
									</div>
								))}
							</div>
						))}
					</div>
				)}

				{!loading && !error && !results && query.trim() && (
					<div className="p-4 text-center text-sm text-[var(--vscode-descriptionForeground)]">
						No results found
					</div>
				)}

				{!loading && !error && !results && !query.trim() && (
					<div className="p-4 text-center text-sm text-[var(--vscode-descriptionForeground)]">
						Enter a search query to find code
					</div>
				)}
			</div>
		</div>
	);
}
