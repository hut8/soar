import { test, expect } from '../fixtures/worker-database.fixture';
import { login, logout } from '../utils/auth';
import { testUsers, testClubs } from '../fixtures/data.fixture';

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

		// Navigate to protected page (club operations)
		const protectedPage = `/clubs/${testClubs.validClubId}/operations`;
		await page.goto(protectedPage);
		await expect(page).toHaveURL(protectedPage);

		// Log out
		await logout(page);

		// Try to access protected page again
		await page.goto(protectedPage);

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
