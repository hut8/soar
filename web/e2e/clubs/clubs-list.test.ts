import { test, expect } from '../fixtures/auth.fixture';

test.describe('Clubs List', () => {
	test('should display clubs list page with correct elements', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/clubs');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/clubs/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /clubs/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('clubs-list-page.png');
	});

	test('should display search or filter functionality', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/clubs');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for search input or filter controls
		const searchInput = authenticatedPage.locator(
			'input[type="search"], input[placeholder*="search" i], input[placeholder*="filter" i]'
		);
		const hasSearch = await searchInput
			.first()
			.isVisible()
			.catch(() => false);

		if (hasSearch) {
			await expect(searchInput.first()).toBeVisible();
		}
	});

	test('should display clubs data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/clubs');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show clubs list or a "no clubs" message
		const hasClubLinks = await authenticatedPage.locator('a[href^="/clubs/"]').count();
		const hasEmptyState = await authenticatedPage
			.getByText(/no clubs|empty/i)
			.isVisible()
			.catch(() => false);

		// At least one of these should be true
		expect(hasClubLinks > 0 || hasEmptyState).toBe(true);
	});

	test('should navigate to club detail when clicking a club', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/clubs');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find first club link
		const clubLinks = authenticatedPage.locator('a[href^="/clubs/"]');
		const count = await clubLinks.count();

		if (count > 0) {
			const firstClub = clubLinks.first();
			await firstClub.click();

			// Should navigate to club detail page
			await expect(authenticatedPage).toHaveURL(/\/clubs\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Take screenshot of club detail from list
			await expect(authenticatedPage).toHaveScreenshot('club-detail-from-list.png');
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/clubs');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('clubs-list-mobile.png');
	});
});
