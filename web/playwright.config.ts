import { defineConfig } from '@playwright/test';

export default defineConfig({
	webServer: {
		command: 'npm run build && npm run preview',
		port: 4173,
		timeout: 180000, // 3 minutes for build + server startup
		reuseExistingServer: !process.env.CI
	},
	testDir: 'e2e'
});
