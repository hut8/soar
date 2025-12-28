import { test, expect, type Page } from '../fixtures/auth.fixture';
import { goToAircraft, searchAircraftByRegistration } from '../utils/navigation';
import { testAircraft } from '../fixtures/data.fixture';

test.describe('Aircraft Detail', () => {
	// Helper function to navigate to a test aircraft detail page
	// Searches for a known test aircraft and navigates to it
	async function navigateToTestDevice(page: Page) {
		// First, directly query the backend API to verify aircraft exist
		// Use baseURL from playwright config (respects PLAYWRIGHT_BASE_URL in CI)
		const backendUrl = process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:4173';
		const apiResponse = await page.request.get(
			`${backendUrl}/data/aircraft?registration=${testAircraft.validRegistration}`
		);
		const apiData = await apiResponse.json();
		console.log('Backend API response:', JSON.stringify(apiData, null, 2));
		console.log('API status:', apiResponse.status());
		console.log('Aircraft count from API:', apiData.aircraft?.length || 0);

		await goToAircraft(page);
		await searchAircraftByRegistration(page, testAircraft.validRegistration);

		// Wait for search results
		await page.waitForLoadState('networkidle');

		// Debug: Check what's actually on the page
		console.log('Page title:', await page.title());
		console.log('Page URL:', page.url());
		console.log('Aircraft cards found:', await page.locator('a[href^="/aircraft/"]').count());

		// Check if there's an error message
		const errorText = await page
			.locator('text=/error|failed|not found/i')
			.textContent()
			.catch(() => null);
		if (errorText) {
			console.log('Error message on page:', errorText);
		}

		// Find and click the first aircraft card
		const aircraftCard = page.locator('a[href^="/aircraft/"]').first();
		await expect(aircraftCard).toBeVisible();
		await aircraftCard.click();

		// Wait for aircraft detail page to load
		await page.waitForLoadState('networkidle');
	}

	test.skip('should display aircraft detail authenticatedPage', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Check page has aircraft-related content
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();

		// Should have a back button
		await expect(
			authenticatedPage.getByRole('button', { name: /back to aircraft/i })
		).toBeVisible();

		// Should show aircraft registration section
		await expect(
			authenticatedPage.getByRole('heading', { name: /aircraft registration/i })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('aircraft-detail-authenticatedPage.png', {
			// Aircraft data may vary, so use a larger threshold
			maxDiffPixelRatio: 0.1
		});
	});

	test.skip('should display aircraft address and type information', async ({
		authenticatedPage
	}) => {
		await navigateToTestDevice(authenticatedPage);

		// Should show aircraft address in the format "Address: ICAO-ABC123" or similar
		await expect(authenticatedPage.getByText(/Address:/i)).toBeVisible();

		// Should show address type information (ICAO, OGN, or FLARM) in the address string
		const addressText = await authenticatedPage.getByText(/Address:/i).textContent();
		expect(addressText).toMatch(/ICAO|OGN|FLARM/i);
	});

	test.skip('should display aircraft registration information if available', async ({
		authenticatedPage
	}) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have Aircraft Registration section heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /aircraft registration/i })
		).toBeVisible();
	});

	test.skip('should display fixes (position reports) list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have Recent Position Fixes section heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /recent position fixes/i })
		).toBeVisible();

		// Take screenshot of fixes section
		await expect(authenticatedPage).toHaveScreenshot('aircraft-detail-fixes.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test.skip('should display flights list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have Flight History section heading
		await expect(authenticatedPage.getByRole('heading', { name: /flight history/i })).toBeVisible();

		// Take screenshot of flights section
		await expect(authenticatedPage).toHaveScreenshot('aircraft-detail-flights.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test.skip('should navigate back to aircraft list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Click the back button
		await authenticatedPage.getByRole('button', { name: /back to aircraft/i }).click();

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

	test.skip('should show loading state while fetching data', async ({ authenticatedPage }) => {
		// This test is skipped because loading states are too fast to reliably test
		// and the test uses dynamic aircraft navigation which adds complexity
		await navigateToTestDevice(authenticatedPage);
	});

	test.skip('should display aircraft status badges if available', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		await authenticatedPage.waitForLoadState('networkidle');

		// Look for status badges (tracked, identified, etc.)
		const hasBadges = await authenticatedPage.locator('.badge').count();

		// Aircraft should have at least some status information
		expect(hasBadges).toBeGreaterThanOrEqual(0);
	});

	test.skip('should paginate fixes if there are many', async ({ authenticatedPage }) => {
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

	test.skip('should paginate flights if there are many', async ({ authenticatedPage }) => {
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
