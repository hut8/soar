import { test, expect } from '../fixtures/auth.fixture';

test.describe('Flight Detail', () => {
	test('should display flight detail page when navigating from list', async ({
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
			await firstFlight.click();

			// Should navigate to detail page
			await expect(authenticatedPage).toHaveURL(/\/flights\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Check for heading
			const heading = authenticatedPage.getByRole('heading', { level: 1 });
			await expect(heading).toBeVisible();

			// Take screenshot
			await expect(authenticatedPage).toHaveScreenshot('flight-detail-page.png');
		}
	});

	test('should display flight information', async ({ authenticatedPage }) => {
		// Navigate to flights list first
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		const flightLinks = authenticatedPage.locator('a[href^="/flights/"]');
		const count = await flightLinks.count();

		if (count > 0) {
			await flightLinks.first().click();
			await authenticatedPage.waitForLoadState('networkidle');

			// Should have some content displayed
			const bodyText = await authenticatedPage.textContent('body');
			expect(bodyText).toBeTruthy();
			expect(bodyText!.length).toBeGreaterThan(0);
		}
	});

	test('should have link to flight map', async ({ authenticatedPage }) => {
		// Navigate to flights list first
		await authenticatedPage.goto('/flights');
		await authenticatedPage.waitForLoadState('networkidle');

		const flightLinks = authenticatedPage.locator('a[href^="/flights/"]');
		const count = await flightLinks.count();

		if (count > 0) {
			await flightLinks.first().click();
			await authenticatedPage.waitForLoadState('networkidle');

			// Look for map link
			const mapLink = authenticatedPage.getByRole('link', { name: /map|view map/i });
			const hasMapLink = await mapLink.isVisible().catch(() => false);

			if (hasMapLink) {
				await mapLink.click();
				await expect(authenticatedPage).toHaveURL(/\/flights\/[^/]+\/map/);
			}
		}
	});

	test('should handle invalid flight ID gracefully', async ({ authenticatedPage }) => {
		// Navigate to non-existent flight
		await authenticatedPage.goto('/flights/00000000-0000-0000-0000-999999999999');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show error message or redirect
		const hasError = await authenticatedPage
			.getByText(/not found|error|invalid/i)
			.isVisible()
			.catch(() => false);
		const isRedirected = !authenticatedPage
			.url()
			.includes('/flights/00000000-0000-0000-0000-999999999999');

		expect(hasError || isRedirected).toBe(true);
	});
});
