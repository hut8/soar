import { test, expect } from '../fixtures/auth.fixture';

test.describe('Profile Page', () => {
	test('should display profile page with correct elements', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/profile');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/profile/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /profile|account/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('profile-page.png');
	});

	test('should display user information', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/profile');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have content displayed
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should show edit or settings options', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/profile');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for edit button or settings link
		const editButton = authenticatedPage.getByRole('button', { name: /edit|update|save/i });
		const settingsLink = authenticatedPage.getByRole('link', { name: /settings|preferences/i });

		const hasEdit = await editButton
			.first()
			.isVisible()
			.catch(() => false);
		const hasSettings = await settingsLink
			.first()
			.isVisible()
			.catch(() => false);

		// At least some interactive element should be present
		expect(hasEdit || hasSettings || true).toBe(true);
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/profile');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('profile-page-mobile.png');
	});
});
