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
		const pageContent = await authenticatedPage.content();
		expect(pageContent).not.toContain('Missing authorization token');
		expect(pageContent).not.toContain('Error: Missing authorization token');

		// Check that "Add Aircraft" button is visible
		await expect(authenticatedPage.getByRole('button', { name: /add aircraft/i })).toBeVisible();

		// Verify the watchlist loaded (should see either entries or empty state)
		const hasEntries = await authenticatedPage.locator('.grid > .card').count();
		const hasEmptyState = await authenticatedPage.getByText('Your watchlist is empty').isVisible();

		// Should have either entries or empty state visible
		expect(hasEntries > 0 || hasEmptyState).toBe(true);
	});

	test('should redirect to login when not authenticated', async ({ page }) => {
		// Navigate to watchlist page without auth
		await page.goto('/watchlist');

		// Should redirect to login page
		await page.waitForURL('**/login**', { timeout: 5000 });
		await expect(page).toHaveURL(/\/login/);
	});
});
