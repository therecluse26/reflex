import { useState } from 'react';

function App() {
  const [count, setCount] = useState(0);

  return (
    <div className="min-h-screen bg-[var(--vscode-editor-background)] text-[var(--vscode-editor-foreground)] p-4">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-2xl font-bold mb-4">Reflex Code Search</h1>
        <p className="mb-4">Welcome to Reflex! This is the webview UI.</p>

        <div className="p-4 bg-[var(--vscode-editor-inactiveSelectionBackground)] rounded">
          <button
            onClick={() => setCount((count) => count + 1)}
            className="px-4 py-2 bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)] rounded hover:bg-[var(--vscode-button-hoverBackground)]"
          >
            Count is {count}
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
