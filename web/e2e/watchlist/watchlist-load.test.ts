import { expect } from '@playwright/test';
import { test } from '../fixtures/auth.fixture';

test.describe('Watchlist', () => {
	test('should load watchlist page when authenticated', async ({ authenticatedPage }) => {
		// Navigate to watchlist page
		await authenticatedPage.goto('/watchlist');

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check that the page title is visible
		await expect(authenticatedPage.getByRole('heading', { name: 'My Watchlist' })).toBeVisible();

		// Check that we don't see authorization error
		// The main purpose of this test is to verify the authorization fix is working
		const pageText = await authenticatedPage.textContent('body');
		expect(pageText).not.toMatch(/missing authorization token/i);
		expect(pageText).not.toMatch(/error.*missing authorization token/i);

		// Wait for either "Add Aircraft" button or empty state to appear
		// This confirms the page loaded successfully
		await Promise.race([
			authenticatedPage.getByRole('button', { name: /add aircraft/i }).waitFor({ timeout: 10000 }),
			authenticatedPage.getByText('Your watchlist is empty').waitFor({ timeout: 10000 })
		]);

		// Verify we can see the expected UI elements
		// At least one of these should be visible
		const hasAddButton = await authenticatedPage
			.getByRole('button', { name: /add aircraft/i })
			.isVisible();
		const hasEmptyMessage = await authenticatedPage
			.getByText('Your watchlist is empty')
			.isVisible();

		expect(hasAddButton || hasEmptyMessage).toBe(true);
	});

	test('should redirect to login when not authenticated', async ({ page }) => {
		// Navigate to watchlist page without auth
		await page.goto('/watchlist');

		// Should redirect to login page
		await page.waitForURL('**/login**', { timeout: 5000 });
		await expect(page).toHaveURL(/\/login/);
	});
});
