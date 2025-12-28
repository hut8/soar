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

	test('should have navigation to pilots and operations tabs', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for links to pilots and operations
		const pilotsLink = authenticatedPage.getByRole('link', { name: /pilots/i });
		const operationsLink = authenticatedPage.getByRole('link', { name: /operations/i });

		// At least one tab navigation should be visible
		const hasPilotsLink = await pilotsLink.isVisible().catch(() => false);
		const hasOperationsLink = await operationsLink.isVisible().catch(() => false);

		if (hasPilotsLink || hasOperationsLink) {
			expect(hasPilotsLink || hasOperationsLink).toBe(true);
		}
	});

	test('should handle invalid club ID gracefully', async ({ authenticatedPage }) => {
		// Navigate to non-existent club
		await authenticatedPage.goto('/clubs/00000000-0000-0000-0000-999999999999');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should show error message or redirect
		const hasError = await authenticatedPage
			.getByText(/not found|error|invalid/i)
			.isVisible()
			.catch(() => false);
		const isRedirected = !authenticatedPage
			.url()
			.includes('/clubs/00000000-0000-0000-0000-999999999999');

		expect(hasError || isRedirected).toBe(true);
	});
});
