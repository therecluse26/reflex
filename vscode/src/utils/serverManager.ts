import { spawn, ChildProcess } from 'child_process';
import * as vscode from 'vscode';
import { findRfxBinary } from './reflexClient';

/**
 * Server state tracking
 */
enum ServerState {
	Stopped = 'stopped',
	Starting = 'starting',
	Running = 'running',
	Error = 'error'
}

/**
 * Manages the lifecycle of the Reflex API server (rfx serve)
 *
 * Automatically starts the server in the background when needed,
 * monitors its health, and handles crashes/restarts.
 */
export class ServerManager {
	private static readonly PORT_RANGE_START = 57878;
	private static readonly PORT_RANGE_END = 57978;
	private static readonly HOST = '127.0.0.1';
	private static readonly HEALTH_CHECK_INTERVAL_MS = 500;
	private static readonly HEALTH_CHECK_TIMEOUT_MS = 10000; // 10 seconds
	private static readonly MAX_RESTART_ATTEMPTS = 3;

	private state: ServerState = ServerState.Stopped;
	private process?: ChildProcess;
	private outputChannel: vscode.OutputChannel;
	private restartAttempts = 0;
	private startPromise?: Promise<void>;
	private port?: number;

	constructor(
		private readonly workspaceFolder: vscode.WorkspaceFolder
	) {
		this.outputChannel = vscode.window.createOutputChannel('Reflex Server');
	}

	/**
	 * Get the server port
	 */
	public getPort(): number {
		if (!this.port) {
			throw new Error('Server port not assigned - server has not been started');
		}
		return this.port;
	}

	/**
	 * Get the server host
	 */
	public getHost(): string {
		return ServerManager.HOST;
	}

	/**
	 * Get the server URL
	 */
	public getUrl(): string {
		if (!this.port) {
			throw new Error('Server port not assigned - server has not been started');
		}
		return `http://${ServerManager.HOST}:${this.port}`;
	}

	/**
	 * Check if server is running
	 */
	public isRunning(): boolean {
		return this.state === ServerState.Running;
	}

	/**
	 * Ensure server is running, auto-start if needed
	 */
	public async ensureRunning(): Promise<void> {
		// If already running, nothing to do
		if (this.state === ServerState.Running) {
			return;
		}

		// If currently starting, wait for that to complete
		if (this.state === ServerState.Starting && this.startPromise) {
			return this.startPromise;
		}

		// Start the server
		return this.start();
	}

	/**
	 * Start the server
	 */
	public async start(): Promise<void> {
		// Prevent multiple concurrent starts
		if (this.startPromise) {
			return this.startPromise;
		}

		this.startPromise = this._doStart();
		try {
			await this.startPromise;
		} finally {
			this.startPromise = undefined;
		}
	}

	/**
	 * Find an available port in the configured range
	 */
	private async findAvailablePort(): Promise<number> {
		for (let port = ServerManager.PORT_RANGE_START; port <= ServerManager.PORT_RANGE_END; port++) {
			try {
				// Try to connect to the port - if it succeeds, port is in use
				const response = await fetch(`http://${ServerManager.HOST}:${port}/health`, {
					method: 'GET',
					signal: AbortSignal.timeout(100)
				});
				// Port is in use, try next one
			} catch (error) {
				// Port is available (connection refused or timeout)
				return port;
			}
		}
		throw new Error(`No available ports in range ${ServerManager.PORT_RANGE_START}-${ServerManager.PORT_RANGE_END}`);
	}

	/**
	 * Internal start implementation
	 */
	private async _doStart(): Promise<void> {
		if (this.state === ServerState.Running) {
			return;
		}

		if (this.state === ServerState.Starting) {
			throw new Error('Server is already starting');
		}

		// Find an available port
		this.port = await this.findAvailablePort();

		this.outputChannel.appendLine(`[${new Date().toISOString()}] Starting Reflex server on port ${this.port}...`);
		this.state = ServerState.Starting;

		try {
			// Find rfx binary
			const rfxPath = await findRfxBinary();
			if (!rfxPath) {
				throw new Error(
					'rfx binary not found. Please install Reflex or configure the binary path in settings.\n' +
					'Install via: cargo install reflex-search'
				);
			}

			// Spawn the server process
			this.process = spawn(
				rfxPath,
				['serve', '--port', String(this.port), '--host', ServerManager.HOST],
				{
					cwd: this.workspaceFolder.uri.fsPath,
					shell: false,
					detached: false,
					stdio: ['ignore', 'pipe', 'pipe']
				}
			);

			// Log server output
			this.process.stdout?.on('data', (data: Buffer) => {
				this.outputChannel.append(data.toString());
			});

			this.process.stderr?.on('data', (data: Buffer) => {
				this.outputChannel.append(data.toString());
			});

			// Handle process exit
			this.process.on('exit', (code, signal) => {
				this.outputChannel.appendLine(
					`[${new Date().toISOString()}] Server exited with code ${code}, signal ${signal}`
				);

				if (this.state === ServerState.Running) {
					// Unexpected exit - attempt restart
					this.state = ServerState.Stopped;
					this.process = undefined;
					this._attemptRestart();
				} else {
					// Expected exit
					this.state = ServerState.Stopped;
					this.process = undefined;
				}
			});

			// Handle spawn errors
			this.process.on('error', (error: Error) => {
				this.outputChannel.appendLine(`[${new Date().toISOString()}] Server error: ${error.message}`);
				this.state = ServerState.Error;
				this.process = undefined;
				throw error;
			});

			// Wait for server to become healthy
			await this._waitForHealthy();

			this.outputChannel.appendLine(`[${new Date().toISOString()}] Server is ready at ${this.getUrl()}`);
			this.state = ServerState.Running;
			this.restartAttempts = 0; // Reset restart counter on success

		} catch (error) {
			this.state = ServerState.Error;
			this.process?.kill();
			this.process = undefined;

			const errorMessage = error instanceof Error ? error.message : String(error);
			this.outputChannel.appendLine(`[${new Date().toISOString()}] Failed to start server: ${errorMessage}`);
			this.outputChannel.show();

			throw new Error(`Failed to start Reflex server: ${errorMessage}`);
		}
	}

	/**
	 * Wait for server to respond to health checks
	 */
	private async _waitForHealthy(): Promise<void> {
		const startTime = Date.now();

		while (Date.now() - startTime < ServerManager.HEALTH_CHECK_TIMEOUT_MS) {
			try {
				const response = await fetch(`${this.getUrl()}/health`, {
					method: 'GET',
					signal: AbortSignal.timeout(1000)
				});

				if (response.ok) {
					return; // Server is healthy!
				}
			} catch (error) {
				// Health check failed, retry
			}

			// Wait before next check
			await new Promise(resolve => setTimeout(resolve, ServerManager.HEALTH_CHECK_INTERVAL_MS));

			// Check if process died
			if (!this.process || this.process.exitCode !== null) {
				throw new Error('Server process exited during startup');
			}
		}

		throw new Error('Server health check timeout - server did not become ready');
	}

	/**
	 * Attempt to restart server after crash
	 */
	private async _attemptRestart(): Promise<void> {
		if (this.restartAttempts >= ServerManager.MAX_RESTART_ATTEMPTS) {
			this.outputChannel.appendLine(
				`[${new Date().toISOString()}] Max restart attempts (${ServerManager.MAX_RESTART_ATTEMPTS}) reached`
			);
			vscode.window.showErrorMessage(
				'Reflex server crashed multiple times. Please check the output for errors.',
				'Show Output'
			).then(action => {
				if (action === 'Show Output') {
					this.outputChannel.show();
				}
			});
			return;
		}

		this.restartAttempts++;
		const backoffMs = Math.min(1000 * Math.pow(2, this.restartAttempts - 1), 10000);

		this.outputChannel.appendLine(
			`[${new Date().toISOString()}] Attempting restart ${this.restartAttempts}/${ServerManager.MAX_RESTART_ATTEMPTS} in ${backoffMs}ms...`
		);

		await new Promise(resolve => setTimeout(resolve, backoffMs));

		try {
			await this.start();
			vscode.window.showInformationMessage('Reflex server restarted successfully');
		} catch (error) {
			this.outputChannel.appendLine(
				`[${new Date().toISOString()}] Restart failed: ${error instanceof Error ? error.message : String(error)}`
			);
		}
	}

	/**
	 * Stop the server gracefully
	 */
	public async stop(): Promise<void> {
		if (this.state === ServerState.Stopped) {
			return;
		}

		this.outputChannel.appendLine(`[${new Date().toISOString()}] Stopping server...`);

		if (this.process) {
			// Kill the process
			this.process.kill('SIGTERM');

			// Wait up to 5 seconds for graceful shutdown
			await new Promise<void>((resolve) => {
				const timeout = setTimeout(() => {
					if (this.process && this.process.exitCode === null) {
						this.outputChannel.appendLine('[${new Date().toISOString()}] Force killing server...');
						this.process.kill('SIGKILL');
					}
					resolve();
				}, 5000);

				this.process?.once('exit', () => {
					clearTimeout(timeout);
					resolve();
				});
			});

			this.process = undefined;
		}

		this.state = ServerState.Stopped;
		this.outputChannel.appendLine(`[${new Date().toISOString()}] Server stopped`);
	}

	/**
	 * Dispose of resources
	 */
	public dispose(): void {
		this.stop();
		this.outputChannel.dispose();
	}
}
