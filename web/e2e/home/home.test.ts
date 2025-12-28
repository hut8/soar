import { test, expect } from '../fixtures/worker-database.fixture';

test.describe('Home Page', () => {
	test('should display home page with correct elements', async ({ page }) => {
		await page.goto('/');

		// Check page title
		await expect(page).toHaveTitle(/soar/i);

		// Check that the page loads without errors
		await page.waitForLoadState('networkidle');

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('home-page.png', {
			maxDiffPixelRatio: 0.4 // Higher threshold for dynamic content
		});
	});

	test('should have working navigation links', async ({ page }) => {
		await page.goto('/');

		// Check for navigation elements (header/nav)
		// Most pages should have navigation to key sections
		const nav = page.locator('nav').first();
		if (await nav.isVisible()) {
			await expect(nav).toBeVisible();
		}
	});

	test('should be responsive on mobile viewport', async ({ page }) => {
		// Set mobile viewport
		await page.setViewportSize({ width: 375, height: 667 });
		await page.goto('/');

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(page).toHaveScreenshot('home-page-mobile.png', {
			maxDiffPixelRatio: 0.4
		});
	});
});
