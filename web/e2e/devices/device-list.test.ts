import { test, expect } from '../fixtures/auth.fixture';
import { goToDevices, searchDevicesByRegistration } from '../utils/navigation';
import { testDevices } from '../fixtures/data.fixture';

test.describe('Device List', () => {
	test('should display device list page with correct elements', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/devices/i);

		// Check main heading (use level 1 to be specific to h1)
		await expect(
			authenticatedPage.getByRole('heading', { name: /^aircraft devices$/i, level: 1 })
		).toBeVisible();

		// Check search section is present
		await expect(
			authenticatedPage.getByRole('heading', { name: /search aircraft devices/i, level: 3 })
		).toBeVisible();

		// Check search input is visible (default is registration search)
		await expect(
			authenticatedPage.locator('input[placeholder*="Aircraft registration"]:visible')
		).toBeVisible();

		// Check search button
		await expect(authenticatedPage.getByRole('button', { name: /search devices/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('device-list-authenticatedPage.png');
	});

	test('should show search type selector with all options', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Check that all search type options are visible
		// Filter for visible elements since both mobile and desktop versions exist
		await expect(
			authenticatedPage.locator('text=Registration').locator('visible=true').first()
		).toBeVisible();
		await expect(
			authenticatedPage.locator('text=Device Address').locator('visible=true').first()
		).toBeVisible();
		await expect(
			authenticatedPage.locator('text=Club').locator('visible=true').first()
		).toBeVisible();
	});

	test('should switch between search types', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Initially should show registration search
		await expect(
			authenticatedPage.locator('input[placeholder*="Aircraft registration"]:visible')
		).toBeVisible();

		// Click on Device Address search type (filter for visible elements)
		await authenticatedPage.locator('text=Device Address').locator('visible=true').first().click();

		// Should show device address input
		await expect(
			authenticatedPage.locator('input[placeholder="Device address"]:visible')
		).toBeVisible();

		// Should show address type selector (ICAO, OGN, FLARM)
		await expect(authenticatedPage.getByText('ICAO')).toBeVisible();
		await expect(authenticatedPage.getByText('OGN')).toBeVisible();
		await expect(authenticatedPage.getByText('FLARM')).toBeVisible();

		// Take screenshot of device address search
		await expect(authenticatedPage).toHaveScreenshot('device-search-type-address.png');

		// Click on Club search type (filter for visible elements)
		await authenticatedPage.locator('text=Club').locator('visible=true').first().click();

		// Should show club selector
		// Note: The actual club selector UI may vary
		await expect(
			authenticatedPage.locator('input[placeholder*="Select a club"]:visible')
		).toBeVisible();

		// Take screenshot of club search
		await expect(authenticatedPage).toHaveScreenshot('device-search-type-club.png');
	});

	test('should search for devices by registration', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Fill in registration
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testDevices.validRegistration);

		// Click search
		await authenticatedPage.getByRole('button', { name: /search devices/i }).click();

		// Wait for results or "no devices found" message
		// We can't guarantee results will be found, but we can check the UI responds
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show either results or "no devices found" message
		const hasResults = await authenticatedPage.getByText(/search results/i).isVisible();
		const hasNoResults = await authenticatedPage.getByText(/no devices found/i).isVisible();

		expect(hasResults || hasNoResults).toBe(true);

		// Take screenshot of results
		await expect(authenticatedPage).toHaveScreenshot('device-search-results.png', {
			// Use a larger threshold for screenshot comparison since results may vary
			maxDiffPixelRatio: 0.1
		});
	});

	test('should show error when searching with empty query', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Don't fill in any search query
		// Click search directly
		await authenticatedPage.getByRole('button', { name: /search devices/i }).click();

		// Should show error message
		await expect(authenticatedPage.getByText(/please enter a search query/i)).toBeVisible();

		// Take screenshot of validation error
		await expect(authenticatedPage).toHaveScreenshot('device-search-error-empty.png');
	});

	test('should handle "no devices found" gracefully', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Search for a registration that definitely doesn't exist
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testDevices.invalidRegistration);

		// Click search
		await authenticatedPage.getByRole('button', { name: /search devices/i }).click();

		// Wait for response
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show "no devices found" message
		await expect(authenticatedPage.getByText(/no devices found/i)).toBeVisible();

		// Take screenshot of no results state
		await expect(authenticatedPage).toHaveScreenshot('device-search-no-results.png');
	});

	test('should display device cards with correct information', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Search for devices
		await searchDevicesByRegistration(authenticatedPage, testDevices.validRegistration);

		// Check if results were found
		const hasResults = await authenticatedPage.getByText(/search results/i).isVisible();

		if (hasResults) {
			// Should show device cards
			const deviceCards = authenticatedPage.locator('a[href^="/devices/"]');
			const count = await deviceCards.count();

			// Should have at least one device card
			expect(count).toBeGreaterThan(0);

			// First device card should have expected elements
			const firstCard = deviceCards.first();

			// Card should be visible and clickable
			await expect(firstCard).toBeVisible();

			// Should have text content (just verify the card has content)
			const cardText = await firstCard.textContent();
			expect(cardText).toBeTruthy();
			expect(cardText.length).toBeGreaterThan(0);

			// Take screenshot of device card
			await expect(firstCard).toHaveScreenshot('device-card.png');
		}
	});

	test('should navigate to device detail when clicking a device card', async ({
		authenticatedPage
	}) => {
		await goToDevices(authenticatedPage);

		// Search for devices
		await searchDevicesByRegistration(authenticatedPage, testDevices.validRegistration);

		// Check if results were found
		const hasResults = await authenticatedPage.getByText(/search results/i).isVisible();

		if (hasResults) {
			// Click on the first device card
			const firstCard = authenticatedPage.locator('a[href^="/devices/"]').first();
			await firstCard.click();

			// Should navigate to device detail page
			await expect(authenticatedPage).toHaveURL(/\/devices\/[^/]+/);

			// Wait for page to load
			await authenticatedPage.waitForLoadState('networkidle');

			// Take screenshot of device detail page
			await expect(authenticatedPage).toHaveScreenshot('device-detail-from-list.png');
		}
	});

	// Skipping this test as it's prone to race conditions in CI
	// The backend is fast enough that the loading state often completes before Playwright can detect it
	test.skip('should show loading state during search', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Fill in registration
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testDevices.validRegistration);

		// Start search (don't await - we want to check loading state)
		const searchPromise = authenticatedPage
			.getByRole('button', { name: /search devices/i })
			.click();

		// Check for loading indicator
		// The button text changes to "Searching..."
		await expect(authenticatedPage.getByRole('button', { name: /searching/i })).toBeVisible();

		// Wait for search to complete
		await searchPromise;
		await authenticatedPage.waitForLoadState('networkidle');
	});

	test('should show pagination when results exceed page size', async ({ authenticatedPage }) => {
		await goToDevices(authenticatedPage);

		// Search for a query likely to return many results
		// Note: This depends on having sufficient test data
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill('N');
		await authenticatedPage.getByRole('button', { name: /search devices/i }).click();

		await authenticatedPage.waitForLoadState('networkidle');

		// Check if pagination controls are visible
		const hasPagination =
			(await authenticatedPage.getByRole('button', { name: /next/i }).isVisible()) ||
			(await authenticatedPage.getByRole('button', { name: /previous/i }).isVisible());

		if (hasPagination) {
			// Take screenshot of pagination
			await expect(authenticatedPage).toHaveScreenshot('device-list-with-pagination.png');

			// Test pagination functionality
			const nextButton = authenticatedPage.getByRole('button', { name: /next/i });
			if (await nextButton.isEnabled()) {
				await nextButton.click();
				await authenticatedPage.waitForLoadState('networkidle');

				// Should update page number
				await expect(authenticatedPage.getByText(/page \d+ of \d+/i)).toBeVisible();
			}
		}
	});
});
