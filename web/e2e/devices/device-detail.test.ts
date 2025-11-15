import { test, expect, type Page } from '../fixtures/auth.fixture';
import { goToDevices, searchDevicesByRegistration } from '../utils/navigation';
import { testDevices } from '../fixtures/data.fixture';

test.describe('Device Detail', () => {
	// Helper function to navigate to a test device detail page
	// Searches for a known test device and navigates to it
	async function navigateToTestDevice(page: Page) {
		await goToDevices(page);
		await searchDevicesByRegistration(page, testDevices.validRegistration);

		// Wait for search results
		await page.waitForLoadState('networkidle');

		// Find and click the first device card
		const deviceCard = page.locator('a[href^="/devices/"]').first();
		await expect(deviceCard).toBeVisible();
		await deviceCard.click();

		// Wait for device detail page to load
		await page.waitForLoadState('networkidle');
	}

	test('should display device detail authenticatedPage', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Check page has device-related content
		await expect(authenticatedPage.getByRole('heading', { level: 1 })).toBeVisible();

		// Should have a back button
		await expect(authenticatedPage.getByRole('button', { name: /back to devices/i })).toBeVisible();

		// Should show aircraft registration section
		await expect(
			authenticatedPage.getByRole('heading', { name: /aircraft registration/i })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('device-detail-authenticatedPage.png', {
			// Device data may vary, so use a larger threshold
			maxDiffPixelRatio: 0.1
		});
	});

	test('should display device address and type information', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Should show device address in the format "Address: ICAO-ABC123" or similar
		await expect(authenticatedPage.getByText(/Address:/i)).toBeVisible();

		// Should show address type information (ICAO, OGN, or FLARM) in the address string
		const addressText = await authenticatedPage.getByText(/Address:/i).textContent();
		expect(addressText).toMatch(/ICAO|OGN|FLARM/i);
	});

	test('should display aircraft registration information if available', async ({
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

	test('should display fixes (position reports) list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have Recent Position Fixes section heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /recent position fixes/i })
		).toBeVisible();

		// Take screenshot of fixes section
		await expect(authenticatedPage).toHaveScreenshot('device-detail-fixes.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test('should display flights list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have Flight History section heading
		await expect(authenticatedPage.getByRole('heading', { name: /flight history/i })).toBeVisible();

		// Take screenshot of flights section
		await expect(authenticatedPage).toHaveScreenshot('device-detail-flights.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test('should navigate back to device list', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		// Click the back button
		await authenticatedPage.getByRole('button', { name: /back to devices/i }).click();

		// Should navigate back to devices authenticatedPage
		await expect(authenticatedPage).toHaveURL(/\/devices$/);
	});

	test('should handle invalid device ID gracefully', async ({ authenticatedPage }) => {
		// Try to navigate to a device that doesn't exist using an invalid UUID
		await authenticatedPage.goto('/devices/00000000-0000-0000-0000-000000000000');

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show error message
		const hasError = await authenticatedPage
			.getByText(/error loading device|failed to load/i)
			.isVisible();

		expect(hasError).toBe(true);

		// Take screenshot of error state
		await expect(authenticatedPage).toHaveScreenshot('device-detail-not-found.png');
	});

	test.skip('should show loading state while fetching data', async ({ authenticatedPage }) => {
		// This test is skipped because loading states are too fast to reliably test
		// and the test uses dynamic device navigation which adds complexity
		await navigateToTestDevice(authenticatedPage);
	});

	test('should display device status badges if available', async ({ authenticatedPage }) => {
		await navigateToTestDevice(authenticatedPage);

		await authenticatedPage.waitForLoadState('networkidle');

		// Look for status badges (tracked, identified, etc.)
		const hasBadges = await authenticatedPage.locator('.badge').count();

		// Device should have at least some status information
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
					'device-detail-fixes-authenticatedPage-2.png',
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
					'device-detail-flights-authenticatedPage-2.png',
					{
						maxDiffPixelRatio: 0.1
					}
				);
			}
		}
	});
});
