import { expect } from '../fixtures/worker-database.fixture';
import { test } from '../fixtures/auth.fixture';

test.describe('Watchlist', () => {
	test('should load watchlist page when authenticated without authorization errors', async ({
		authenticatedPage
	}) => {
		// Navigate to watchlist page
		await authenticatedPage.goto('/watchlist');

		// Wait for the page title to be visible - this confirms page loaded
		const heading = authenticatedPage.getByRole('heading', { name: 'My Watchlist', exact: true });
		await expect(heading).toBeVisible({ timeout: 10000 });

		// Wait for loading state to complete
		// The loading text should either not exist or not be visible
		await expect(authenticatedPage.getByText('Loading watchlist...', { exact: true })).toHaveCount(
			0,
			{ timeout: 10000 }
		);

		// Verify no authorization error is present
		// If the bug exists, we would see "Error: Missing authorization token"
		await expect(
			authenticatedPage.getByText(/error.*missing authorization token/i)
		).not.toBeVisible();

		// Also check the error card doesn't exist
		const errorCard = authenticatedPage.locator('.variant-ghost-error');
		await expect(errorCard).not.toBeVisible();

		// The page should now be in either empty or populated state
		// Both states should have the "Add Aircraft" button in the header
		const addButton = authenticatedPage.getByRole('button', { name: 'Add Aircraft' }).first();
		await expect(addButton).toBeVisible();

		// Verify we can interact with the page (clicking Add Aircraft should open modal)
		await addButton.click();

		// The modal should appear - verify by checking for modal close button or heading
		const modalHeading = authenticatedPage.getByRole('heading', { name: /add.*aircraft/i });
		await expect(modalHeading).toBeVisible({ timeout: 5000 });
	});

	test('should redirect to login when not authenticated', async ({ page }) => {
		// Navigate to watchlist page without auth
		await page.goto('/watchlist');

		// Should redirect to login page
		await page.waitForURL('**/login**', { timeout: 5000 });
		await expect(page).toHaveURL(/\/login/);
	});
});
