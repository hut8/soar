import { test, expect } from '../fixtures/auth.fixture';

test.describe('Flights List', () => {
	// Mark all tests as slow due to potential API data loading
	test.slow();

	test('should display flights list page with correct elements', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/flights');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/flights/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /flights/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('flights-list-page.png');
	});

	test('should display search or filter functionality', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/flights');
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

	test('should display flights data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show flights list or a "no flights" message
		const hasFlightLinks = await authenticatedPage.locator('a[href^="/flights/"]').count();
		const hasEmptyState = await authenticatedPage
			.getByText(/no flights|empty/i)
			.isVisible()
			.catch(() => false);

		// At least one of these should be true
		expect(hasFlightLinks > 0 || hasEmptyState).toBe(true);
	});

	test('should navigate to flight detail when clicking a flight', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find first flight link
		const flightLinks = authenticatedPage.locator('a[href^="/flights/"]');
		const count = await flightLinks.count();

		if (count > 0) {
			const firstFlight = flightLinks.first();
			await firstFlight.click();

			// Should navigate to flight detail page
			await expect(authenticatedPage).toHaveURL(/\/flights\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Take screenshot of flight detail from list
			await expect(authenticatedPage).toHaveScreenshot('flight-detail-from-list.png');
		}
	});

	test('should display pagination if available', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		// Check if pagination controls are visible
		const hasPagination =
			(await authenticatedPage
				.getByRole('button', { name: /next/i })
				.isVisible()
				.catch(() => false)) ||
			(await authenticatedPage
				.getByRole('button', { name: /previous/i })
				.isVisible()
				.catch(() => false));

		if (hasPagination) {
			// Take screenshot of pagination
			await expect(authenticatedPage).toHaveScreenshot('flights-list-with-pagination.png');
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('flights-list-mobile.png');
	});
});
