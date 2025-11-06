import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
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
	// Start both the Rust backend and SvelteKit preview server for E2E tests
	webServer: [
		{
			// Start Rust backend server with test database
			command:
				'DATABASE_URL=postgres://postgres:postgres@localhost:5432/soar_test ../target/release/soar web --port 61225 --interface localhost',
			port: 61225,
			timeout: 60000, // 1 minute for backend startup
			reuseExistingServer: !process.env.CI,
			env: {
				DATABASE_URL: 'postgres://postgres:postgres@localhost:5432/soar_test'
			}
		},
		{
			// Start SvelteKit preview server (proxies /data/* to Rust backend)
			command: 'npm run build && npm run preview',
			port: 4173,
			timeout: 180000, // 3 minutes for build + server startup
			reuseExistingServer: !process.env.CI
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
