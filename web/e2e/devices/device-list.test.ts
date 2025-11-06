import { test, expect } from '@playwright/test';
import { goToDevices, searchDevicesByRegistration } from '../utils/navigation';
import { testDevices } from '../fixtures/data.fixture';

test.describe('Device List', () => {
	test('should display device list page with correct elements', async ({ page }) => {
		await goToDevices(page);

		// Check page title
		await expect(page).toHaveTitle(/devices/i);

		// Check main heading (use level 1 to be specific to h1)
		await expect(
			page.getByRole('heading', { name: /^aircraft devices$/i, level: 1 })
		).toBeVisible();

		// Check search section is present
		await expect(
			page.getByRole('heading', { name: /search aircraft devices/i, level: 3 })
		).toBeVisible();

		// Check search input is visible (default is registration search)
		await expect(page.locator('input[placeholder*="Aircraft registration"]:visible')).toBeVisible();

		// Check search button
		await expect(page.getByRole('button', { name: /search devices/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('device-list-page.png');
	});

	test('should show search type selector with all options', async ({ page }) => {
		await goToDevices(page);

		// Check that all search type options are visible
		await expect(page.getByText('Registration')).toBeVisible();
		await expect(page.getByText('Device Address')).toBeVisible();
		await expect(page.getByText('Club')).toBeVisible();
	});

	test('should switch between search types', async ({ page }) => {
		await goToDevices(page);

		// Initially should show registration search
		await expect(page.locator('input[placeholder*="Aircraft registration"]:visible')).toBeVisible();

		// Click on Device Address search type
		await page.getByRole('button', { name: /device address/i }).click();

		// Should show device address input
		await expect(page.locator('input[placeholder="Device address"]:visible')).toBeVisible();

		// Should show address type selector (ICAO, OGN, FLARM)
		await expect(page.getByText('ICAO')).toBeVisible();
		await expect(page.getByText('OGN')).toBeVisible();
		await expect(page.getByText('FLARM')).toBeVisible();

		// Take screenshot of device address search
		await expect(page).toHaveScreenshot('device-search-type-address.png');

		// Click on Club search type
		await page.getByRole('button', { name: /^club$/i }).click();

		// Should show club selector
		// Note: The actual club selector UI may vary
		await expect(page.locator('input[placeholder*="Select a club"]:visible')).toBeVisible();

		// Take screenshot of club search
		await expect(page).toHaveScreenshot('device-search-type-club.png');
	});

	test('should search for devices by registration', async ({ page }) => {
		await goToDevices(page);

		// Fill in registration
		await page
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testDevices.validRegistration);

		// Click search
		await page.getByRole('button', { name: /search devices/i }).click();

		// Wait for results or "no devices found" message
		// We can't guarantee results will be found, but we can check the UI responds
		await page.waitForLoadState('networkidle');

		// Should show either results or "no devices found" message
		const hasResults = await page.getByText(/search results/i).isVisible();
		const hasNoResults = await page.getByText(/no devices found/i).isVisible();

		expect(hasResults || hasNoResults).toBe(true);

		// Take screenshot of results
		await expect(page).toHaveScreenshot('device-search-results.png', {
			// Use a larger threshold for screenshot comparison since results may vary
			maxDiffPixelRatio: 0.1
		});
	});

	test('should show error when searching with empty query', async ({ page }) => {
		await goToDevices(page);

		// Don't fill in any search query
		// Click search directly
		await page.getByRole('button', { name: /search devices/i }).click();

		// Should show error message
		await expect(page.getByText(/please enter a search query/i)).toBeVisible();

		// Take screenshot of validation error
		await expect(page).toHaveScreenshot('device-search-error-empty.png');
	});

	test('should handle "no devices found" gracefully', async ({ page }) => {
		await goToDevices(page);

		// Search for a registration that definitely doesn't exist
		await page
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testDevices.invalidRegistration);

		// Click search
		await page.getByRole('button', { name: /search devices/i }).click();

		// Wait for response
		await page.waitForLoadState('networkidle');

		// Should show "no devices found" message
		await expect(page.getByText(/no devices found/i)).toBeVisible();

		// Take screenshot of no results state
		await expect(page).toHaveScreenshot('device-search-no-results.png');
	});

	test('should display device cards with correct information', async ({ page }) => {
		await goToDevices(page);

		// Search for devices
		await searchDevicesByRegistration(page, testDevices.validRegistration);

		// Check if results were found
		const hasResults = await page.getByText(/search results/i).isVisible();

		if (hasResults) {
			// Should show device cards
			const deviceCards = page.locator('a[href^="/devices/"]');
			const count = await deviceCards.count();

			// Should have at least one device card
			expect(count).toBeGreaterThan(0);

			// First device card should have expected elements
			const firstCard = deviceCards.first();

			// Should show device address (in monospace font)
			await expect(firstCard.locator('.font-mono')).toBeVisible();

			// Should have icon/visual elements
			// (exact content varies, but card should be clickable)
			await expect(firstCard).toBeVisible();

			// Take screenshot of device card
			await expect(firstCard).toHaveScreenshot('device-card.png');
		}
	});

	test('should navigate to device detail when clicking a device card', async ({ page }) => {
		await goToDevices(page);

		// Search for devices
		await searchDevicesByRegistration(page, testDevices.validRegistration);

		// Check if results were found
		const hasResults = await page.getByText(/search results/i).isVisible();

		if (hasResults) {
			// Click on the first device card
			const firstCard = page.locator('a[href^="/devices/"]').first();
			await firstCard.click();

			// Should navigate to device detail page
			await expect(page).toHaveURL(/\/devices\/[^/]+/);

			// Wait for page to load
			await page.waitForLoadState('networkidle');

			// Take screenshot of device detail page
			await expect(page).toHaveScreenshot('device-detail-from-list.png');
		}
	});

	test('should show loading state during search', async ({ page }) => {
		await goToDevices(page);

		// Fill in registration
		await page
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testDevices.validRegistration);

		// Start search (don't await - we want to check loading state)
		const searchPromise = page.getByRole('button', { name: /search devices/i }).click();

		// Check for loading indicator
		// The button text changes to "Searching..."
		await expect(page.getByRole('button', { name: /searching/i })).toBeVisible();

		// Wait for search to complete
		await searchPromise;
		await page.waitForLoadState('networkidle');
	});

	test('should show pagination when results exceed page size', async ({ page }) => {
		await goToDevices(page);

		// Search for a query likely to return many results
		// Note: This depends on having sufficient test data
		await page.locator('input[placeholder*="Aircraft registration"]:visible').fill('N');
		await page.getByRole('button', { name: /search devices/i }).click();

		await page.waitForLoadState('networkidle');

		// Check if pagination controls are visible
		const hasPagination =
			(await page.getByRole('button', { name: /next/i }).isVisible()) ||
			(await page.getByRole('button', { name: /previous/i }).isVisible());

		if (hasPagination) {
			// Take screenshot of pagination
			await expect(page).toHaveScreenshot('device-list-with-pagination.png');

			// Test pagination functionality
			const nextButton = page.getByRole('button', { name: /next/i });
			if (await nextButton.isEnabled()) {
				await nextButton.click();
				await page.waitForLoadState('networkidle');

				// Should update page number
				await expect(page.getByText(/page \d+ of \d+/i)).toBeVisible();
			}
		}
	});
});
