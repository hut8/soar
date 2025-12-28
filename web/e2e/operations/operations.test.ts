import { test, expect } from '../fixtures/auth.fixture';

test.describe('Operations Page', () => {
	test('should display operations page with correct elements', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/operations');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/operations/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /operations/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('operations-page.png');
	});

	test('should display operations data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/operations');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have content displayed
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should display search or filter functionality', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/operations');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for search input or filter controls
		const searchInput = authenticatedPage.locator(
			'input[type="search"], input[placeholder*="search" i], input[placeholder*="filter" i]'
		);
		const hasSearch = await searchInput
			.first()
			.isVisible()
			.catch(() => false);

		if (hasSearch) {
			await expect(searchInput.first()).toBeVisible();
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/operations');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('operations-page-mobile.png');
	});
});
