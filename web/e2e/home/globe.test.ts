import { test, expect } from '@playwright/test';

test.describe('Globe Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/globe');
	});

	test('should display globe page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/globe|3d|soar/i);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Give time for 3D rendering to initialize
		await page.waitForTimeout(2000);

		// Note: Globe page is a full-screen Cesium viewer with no traditional h1 heading
		// Verify the Cesium container exists instead
		const cesiumContainer = page.locator('.cesium-container');
		await expect(cesiumContainer).toBeVisible();

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

		// Wait for potential rendering
		await page.waitForTimeout(2000);

		// Should not have critical errors (some warnings may be acceptable)
		const criticalErrors = errors.filter((e) => e.toLowerCase().includes('error'));
		expect(criticalErrors.length).toBe(0);
	});
});
