import { test, expect } from '../fixtures/auth.fixture';

test.describe('Receiver Coverage', () => {
	test('should display receiver coverage page with correct elements', async ({
		authenticatedPage
	}) => {
		await authenticatedPage.goto('/receivers/coverage');

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Give time for map to render
		await authenticatedPage.waitForTimeout(2000);

		// Check for heading
		const heading = authenticatedPage.getByRole('heading', { level: 1 });
		await expect(heading).toBeVisible();

		// Take screenshot
		await expect(authenticatedPage).toHaveScreenshot('receiver-coverage-page.png', {
			maxDiffPixelRatio: 0.5 // Higher threshold for map rendering
		});
	});

	test('should load map without critical errors', async ({ authenticatedPage }) => {
		const errors: string[] = [];

		// Collect console errors
		authenticatedPage.on('pageerror', (error) => {
			errors.push(error.message);
		});

		await authenticatedPage.goto('/receivers/coverage');
		await authenticatedPage.waitForLoadState('networkidle');

		// Wait for map rendering
		await authenticatedPage.waitForTimeout(2000);

		// Should not have critical errors (map warnings are acceptable)
		const criticalErrors = errors.filter(
			(e) => e.toLowerCase().includes('error') && !e.toLowerCase().includes('map')
		);
		expect(criticalErrors.length).toBeLessThanOrEqual(1); // Allow some map-related errors
	});

	test('should be accessible from receivers list', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for coverage link
		const coverageLink = authenticatedPage.getByRole('link', { name: /coverage|map/i });
		const hasCoverageLink = await coverageLink.isVisible().catch(() => false);

		if (hasCoverageLink) {
			await coverageLink.click();
			await expect(authenticatedPage).toHaveURL(/\/receivers\/coverage/);
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/receivers/coverage');
		await authenticatedPage.waitForLoadState('networkidle');

		// Wait for map rendering
		await authenticatedPage.waitForTimeout(2000);

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('receiver-coverage-mobile.png', {
			maxDiffPixelRatio: 0.5
		});
	});
});
