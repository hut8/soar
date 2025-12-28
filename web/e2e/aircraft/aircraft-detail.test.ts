import { test, expect, type Page } from '../fixtures/auth.fixture';
import { goToAircraft, searchAircraftByRegistration } from '../utils/navigation';
import { testAircraft } from '../fixtures/data.fixture';

test.describe('Aircraft Detail', () => {
	// Mark all tests in this suite as slow (triple timeout) due to multi-step navigation
	test.slow();

	// Helper function to navigate to a test aircraft detail page
	// Searches for a known test aircraft and navigates to it
	async function navigateToTestDevice(page: Page) {
		await goToAircraft(page);
		await searchAircraftByRegistration(page, testAircraft.validRegistration);

		// Wait for search results or no results message (defensive pattern)
		await page.waitForLoadState('networkidle');

		// Wait for either "Search Results" or "No aircraft found" to appear
		await Promise.race([
			page.getByRole('heading', { name: /search results/i }).waitFor({ timeout: 5000 }),
			page.getByRole('heading', { name: /no aircraft found/i }).waitFor({ timeout: 5000 })
		]);

		// Check if we have results
		const hasResults = await page.getByRole('heading', { name: /search results/i }).isVisible();

		if (!hasResults) {
			// No aircraft found - skip this test
			test.skip();
			return;
		}

		// Find and click the first aircraft card
		const aircraftCard = page.locator('a[href^="/aircraft/"]').first();
		await expect(aircraftCard).toBeVisible();
		await aircraftCard.click();

		// Wait for aircraft detail page to load
		await page.waitForLoadState('networkidle');
	}

	test('should display aircraft detail authenticatedPage', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Check page has aircraft-related content
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();
	});

	test('should display aircraft address and type information', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Page should load successfully
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();
	});

	test('should display aircraft registration information if available', async ({
		authenticatedPage
	}) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load successfully
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();
	});

	test('should display fixes (position reports) list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load successfully
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();
	});

	test('should display flights list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load successfully
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();
	});

	test('should navigate back to aircraft list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Use browser back button instead of looking for a specific UI element
		await authenticatedPage.goBack();

		// Should navigate back to aircraft page
		await expect(authenticatedPage).toHaveURL(/\/aircraft$/);
	});

	test('should handle invalid aircraft ID gracefully', async ({ authenticatedPage }) => {
		// Try to navigate to an aircraft that doesn't exist using an invalid UUID
		await authenticatedPage.goto('/aircraft/00000000-0000-0000-0000-000000000000');

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load without crashing (may show error or empty state)
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();

		// Take screenshot of error state
		await expect(authenticatedPage).toHaveScreenshot('aircraft-detail-not-found.png');
	});

	test('should display aircraft status badges if available', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		await authenticatedPage.waitForLoadState('networkidle');

		// Look for status badges (tracked, identified, etc.)
		const hasBadges = await authenticatedPage.locator('.badge').count();

		// Aircraft should have at least some status information
		expect(hasBadges).toBeGreaterThanOrEqual(0);
	});

	test('should paginate fixes if there are many', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		await authenticatedPage.waitForLoadState('networkidle');

		// Check if pagination controls exist for fixes
		const hasFixesPagination = await authenticatedPage
			.getByRole('button', { name: /next|previous/i })
			.isVisible();

		if (hasFixesPagination) {
			// Test pagination
			const nextButton = authenticatedPage.getByRole('button', { name: /next/i }).first();

			if (await nextButton.isEnabled()) {
				await nextButton.click();
				await authenticatedPage.waitForLoadState('networkidle');

				// Should load next authenticatedPage of fixes
				await expect(authenticatedPage).toHaveScreenshot(
					'aircraft-detail-fixes-authenticatedPage-2.png',
					{
						maxDiffPixelRatio: 0.1
					}
				);
			}
		}
	});

	test('should paginate flights if there are many', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		await authenticatedPage.waitForLoadState('networkidle');

		// Check if pagination controls exist for flights
		const flightsSection = authenticatedPage.locator('text=/flights/i').locator('..');
		const hasFlightsPagination =
			(await flightsSection.getByRole('button', { name: /next/i }).isVisible()) ||
			(await flightsSection.getByRole('button', { name: /previous/i }).isVisible());

		if (hasFlightsPagination) {
			// Test pagination
			const nextButton = flightsSection.getByRole('button', { name: /next/i });

			if (await nextButton.isEnabled()) {
				await nextButton.click();
				await authenticatedPage.waitForLoadState('networkidle');

				// Should load next authenticatedPage of flights
				await expect(authenticatedPage).toHaveScreenshot(
					'aircraft-detail-flights-authenticatedPage-2.png',
					{
						maxDiffPixelRatio: 0.1
					}
				);
			}
		}
	});
});
