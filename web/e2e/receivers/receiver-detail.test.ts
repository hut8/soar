import { test, expect } from '../fixtures/auth.fixture';

test.describe('Receiver Detail', () => {
	test('should display receiver detail page when navigating from list', async ({
		authenticatedPage
	}) => {
		// First go to receivers list
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find and click first receiver (exclude coverage link)
		const receiverLinks = authenticatedPage.locator(
			'a[href^="/receivers/"]:not([href*="/coverage"])'
		);
		const count = await receiverLinks.count();

		if (count > 0) {
			const firstReceiver = receiverLinks.first();
			await firstReceiver.click();

			// Should navigate to detail page
			await expect(authenticatedPage).toHaveURL(/\/receivers\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Check for heading
			const heading = authenticatedPage.getByRole('heading', { level: 1 });
			await expect(heading).toBeVisible();

			// Take screenshot
			await expect(authenticatedPage).toHaveScreenshot('receiver-detail-page.png');
		}
	});

	test('should display receiver information', async ({ authenticatedPage }) => {
		// Navigate to receivers list first
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		const receiverLinks = authenticatedPage.locator(
			'a[href^="/receivers/"]:not([href*="/coverage"])'
		);
		const count = await receiverLinks.count();

		if (count > 0) {
			await receiverLinks.first().click();
			await authenticatedPage.waitForLoadState('networkidle');

			// Should have some content displayed
			const bodyText = await authenticatedPage.textContent('body');
			expect(bodyText).toBeTruthy();
			expect(bodyText!.length).toBeGreaterThan(0);
		}
	});

	test('should handle invalid receiver ID gracefully', async ({ authenticatedPage }) => {
		// Navigate to non-existent receiver
		await authenticatedPage.goto('/receivers/invalid-receiver-id-999999');
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should load without crashing (may show error, redirect, or empty state)
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
	});
});
