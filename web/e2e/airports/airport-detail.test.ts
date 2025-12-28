import { test, expect } from '../fixtures/auth.fixture';

test.describe('Airport Detail', () => {
	test('should display airport detail page when navigating from list', async ({
		authenticatedPage
	}) => {
		// First go to airports list
		await authenticatedPage.goto('/airports');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find and click first airport
		const airportLinks = authenticatedPage.locator('a[href^="/airports/"]');
		const count = await airportLinks.count();

		if (count > 0) {
			const firstAirport = airportLinks.first();
			await firstAirport.click();

			// Should navigate to detail page
			await expect(authenticatedPage).toHaveURL(/\/airports\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Check for heading (airport name or identifier)
			const heading = authenticatedPage.getByRole('heading', { level: 1 });
			await expect(heading).toBeVisible();

			// Take screenshot
			await expect(authenticatedPage).toHaveScreenshot('airport-detail-page.png');
		}
	});

	test('should display airport information', async ({ authenticatedPage }) => {
		// Navigate to airports list first
		await authenticatedPage.goto('/airports');
		await authenticatedPage.waitForLoadState('networkidle');

		const airportLinks = authenticatedPage.locator('a[href^="/airports/"]');
		const count = await airportLinks.count();

		if (count > 0) {
			await airportLinks.first().click();
			await authenticatedPage.waitForLoadState('networkidle');

			// Should have some content displayed
			const bodyText = await authenticatedPage.textContent('body');
			expect(bodyText).toBeTruthy();
			expect(bodyText!.length).toBeGreaterThan(0);
		}
	});

	test('should handle invalid airport ID gracefully', async ({ authenticatedPage }) => {
		// Navigate to non-existent airport
		await authenticatedPage.goto('/airports/invalid-airport-id-999999');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show error message or redirect
		const hasError = await authenticatedPage
			.getByText(/not found|error|invalid/i)
			.isVisible()
			.catch(() => false);
		const isRedirected = !authenticatedPage.url().includes('/airports/invalid-airport-id-999999');

		expect(hasError || isRedirected).toBe(true);
	});
});
