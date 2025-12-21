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

		// Wait for loading to complete - check that "Loading watchlist..." is not present
		// or wait for either the empty state or the entries grid to appear
		await expect(authenticatedPage.getByText('Loading watchlist...')).not.toBeVisible({
			timeout: 10000
		});

		// Check that we don't see authorization error
		// If the authorization fix is working, we shouldn't see this error
		await expect(authenticatedPage.getByText(/missing authorization token/i)).not.toBeVisible();

		// Check that "Add Aircraft" button is visible (appears in both empty and filled states)
		await expect(authenticatedPage.getByRole('button', { name: /add aircraft/i })).toBeVisible();

		// Verify the watchlist loaded successfully (should see either entries or empty state)
		// Check for either the grid container or the empty state message
		const hasEmptyState = await authenticatedPage
			.getByText('Your watchlist is empty')
			.isVisible()
			.catch(() => false);
		const hasGrid = await authenticatedPage
			.locator('.grid')
			.isVisible()
			.catch(() => false);

		// Should have either entries grid or empty state visible
		expect(hasGrid || hasEmptyState).toBe(true);
	});

	test('should redirect to login when not authenticated', async ({ page }) => {
		// Navigate to watchlist page without auth
		await page.goto('/watchlist');

		// Should redirect to login page
		await page.waitForURL('**/login**', { timeout: 5000 });
		await expect(page).toHaveURL(/\/login/);
	});
});
