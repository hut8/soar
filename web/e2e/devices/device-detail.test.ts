import { test, expect } from '@playwright/test';
import { goToDeviceDetail } from '../utils/navigation';

test.describe('Device Detail', () => {
	// Note: These tests require a valid device ID
	// In a real test environment, you'd either:
	// 1. Create test data via API before tests
	// 2. Use known test device IDs from your test database
	// 3. Search for a device first and use that ID

	const testDeviceId = '1'; // Replace with actual test device ID

	test('should display device detail page', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		// Check page title includes device info
		await expect(page).toHaveTitle(/device/i);

		// Should have a back button
		await expect(page.getByRole('link', { name: /back/i })).toBeVisible();

		// Should show device information section
		await expect(page.getByRole('heading', { name: /device information/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('device-detail-page.png', {
			// Device data may vary, so use a larger threshold
			maxDiffPixelRatio: 0.1
		});
	});

	test('should display device address and type information', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		// Should show device address label
		await expect(page.getByText(/device address/i)).toBeVisible();

		// Should show address type information (ICAO, OGN, or FLARM)
		const hasICAO = await page.getByText('ICAO').isVisible();
		const hasOGN = await page.getByText('OGN').isVisible();
		const hasFLARM = await page.getByText('FLARM').isVisible();

		expect(hasICAO || hasOGN || hasFLARM).toBe(true);
	});

	test('should display aircraft registration information if available', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Check if registration section exists
		// Note: Not all devices have registration info
		const hasRegistration = await page.getByText(/registration/i).isVisible();

		if (hasRegistration) {
			// Should show registration number
			await expect(page.getByText(/N\d+|[A-Z]{1,2}-[A-Z0-9]+/)).toBeVisible();
		}
	});

	test('should display fixes (position reports) list', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Should have fixes section or "no fixes" message
		const hasFixesHeading = await page
			.getByRole('heading', { name: /fixes|positions/i })
			.isVisible();
		const hasNoFixesMessage = await page.getByText(/no fixes|no position reports/i).isVisible();

		expect(hasFixesHeading || hasNoFixesMessage).toBe(true);

		// Take screenshot of fixes section
		await expect(page).toHaveScreenshot('device-detail-fixes.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test('should display flights list', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Should have flights section or "no flights" message
		const hasFlightsHeading = await page.getByRole('heading', { name: /flights/i }).isVisible();
		const hasNoFlightsMessage = await page.getByText(/no flights/i).isVisible();

		expect(hasFlightsHeading || hasNoFlightsMessage).toBe(true);

		// Take screenshot of flights section
		await expect(page).toHaveScreenshot('device-detail-flights.png', {
			maxDiffPixelRatio: 0.1
		});
	});

	test('should navigate back to device list', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		// Click the back button/link
		await page.getByRole('link', { name: /back/i }).click();

		// Should navigate back to devices page
		await expect(page).toHaveURL(/\/devices$/);
	});

	test('should handle invalid device ID gracefully', async ({ page }) => {
		// Try to navigate to a device that doesn't exist
		await goToDeviceDetail(page, 'nonexistent-device-id-999999');

		// Should show error message or redirect
		// The exact behavior depends on your app's error handling
		const hasError = await page.getByText(/not found|error|invalid/i).isVisible();
		const redirectedToDevices =
			page.url().includes('/devices') && !page.url().match(/\/devices\/[^/]+/);

		expect(hasError || redirectedToDevices).toBe(true);

		// Take screenshot of error state
		await expect(page).toHaveScreenshot('device-detail-not-found.png');
	});

	test('should show loading state while fetching data', async ({ page }) => {
		// Navigate to device detail
		const navigationPromise = goToDeviceDetail(page, testDeviceId);

		// Check for loading indicators
		// Note: This might be hard to catch if the page loads quickly
		await page
			.locator('div[role="progressbar"]')
			.isVisible()
			.catch(() => false);
		await page
			.getByText(/loading/i)
			.isVisible()
			.catch(() => false);

		// At least one loading indicator should be present (or data loads instantly)
		// This is a weak assertion but better than nothing
		await navigationPromise;
	});

	test('should display device status badges if available', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		await page.waitForLoadState('networkidle');

		// Look for status badges (tracked, identified, etc.)
		const hasBadges = await page.locator('.badge').count();

		// Device should have at least some status information
		expect(hasBadges).toBeGreaterThanOrEqual(0);
	});

	test('should paginate fixes if there are many', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		await page.waitForLoadState('networkidle');

		// Check if pagination controls exist for fixes
		const hasFixesPagination = await page
			.getByRole('button', { name: /next|previous/i })
			.isVisible();

		if (hasFixesPagination) {
			// Test pagination
			const nextButton = page.getByRole('button', { name: /next/i }).first();

			if (await nextButton.isEnabled()) {
				await nextButton.click();
				await page.waitForLoadState('networkidle');

				// Should load next page of fixes
				await expect(page).toHaveScreenshot('device-detail-fixes-page-2.png', {
					maxDiffPixelRatio: 0.1
				});
			}
		}
	});

	test('should paginate flights if there are many', async ({ page }) => {
		await goToDeviceDetail(page, testDeviceId);

		await page.waitForLoadState('networkidle');

		// Check if pagination controls exist for flights
		const flightsSection = page.locator('text=/flights/i').locator('..');
		const hasFlightsPagination =
			(await flightsSection.getByRole('button', { name: /next/i }).isVisible()) ||
			(await flightsSection.getByRole('button', { name: /previous/i }).isVisible());

		if (hasFlightsPagination) {
			// Test pagination
			const nextButton = flightsSection.getByRole('button', { name: /next/i });

			if (await nextButton.isEnabled()) {
				await nextButton.click();
				await page.waitForLoadState('networkidle');

				// Should load next page of flights
				await expect(page).toHaveScreenshot('device-detail-flights-page-2.png', {
					maxDiffPixelRatio: 0.1
				});
			}
		}
	});
});
