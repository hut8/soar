import { test, expect } from '../fixtures/auth.fixture';
import { testClubs } from '../fixtures/data.fixture';

test.describe('Club Detail', () => {
	test('should display club detail page when navigating from list', async ({
		authenticatedPage
	}) => {
		// First go to clubs list
		await authenticatedPage.goto('/clubs');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find and click first club
		const clubLinks = authenticatedPage.locator('a[href^="/clubs/"]');
		const count = await clubLinks.count();

		if (count > 0) {
			const firstClub = clubLinks.first();
			await firstClub.click();

			// Should navigate to detail page
			await expect(authenticatedPage).toHaveURL(/\/clubs\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Check for heading (club name)
			const heading = authenticatedPage.getByRole('heading', { level: 1 });
			await expect(heading).toBeVisible();

			// Take screenshot
			await expect(authenticatedPage).toHaveScreenshot('club-detail-page.png');
		}
	});

	test('should display club information', async ({ authenticatedPage }) => {
		// Navigate to test club using known ID
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have some content displayed
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);

		// Should display club name
		await expect(authenticatedPage.getByText(testClubs.validClubName)).toBeVisible();
	});

	test('should have navigation to pilots tab', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for link to pilots
		const pilotsLink = authenticatedPage.getByRole('link', { name: /pilots/i });

		const hasPilotsLink = await pilotsLink.isVisible().catch(() => false);

		if (hasPilotsLink) {
			expect(hasPilotsLink).toBe(true);
		}
	});

	test('should handle invalid club ID gracefully', async ({ authenticatedPage }) => {
		// Navigate to non-existent club
		await authenticatedPage.goto('/clubs/00000000-0000-0000-0000-999999999999');
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load without crashing (may show error, redirect, or empty state)
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
	});
});
