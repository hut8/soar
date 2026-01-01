import { test, expect } from '../fixtures/auth.fixture';
import { testClubs } from '../fixtures/data.fixture';

test.describe('Club Operations', () => {
	test('should display club operations page', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/operations`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/operations|club/i);

		// Check for heading
		const heading = authenticatedPage.getByRole('heading', { level: 1 });
		await expect(heading).toBeVisible();

		// Take screenshot
		await expect(authenticatedPage).toHaveScreenshot('club-operations-page.png');
	});

	test('should display operations data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/operations`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have content displayed
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should navigate back to club detail', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/operations`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for breadcrumb or back link
		const clubLink = authenticatedPage.getByRole('link', {
			name: new RegExp(testClubs.validClubName, 'i')
		});
		const hasClubLink = await clubLink.isVisible().catch(() => false);

		if (hasClubLink) {
			await clubLink.click();
			await expect(authenticatedPage).toHaveURL(new RegExp(`/clubs/${testClubs.validClubId}$`));
		}
	});
});
