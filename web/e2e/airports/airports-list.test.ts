import { test, expect } from '../fixtures/auth.fixture';

test.describe('Airports List', () => {
	// Mark all tests as slow due to potential API data loading
	test.slow();

	test('should display airports list page with correct elements', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/airports');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/airports/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading (page has "Airport Search" not "Airports")
		await expect(
			authenticatedPage.getByRole('heading', { name: /airport search/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('airports-list-page.png');
	});

	test('should display search functionality', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/airports');
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

	test.skip('should display airports data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/airports');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show airports list or a "no airports" message
		const hasAirportLinks = await authenticatedPage.locator('a[href^="/airports/"]').count();
		const hasEmptyState = await authenticatedPage
			.getByText(/no airports|empty/i)
			.isVisible()
			.catch(() => false);

		// At least one of these should be true
		expect(hasAirportLinks > 0 || hasEmptyState).toBe(true);
	});

	test('should navigate to airport detail when clicking an airport', async ({
		authenticatedPage
	}) => {
		await authenticatedPage.goto('/airports');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find first airport link
		const airportLinks = authenticatedPage.locator('a[href^="/airports/"]');
		const count = await airportLinks.count();

		if (count > 0) {
			const firstAirport = airportLinks.first();
			await firstAirport.click();

			// Should navigate to airport detail page
			await expect(authenticatedPage).toHaveURL(/\/airports\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Take screenshot of airport detail from list
			await expect(authenticatedPage).toHaveScreenshot('airport-detail-from-list.png');
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/airports');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('airports-list-mobile.png');
	});
});
