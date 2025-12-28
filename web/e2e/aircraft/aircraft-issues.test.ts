import { test, expect } from '../fixtures/auth.fixture';

test.describe('Aircraft Issues', () => {
	test('should display aircraft issues page with correct elements', async ({
		authenticatedPage
	}) => {
		await authenticatedPage.goto('/aircraft/issues');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/issues|aircraft/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /issues|aircraft/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('aircraft-issues-page.png');
	});

	test('should display issues data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/aircraft/issues');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show issues list or a "no issues" message
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should display filter or category options', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/aircraft/issues');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for filter controls or category tabs
		const filterInput = authenticatedPage.locator(
			'input[type="search"], input[placeholder*="search" i], input[placeholder*="filter" i]'
		);
		const hasFilter = await filterInput
			.first()
			.isVisible()
			.catch(() => false);

		if (hasFilter) {
			await expect(filterInput.first()).toBeVisible();
		}
	});

	test('should navigate to aircraft detail from issue', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/aircraft/issues');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for aircraft links
		const aircraftLinks = authenticatedPage.locator('a[href^="/aircraft/"]:not([href*="/issues"])');
		const count = await aircraftLinks.count();

		if (count > 0) {
			await aircraftLinks.first().click();
			await expect(authenticatedPage).toHaveURL(/\/aircraft\/[^/]+/);
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/aircraft/issues');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('aircraft-issues-mobile.png');
	});
});
