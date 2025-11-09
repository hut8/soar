import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	// Global setup to seed test database before running tests
	globalSetup: process.env.CI ? undefined : './playwright.global-setup.ts',

	// Test directory
	testDir: 'e2e',

	// Maximum time one test can run
	timeout: 30 * 1000,

	// Test execution settings
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined,

	// Reporter configuration
	reporter: [['html'], ['list'], ...(process.env.CI ? [['github' as const]] : [])],

	// Shared settings for all projects
	use: {
		// Base URL for navigation
		baseURL: 'http://localhost:4173',

		// Collect trace on failure for debugging
		trace: 'on-first-retry',

		// Screenshot settings
		screenshot: 'only-on-failure',

		// Video settings (only on failure to save space)
		video: 'retain-on-failure'
	},

	// Configure projects for different browsers
	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] }
		}

		// Uncomment to test on more browsers:
		// {
		//   name: 'firefox',
		//   use: { ...devices['Desktop Firefox'] },
		// },
		// {
		//   name: 'webkit',
		//   use: { ...devices['Desktop Safari'] },
		// },
	],

	// Web server configuration
	// In CI, servers are started manually in the workflow
	// In local development, Playwright starts them automatically
	webServer: process.env.CI
		? undefined
		: [
				{
					// Start Rust backend server with test database
					command:
						'JWT_SECRET=test-jwt-secret-for-e2e-tests SOAR_ENV=test DATABASE_URL=postgres://postgres:postgres@localhost:5432/soar_test NATS_URL=nats://localhost:4222 ../target/release/soar web --port 61226 --interface localhost',
					port: 61226,
					timeout: 120000, // 2 minutes for backend startup
					reuseExistingServer: true, // Can reuse since globalSetup seeds database first
					env: {
						DATABASE_URL: 'postgres://postgres:postgres@localhost:5432/soar_test',
						// NATS URL for backend (optional, backend should handle missing NATS gracefully)
						NATS_URL: 'nats://localhost:4222',
						// Disable Sentry in tests
						SENTRY_DSN: '',
						// Set environment
						SOAR_ENV: 'test'
					}
				},
				{
					// Start SvelteKit preview server (proxies /data/* to Rust backend)
					command: 'npm run build && npm run preview',
					port: 4173,
					timeout: 180000, // 3 minutes for build + server startup
					reuseExistingServer: true
				}
			],

	// Screenshot comparison settings
	expect: {
		toHaveScreenshot: {
			// Maximum pixel difference ratio
			maxDiffPixelRatio: 0.05,

			// Animation settings
			animations: 'disabled',

			// Caret visibility
			caret: 'hide'
		}
	}
});
