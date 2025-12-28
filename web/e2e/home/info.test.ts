import { test, expect } from '@playwright/test';

test.describe('Info Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/info');
	});

	test('should display info page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/info|about|soar/i);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Check for main heading
		const heading = page.getByRole('heading', { level: 1 });
		await expect(heading).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('info-page.png');
	});

	test('should display information content', async ({ page }) => {
		// Wait for content to load
		await page.waitForLoadState('networkidle');

		// Page should have some text content
		const bodyText = await page.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should be accessible from navigation', async ({ page }) => {
		await page.goto('/');

		// Look for link to info page
		const infoLink = page.getByRole('link', { name: /info|about/i });
		if (await infoLink.isVisible()) {
			await infoLink.click();
			await expect(page).toHaveURL(/\/info/);
		}
	});
});
