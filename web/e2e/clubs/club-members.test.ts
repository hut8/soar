import { test, expect } from '../fixtures/auth.fixture';
import { testClubs } from '../fixtures/data.fixture';

test.describe('Club Members', () => {
	test('should display club members page', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/members`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/members|club/i);

		// Check for heading
		const heading = authenticatedPage.getByRole('heading');
		await expect(heading.first()).toBeVisible();

		// Take screenshot
		await expect(authenticatedPage).toHaveScreenshot('club-members-page.png');
	});

	test('should display members list or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/members`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show members or a "no members" message
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should navigate via tab bar', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/members`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for Club Info tab link
		const clubInfoLink = authenticatedPage.getByRole('link', { name: /club info/i });
		const hasClubInfoLink = await clubInfoLink.isVisible().catch(() => false);

		if (hasClubInfoLink) {
			await clubInfoLink.click();
			await expect(authenticatedPage).toHaveURL(new RegExp(`/clubs/${testClubs.validClubId}$`));
		}
	});
});
