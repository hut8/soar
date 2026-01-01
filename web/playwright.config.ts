import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	// Test directory
	testDir: 'e2e',

	// Maximum time one test can run
	timeout: 30 * 1000,

	// Test execution settings
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: 0, // Disabled retries for faster feedback
	workers: process.env.CI ? 4 : undefined, // Limit workers in CI to prevent resource exhaustion

	// Reporter configuration
	reporter: [['html'], ['list'], ...(process.env.CI ? [['github' as const]] : [])],

	// Shared settings for all projects
	use: {
		// Base URL points to shared preview server
		// In CI: Started by CI workflow on port 4173
		// Locally: Start with `npm run preview` before running tests
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
				// Enable SwiftShader for software WebGL rendering (required for Cesium)
				launchOptions: {
					args: [
						'--no-sandbox',
						'--disable-setuid-sandbox',
						'--disable-dev-shm-usage',
						'--disable-web-security',
						'--disable-features=IsolateOrigins,site-per-process',
						'--disable-blink-features=AutomationControlled',
						'--use-gl=swiftshader', // Enable software WebGL for Cesium 3D globe
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
	// NOTE: Web server is started externally (by CI workflow or manually with `npm run preview`)
	// All test workers share the same server and database
	// Tests ensure data isolation through unique identifiers (timestamps, UUIDs)

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
