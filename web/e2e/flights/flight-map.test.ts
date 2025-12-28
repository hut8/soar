import { test, expect } from '../fixtures/auth.fixture';

test.describe('Flight Map', () => {
	test('should display flight map page when navigating from flight detail', async ({
		authenticatedPage
	}) => {
		// First go to flights list
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find and click first flight
		const flightLinks = authenticatedPage.locator('a[href^="/flights/"]');
		const count = await flightLinks.count();

		if (count > 0) {
			const firstFlight = flightLinks.first();
			const href = await firstFlight.getAttribute('href');
			await firstFlight.click();
			await authenticatedPage.waitForLoadState('networkidle');

			// Look for map link or navigate directly
			const mapLink = authenticatedPage.getByRole('link', { name: /map|view map/i });
			const hasMapLink = await mapLink.isVisible().catch(() => false);

			if (hasMapLink) {
				await mapLink.click();
			} else if (href) {
				// Navigate directly to map page
				await authenticatedPage.goto(`${href}/map`);
			}

			await authenticatedPage.waitForLoadState('networkidle');

			// Should be on map page
			await expect(authenticatedPage).toHaveURL(/\/flights\/[^/]+\/map/);

			// Give time for map to render
			await authenticatedPage.waitForTimeout(2000);

			// Take screenshot
			await expect(authenticatedPage).toHaveScreenshot('flight-map-page.png', {
				maxDiffPixelRatio: 0.5 // Higher threshold for map rendering
			});
		}
	});

	test('should load map without critical errors', async ({ authenticatedPage }) => {
		// First go to flights list
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		const flightLinks = authenticatedPage.locator('a[href^="/flights/"]');
		const count = await flightLinks.count();

		if (count > 0) {
			const href = await flightLinks.first().getAttribute('href');

			if (href) {
				const errors: string[] = [];

				// Collect console errors
				authenticatedPage.on('pageerror', (error) => {
					errors.push(error.message);
				});

				await authenticatedPage.goto(`${href}/map`);
				await authenticatedPage.waitForLoadState('networkidle');
				await authenticatedPage.waitForTimeout(2000);

				// Should not have critical errors
				const criticalErrors = errors.filter((e) => e.toLowerCase().includes('error'));
				expect(criticalErrors.length).toBe(0);
			}
		}
	});

	test('should handle invalid flight ID gracefully on map page', async ({ authenticatedPage }) => {
		// Navigate to non-existent flight map
		await authenticatedPage.goto('/flights/00000000-0000-0000-0000-999999999999/map');
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load without crashing (may show error, redirect, or empty state)
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
	});
});
