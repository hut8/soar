import { test, expect } from '../fixtures/worker-database.fixture';

test.describe('Globe Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/globe');
	});

	test('should display globe page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/globe|3d|soar/i);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Wait for Cesium to load asynchronously and cesium container to appear
		// Note: Cesium.js is 5.5MB and loads on-demand, so we give it 30s timeout
		const cesiumContainer = page.locator('.cesium-container');
		await expect(cesiumContainer).toBeVisible({ timeout: 30000 });

		// Give time for 3D rendering to initialize
		await page.waitForTimeout(2000);

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('globe-page.png', {
			maxDiffPixelRatio: 0.5 // Higher threshold for 3D rendering
		});
	});

	test('should load without JavaScript errors', async ({ page }) => {
		const errors: string[] = [];

		// Collect console errors
		page.on('pageerror', (error) => {
			errors.push(error.message);
		});

		await page.goto('/globe');
		await page.waitForLoadState('networkidle');

		// Wait for Cesium to load
		const cesiumContainer = page.locator('.cesium-container');
		await expect(cesiumContainer).toBeVisible({ timeout: 30000 });

		// Wait for potential rendering
		await page.waitForTimeout(2000);

		// Should not have critical errors (some warnings may be acceptable)
		const criticalErrors = errors.filter((e) => e.toLowerCase().includes('error'));
		expect(criticalErrors.length).toBe(0);
	});
});
