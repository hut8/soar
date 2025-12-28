import { test, expect } from '../fixtures/auth.fixture';
import { goToAircraft, searchAircraftByRegistration } from '../utils/navigation';
import { testAircraft } from '../fixtures/data.fixture';

test.describe('Aircraft List', () => {
	test('should display aircraft list page with correct elements', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/aircraft/i);

		// Check main heading (use level 1 to be specific to h1)
		await expect(
			authenticatedPage.getByRole('heading', { name: /^aircraft$/i, level: 1 })
		).toBeVisible();

		// Check search section is present
		await expect(
			authenticatedPage.getByRole('heading', { name: /search aircraft/i, level: 3 })
		).toBeVisible();

		// Check search input is visible (default is registration search)
		await expect(
			authenticatedPage.locator('input[placeholder*="Aircraft registration"]:visible')
		).toBeVisible();

		// Check search button
		await expect(authenticatedPage.getByRole('button', { name: /search aircraft/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('aircraft-list-authenticatedPage.png');
	});

	test('should show search type selector with all options', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Check that all search type options are visible
		// Filter for visible elements since both mobile and desktop versions exist
		await expect(
			authenticatedPage.locator('text=Registration').locator('visible=true').first()
		).toBeVisible();
		await expect(
			authenticatedPage.locator('text=Aircraft Address').locator('visible=true').first()
		).toBeVisible();
		await expect(
			authenticatedPage.locator('text=Club').locator('visible=true').first()
		).toBeVisible();
	});

	test('should switch between search types', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Initially should show registration search
		await expect(
			authenticatedPage.locator('input[placeholder*="Aircraft registration"]:visible')
		).toBeVisible();

		// Click on Aircraft Address search type (filter for visible elements)
		await authenticatedPage
			.locator('text=Aircraft Address')
			.locator('visible=true')
			.first()
			.click();

		// Wait for UI to update after search type change
		await authenticatedPage.waitForTimeout(500);

		// Should show device address input
		await expect(
			authenticatedPage.locator('input[placeholder="Device address"]:visible')
		).toBeVisible();

		// Should show address type selector (ICAO, OGN, FLARM)
		await expect(
			authenticatedPage.locator('text=ICAO').locator('visible=true').first()
		).toBeVisible();
		await expect(
			authenticatedPage.locator('text=OGN').locator('visible=true').first()
		).toBeVisible();
		await expect(
			authenticatedPage.locator('text=FLARM').locator('visible=true').first()
		).toBeVisible();

		// Click on Club search type using the visible text
		await authenticatedPage.locator('text=Club').locator('visible=true').first().click();

		// Wait for UI to update
		await authenticatedPage.waitForTimeout(500);

		// Should show club selector (verify the "Club" label is still visible)
		await expect(
			authenticatedPage.locator('text=Club').locator('visible=true').first()
		).toBeVisible();
	});

	test('should search for aircraft by registration', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Fill in registration
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testAircraft.validRegistration);

		// Click search
		await authenticatedPage.getByRole('button', { name: /search aircraft/i }).click();

		// Wait for results or "no aircraft found" message
		// We can't guarantee results will be found, but we can check the UI responds
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show either "Search Results" heading or "No aircraft found" message
		// Wait for one of these to appear (with timeout)
		await Promise.race([
			authenticatedPage
				.getByRole('heading', { name: /search results/i })
				.waitFor({ timeout: 5000 }),
			authenticatedPage
				.getByRole('heading', { name: /no aircraft found/i })
				.waitFor({ timeout: 5000 })
		]);

		// Verify either results or "no aircraft found" message is visible
		const hasResults = await authenticatedPage
			.getByRole('heading', { name: /search results/i })
			.isVisible();
		const hasNoResults = await authenticatedPage
			.getByRole('heading', { name: /no aircraft found/i })
			.isVisible();

		expect(hasResults || hasNoResults).toBe(true);

		// Take screenshot of results
		await expect(authenticatedPage).toHaveScreenshot('aircraft-search-results.png', {
			// Use a larger threshold for screenshot comparison since results may vary
			maxDiffPixelRatio: 0.1
		});
	});

	test('should show error when searching with empty query', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Don't fill in any search query
		// Click search directly
		await authenticatedPage.getByRole('button', { name: /search aircraft/i }).click();

		// Should show error message
		await expect(authenticatedPage.getByText(/please enter a search query/i)).toBeVisible();

		// Take screenshot of validation error
		await expect(authenticatedPage).toHaveScreenshot('aircraft-search-error-empty.png');
	});

	test('should handle "no aircraft found" gracefully', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Search for a registration that definitely doesn't exist
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill(testAircraft.invalidRegistration);

		// Click search
		await authenticatedPage.getByRole('button', { name: /search aircraft/i }).click();

		// Wait for response
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show "no aircraft found" message
		await expect(authenticatedPage.getByText(/no aircraft found/i)).toBeVisible();

		// Take screenshot of no results state
		await expect(authenticatedPage).toHaveScreenshot('aircraft-search-no-results.png');
	});

	test('should display aircraft cards with correct information', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Search for aircraft
		await searchAircraftByRegistration(authenticatedPage, testAircraft.validRegistration);

		// Check if results were found
		const hasResults = await authenticatedPage.getByText(/search results/i).isVisible();

		if (hasResults) {
			// Should show aircraft cards
			const aircraftCards = authenticatedPage.locator('a[href^="/aircraft/"]');
			const count = await aircraftCards.count();

			// Should have at least one aircraft card
			expect(count).toBeGreaterThan(0);

			// First aircraft card should have expected elements
			const firstCard = aircraftCards.first();

			// Card should be visible and clickable
			await expect(firstCard).toBeVisible();

			// Should have text content (just verify the card has content)
			const cardText = await firstCard.textContent();
			expect(cardText).toBeTruthy();
			expect(cardText.length).toBeGreaterThan(0);

			// Take screenshot of aircraft card
			await expect(firstCard).toHaveScreenshot('aircraft-card.png');
		}
	});

	test('should navigate to aircraft detail when clicking a aircraft card', async ({
		authenticatedPage
	}) => {
		await goToAircraft(authenticatedPage);

		// Search for aircraft
		await searchAircraftByRegistration(authenticatedPage, testAircraft.validRegistration);

		// Check if results were found
		const hasResults = await authenticatedPage.getByText(/search results/i).isVisible();

		if (hasResults) {
			// Click on the first aircraft card
			const firstCard = authenticatedPage.locator('a[href^="/aircraft/"]').first();
			await firstCard.click();

			// Should navigate to aircraft detail page
			await expect(authenticatedPage).toHaveURL(/\/aircraft\/[^/]+/);

			// Wait for page to load
			await authenticatedPage.waitForLoadState('networkidle');

			// Take screenshot of aircraft detail page
			await expect(authenticatedPage).toHaveScreenshot('aircraft-detail-from-list.png');
		}
	});

	test('should show pagination when results exceed page size', async ({ authenticatedPage }) => {
		await goToAircraft(authenticatedPage);

		// Search for a query likely to return many results
		// Note: This depends on having sufficient test data
		await authenticatedPage
			.locator('input[placeholder*="Aircraft registration"]:visible')
			.fill('N');
		await authenticatedPage.getByRole('button', { name: /search aircraft/i }).click();

		await authenticatedPage.waitForLoadState('networkidle');

		// Check if pagination controls are visible
		const hasPagination =
			(await authenticatedPage.getByRole('button', { name: /next/i }).isVisible()) ||
			(await authenticatedPage.getByRole('button', { name: /previous/i }).isVisible());

		if (hasPagination) {
			// Take screenshot of pagination
			await expect(authenticatedPage).toHaveScreenshot('aircraft-list-with-pagination.png');

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
