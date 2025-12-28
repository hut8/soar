import { test as base, type Page } from '@playwright/test';
import { execSync, spawn, type ChildProcess } from 'child_process';
import path from 'path';

/**
 * Worker-scoped fixture for isolated database and web server per test worker.
 *
 * This fixture provides true test isolation by:
 * 1. Creating a unique database per worker from the template database
 * 2. Starting a dedicated web server for each worker on a unique port
 * 3. Cleaning up the database and server after all tests in the worker complete
 *
 * Benefits:
 * - Tests can create/modify data without affecting other workers
 * - Registration tests can run without email conflicts
 * - Full parallel execution maintained
 * - Fast database creation via PostgreSQL template cloning
 *
 * Usage:
 *   import { test, expect } from '../fixtures/worker-database.fixture';
 *
 *   test('my test', async ({ page }) => {
 *     await page.goto('/'); // Automatically uses worker's server
 *   });
 */

// Worker-scoped fixtures (one instance per worker)
type WorkerFixtures = {
	workerDatabaseUrl: string;
	workerBaseURL: string;
	workerServerProcess: ChildProcess;
};

export const test = base.extend<{ page: Page }, WorkerFixtures>({
	// Create isolated database for this worker from template
	workerDatabaseUrl: [
		// eslint-disable-next-line @typescript-eslint/no-unused-vars, no-empty-pattern
		async ({}, use, workerInfo) => {
			const workerIndex = workerInfo.workerIndex;
			const dbName = `soar_test_worker_${workerIndex}`;

			console.log(`[Worker ${workerIndex}] Creating database ${dbName} from template...`);

			// Drop existing database if it exists (cleanup from previous failed run)
			try {
				execSync(
					`psql -h localhost -U postgres -d postgres -c "DROP DATABASE IF EXISTS ${dbName} WITH (FORCE);"`,
					{
						env: { ...process.env, PGPASSWORD: 'postgres' },
						stdio: 'pipe'
					}
				);
			} catch {
				// Ignore errors - database might not exist
			}

			// Create database from template (fast operation, ~1 second)
			execSync(
				`psql -h localhost -U postgres -d postgres -c "CREATE DATABASE ${dbName} TEMPLATE soar_test_template;"`,
				{
					env: { ...process.env, PGPASSWORD: 'postgres' },
					stdio: 'inherit'
				}
			);

			const databaseUrl = `postgres://postgres:postgres@localhost:5432/${dbName}`;
			console.log(`[Worker ${workerIndex}] Database ready: ${databaseUrl}`);

			await use(databaseUrl);

			// Cleanup: Drop worker database after all tests complete
			console.log(`[Worker ${workerIndex}] Cleaning up database ${dbName}...`);
			try {
				execSync(
					`psql -h localhost -U postgres -d postgres -c "DROP DATABASE IF EXISTS ${dbName} WITH (FORCE);"`,
					{
						env: { ...process.env, PGPASSWORD: 'postgres' },
						stdio: 'pipe'
					}
				);
			} catch (err) {
				console.error(`[Worker ${workerIndex}] Failed to drop database: ${err}`);
			}
		},
		{ scope: 'worker', timeout: 60000 }
	],

	// Determine base URL for this worker
	workerBaseURL: [
		// eslint-disable-next-line @typescript-eslint/no-unused-vars, no-empty-pattern
		async ({}, use, workerInfo) => {
			const workerIndex = workerInfo.workerIndex;
			const port = 5000 + workerIndex; // Port 5000, 5001, 5002, etc.
			const baseURL = `http://localhost:${port}`;
			console.log(`[Worker ${workerIndex}] Base URL: ${baseURL}`);
			await use(baseURL);
		},
		{ scope: 'worker' }
	],

	// Start dedicated web server for this worker
	workerServerProcess: [
		async ({ workerDatabaseUrl, workerBaseURL }, use, workerInfo) => {
			const workerIndex = workerInfo.workerIndex;
			const port = 5000 + workerIndex;

			console.log(`[Worker ${workerIndex}] Starting web server on port ${port}...`);

			// Binary path: from web/e2e/fixtures/ go up to project root, then into target/release
			const binaryPath = path.join(__dirname, '../../../target/release/soar');

			const serverProcess = spawn(
				binaryPath,
				['web', '--port', port.toString(), '--interface', 'localhost'],
				{
					env: {
						...process.env,
						DATABASE_URL: workerDatabaseUrl,
						JWT_SECRET: 'test_jwt_secret_key_for_e2e_tests_only',
						NATS_URL: 'nats://localhost:4222',
						SOAR_ENV: 'test',
						SENTRY_DSN: '',
						SMTP_SERVER: 'localhost',
						SMTP_PORT: '1025',
						SMTP_USERNAME: 'test',
						SMTP_PASSWORD: 'test',
						FROM_EMAIL: 'test@soar.local',
						FROM_NAME: 'SOAR Test',
						BASE_URL: workerBaseURL,
						RUST_LOG: 'info'
					},
					stdio: 'pipe'
				}
			);

			// Log server output for debugging
			serverProcess.stdout?.on('data', (data) => {
				console.log(`[Worker ${workerIndex} Server] ${data.toString().trim()}`);
			});

			serverProcess.stderr?.on('data', (data) => {
				console.error(`[Worker ${workerIndex} Server] ${data.toString().trim()}`);
			});

			serverProcess.on('error', (error) => {
				console.error(`[Worker ${workerIndex}] Server process error:`, error);
			});

			serverProcess.on('exit', (code, signal) => {
				console.log(`[Worker ${workerIndex}] Server exited with code ${code} and signal ${signal}`);
			});

			// Wait for server to be ready (check root endpoint)
			await waitForServer(workerBaseURL, 60000);
			console.log(`[Worker ${workerIndex}] Web server ready on port ${port}`);

			await use(serverProcess);

			// Cleanup: Kill server process
			console.log(`[Worker ${workerIndex}] Stopping web server...`);
			serverProcess.kill('SIGTERM');

			// Give server time to shut down gracefully
			await new Promise((resolve) => setTimeout(resolve, 1000));

			// Force kill if still running
			if (!serverProcess.killed) {
				serverProcess.kill('SIGKILL');
			}
		},
		{ scope: 'worker', timeout: 120000 }
	],

	// Override page fixture to auto-prepend worker base URL
	// IMPORTANT: Must depend on workerServerProcess to ensure server starts before tests run
	page: async ({ page, workerBaseURL, workerServerProcess }, use) => {
		// Wait for server to be running (workerServerProcess ensures this)
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		const _ensureServerRunning = workerServerProcess;

		// Intercept page.goto to automatically prepend the worker's base URL
		const originalGoto = page.goto.bind(page);
		page.goto = async (url: string, options?: Parameters<typeof page.goto>[1]) => {
			// If URL is absolute (starts with http), use as-is
			// Otherwise prepend worker base URL
			const fullUrl = url.startsWith('http') ? url : workerBaseURL + url;
			return originalGoto(fullUrl, options);
		};

		await use(page);
	}
});

/**
 * Wait for server to start responding on the given URL
 */
async function waitForServer(url: string, timeout: number): Promise<void> {
	const start = Date.now();
	const checkInterval = 100; // Check every 100ms

	while (Date.now() - start < timeout) {
		try {
			const response = await fetch(url);
			// Server is responding if we get any HTTP response (even 404)
			if (response.ok || response.status === 404 || response.status === 401) {
				return;
			}
		} catch {
			// Server not ready yet, continue waiting
		}

		await new Promise((resolve) => setTimeout(resolve, checkInterval));
	}

	throw new Error(`Server at ${url} did not start within ${timeout}ms`);
}

export { expect } from '@playwright/test';
