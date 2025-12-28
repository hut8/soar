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
		// Base URL is provided by worker-scoped fixture (each worker has its own server)
		// This allows true test isolation with per-worker databases

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
	// NOTE: Web servers are now started per-worker via the worker-database.fixture.ts
	// This provides true test isolation with each worker having its own database and server
	// Workers start servers on ports 5000, 5001, 5002, etc.
	//
	// The global webServer config is disabled because:
	// 1. Each worker needs its own isolated database
	// 2. Each worker starts its own server on a unique port
	// 3. This enables parallel test execution without data conflicts

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
