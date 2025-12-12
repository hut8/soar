import { test, expect } from '@playwright/test';
import { login, logout } from '../utils/auth';
import { testUsers } from '../fixtures/data.fixture';

test.describe('Logout', () => {
	test('should successfully logout user', async ({ page }) => {
		// First, log in
		await login(page, testUsers.validUser.email, testUsers.validUser.password);

		// Verify we're logged in (on home page)
		await expect(page).toHaveURL('/');

		// Log out
		await logout(page);

		// Should be redirected to login page
		await expect(page).toHaveURL('/login');

		// Take screenshot of logout success
		await expect(page).toHaveScreenshot('logout-success.png');
	});

	test('should redirect to login when accessing protected page after logout', async ({ page }) => {
		// Log in
		await login(page, testUsers.validUser.email, testUsers.validUser.password);

		// Navigate to protected page (aircraft)
		await page.goto('/aircraft');
		await expect(page).toHaveURL('/aircraft');

		// Log out
		await logout(page);

		// Try to access protected page again
		await page.goto('/aircraft');

		// Should be redirected to login page
		await expect(page).toHaveURL(/\/login/);
	});

	test('should show user menu when clicking on user name', async ({ page }) => {
		// Log in
		await login(page, testUsers.validUser.email, testUsers.validUser.password);

		// Click on user button to open menu (shows first name "Test")
		await page.getByRole('button', { name: 'Test' }).click();

		// Menu should be visible with "Sign out" button
		await expect(page.getByRole('button', { name: /sign out/i })).toBeVisible();

		// Take screenshot of user menu
		await expect(page).toHaveScreenshot('user-menu-open.png');
	});
});
