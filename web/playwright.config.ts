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
	retries: 0, // Disabled retries for faster feedback
	// Allow parallel execution in CI - use PLAYWRIGHT_WORKERS env var to override
	workers: process.env.PLAYWRIGHT_WORKERS ? parseInt(process.env.PLAYWRIGHT_WORKERS) : undefined,

	// Reporter configuration
	reporter: [['html'], ['list'], ...(process.env.CI ? [['github' as const]] : [])],

	// Shared settings for all projects
	use: {
		// Base URL for navigation
		// Use PLAYWRIGHT_BASE_URL environment variable in CI/Docker, otherwise default to localhost
		baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:4173',

		// Ignore HTTPS errors for test environments
		ignoreHTTPSErrors: true,

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
			use: {
				...devices['Desktop Chrome'],
				// Disable security features that can interfere with Docker/testing
				launchOptions: {
					args: [
						'--no-sandbox',
						'--disable-setuid-sandbox',
						'--disable-dev-shm-usage',
						'--disable-web-security',
						'--disable-features=IsolateOrigins,site-per-process',
						'--disable-blink-features=AutomationControlled',
						'--disable-gpu',
						'--disable-software-rasterizer',
						'--disable-accelerated-2d-canvas'
					]
				}
			}
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
		: {
				// Start soar web server with embedded assets and test database
				// This matches production architecture: single process serving both API and assets
				command: '../target/release/soar web --port 4173 --interface localhost',
				port: 4173,
				timeout: 120000, // 2 minutes for server startup
				reuseExistingServer: true, // Can reuse since globalSetup seeds database first
				env: {
					// Test database
					DATABASE_URL: 'postgres://postgres:postgres@localhost:5432/soar_test',
					// JWT secret for tests
					JWT_SECRET: 'test_jwt_secret_key_for_e2e_tests_only',
					// NATS (use test instance or disable if not needed)
					NATS_URL: 'nats://localhost:4222',
					// Environment
					SOAR_ENV: 'test',
					// Disable Sentry in tests
					SENTRY_DSN: '',
					// SMTP configuration - for local testing, run:
					//   docker run -d -p 1025:1025 -p 8025:8025 axllent/mailpit:v1.20
					// Or use ./scripts/run-acceptance-tests which handles this automatically
					SMTP_SERVER: 'localhost',
					SMTP_PORT: '1025',
					SMTP_USERNAME: 'test',
					SMTP_PASSWORD: 'test',
					FROM_EMAIL: 'test@soar.local',
					FROM_NAME: 'SOAR Test',
					BASE_URL: 'http://localhost:4173'
				}
			},

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
