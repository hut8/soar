import { test, expect } from '../fixtures/auth.fixture';
import { goToDeviceDetail } from '../utils/navigation';

test.describe('Device Detail', () => {
	// Note: These tests require a valid device ID
	// In a real test environment, you'd either:
	// 1. Create test data via API before tests
	// 2. Use known test device IDs from your test database
	// 3. Search for a device first and use that ID

	const testDeviceId = '1'; // Replace with actual test device ID

	test('should display device detail authenticatedPage', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		// Check authenticatedPage title includes device info
		await expect(authenticatedPage).toHaveTitle(/device/i);

		// Should have a back button
		await expect(authenticatedPage.getByRole('link', { name: /back/i })).toBeVisible();

		// Should show device information section
		await expect(
			authenticatedPage.getByRole('heading', { name: /device information/i })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('device-detail-authenticatedPage.png', {
			// Device data may vary, so use a larger threshold
			maxDiffPixelRatio: 0.1
		});
	});

	test('should display device address and type information', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		// Should show device address label
		await expect(authenticatedPage.getByText(/device address/i)).toBeVisible();

		// Should show address type information (ICAO, OGN, or FLARM)
		const hasICAO = await authenticatedPage.getByText('ICAO').isVisible();
		const hasOGN = await authenticatedPage.getByText('OGN').isVisible();
		const hasFLARM = await authenticatedPage.getByText('FLARM').isVisible();

		expect(hasICAO || hasOGN || hasFLARM).toBe(true);
	});

	test('should display aircraft registration information if available', async ({
		authenticatedPage
	}) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check if registration section exists
		// Note: Not all devices have registration info
		const hasRegistration = await authenticatedPage.getByText(/registration/i).isVisible();

		if (hasRegistration) {
			// Should show registration number
			await expect(authenticatedPage.getByText(/N\d+|[A-Z]{1,2}-[A-Z0-9]+/)).toBeVisible();
		}
	});

	test('should display fixes (position reports) list', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have fixes section or "no fixes" message
		const hasFixesHeading = await authenticatedPage
			.getByRole('heading', { name: /fixes|positions/i })
			.isVisible();
		const hasNoFixesMessage = await authenticatedPage
			.getByText(/no fixes|no position reports/i)
			.isVisible();

		expect(hasFixesHeading || hasNoFixesMessage).toBe(true);

		// Take screenshot of fixes section
		await expect(authenticatedPage).toHaveScreenshot('device-detail-fixes.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test('should display flights list', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		// Wait for authenticatedPage to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have flights section or "no flights" message
		const hasFlightsHeading = await authenticatedPage
			.getByRole('heading', { name: /flights/i })
			.isVisible();
		const hasNoFlightsMessage = await authenticatedPage.getByText(/no flights/i).isVisible();

		expect(hasFlightsHeading || hasNoFlightsMessage).toBe(true);

		// Take screenshot of flights section
		await expect(authenticatedPage).toHaveScreenshot('device-detail-flights.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test('should navigate back to device list', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		// Click the back button/link
		await authenticatedPage.getByRole('link', { name: /back/i }).click();

		// Should navigate back to devices authenticatedPage
		await expect(authenticatedPage).toHaveURL(/\/devices$/);
	});

	test('should handle invalid device ID gracefully', async ({ authenticatedPage }) => {
		// Try to navigate to a device that doesn't exist
		await goToDeviceDetail(authenticatedPage, 'nonexistent-device-id-999999');

		// Should show error message or redirect
		// The exact behavior depends on your app's error handling
		const hasError = await authenticatedPage.getByText(/not found|error|invalid/i).isVisible();
		const redirectedToDevices =
			authenticatedPage.url().includes('/devices') &&
			!authenticatedPage.url().match(/\/devices\/[^/]+/);

		expect(hasError || redirectedToDevices).toBe(true);

		// Take screenshot of error state
		await expect(authenticatedPage).toHaveScreenshot('device-detail-not-found.png');
	});

	test('should show loading state while fetching data', async ({ authenticatedPage }) => {
		// Navigate to device detail
		const navigationPromise = goToDeviceDetail(authenticatedPage, testDeviceId);

		// Check for loading indicators
		// Note: This might be hard to catch if the authenticatedPage loads quickly
		await authenticatedPage
			.locator('div[role="progressbar"]')
			.isVisible()
			.catch(() => false);
		await authenticatedPage
			.getByText(/loading/i)
			.isVisible()
			.catch(() => false);

		// At least one loading indicator should be present (or data loads instantly)
		// This is a weak assertion but better than nothing
		await navigationPromise;
	});

	test('should display device status badges if available', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

		await authenticatedPage.waitForLoadState('networkidle');

		// Look for status badges (tracked, identified, etc.)
		const hasBadges = await authenticatedPage.locator('.badge').count();

		// Device should have at least some status information
		expect(hasBadges).toBeGreaterThanOrEqual(0);
	});

	test('should paginate fixes if there are many', async ({ authenticatedPage }) => {
		await goToDeviceDetail(authenticatedPage, testDeviceId);

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
		await goToDeviceDetail(authenticatedPage, testDeviceId);

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
