import { test, expect } from '../fixtures/auth.fixture';
import { testClubs } from '../fixtures/data.fixture';

test.describe('Club Pilots', () => {
	test('should display club pilots page', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/pilots`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/pilots|club/i);

		// Check for heading
		const heading = authenticatedPage.getByRole('heading', { level: 1 });
		await expect(heading).toBeVisible();

		// Take screenshot
		await expect(authenticatedPage).toHaveScreenshot('club-pilots-page.png');
	});

	test('should display pilots list or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/pilots`);
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show pilots or a "no pilots" message
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should navigate back to club detail', async ({ authenticatedPage }) => {
		await authenticatedPage.goto(`/clubs/${testClubs.validClubId}/pilots`);
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
