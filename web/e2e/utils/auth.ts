import { type Page, expect } from '@playwright/test';

/**
 * Authentication utilities for E2E tests
 *
 * These helpers provide reusable functions for authentication flows
 * across different test files.
 */

/**
 * Log in a user with the given credentials
 *
 * @param page - Playwright page object
 * @param email - User email
 * @param password - User password
 * @returns Promise that resolves when login is complete
 *
 * @example
 * await login(page, 'test@example.com', 'password123');
 */
export async function login(page: Page, email: string, password: string): Promise<void> {
	// Navigate to login page
	await page.goto('/login');

	// Wait for page to be fully loaded
	await page.waitForLoadState('networkidle');

	// Fill in the login form
	await page.getByPlaceholder('Enter your email').fill(email);
	await page.getByPlaceholder('Enter your password').fill(password);

	// Click the Sign In button
	await page.getByRole('button', { name: /sign in/i }).click();

	// Wait for navigation to complete (redirects to home page on success)
	await page.waitForURL('/');
	await page.waitForLoadState('networkidle');

	// Wait for authentication to be fully established by checking for user button
	await page.getByRole('button', { name: 'Test' }).waitFor({ state: 'visible', timeout: 10000 });
}

/**
 * Log out the current user
 *
 * @param page - Playwright page object
 * @returns Promise that resolves when logout is complete
 *
 * @example
 * await logout(page);
 */
export async function logout(page: Page): Promise<void> {
	// Click on user name button to open menu (shows first name "Test")
	await page.getByRole('button', { name: 'Test' }).click();

	// Click "Sign out" button in the menu
	await page.getByRole('button', { name: /sign out/i }).click();

	// Wait for redirect to login page
	await page.waitForURL('/login');
}

/**
 * Check if a user is currently logged in
 *
 * @param page - Playwright page object
 * @returns Promise<boolean> indicating if user is logged in
 *
 * @example
 * const loggedIn = await isLoggedIn(page);
 */
export async function isLoggedIn(page: Page): Promise<boolean> {
	// Check for presence of authenticated content
	// This is a simple check - adjust based on your app's structure
	// For example, check if we're redirected to login when accessing a protected page
	await page.goto('/aircraft');
	const currentUrl = page.url();
	return !currentUrl.includes('/login');
}

/**
 * Assert that login failed with an error message
 *
 * @param page - Playwright page object
 * @param expectedErrorText - Expected error message (partial match)
 *
 * @example
 * await expectLoginError(page, 'Invalid email or password');
 */
export async function expectLoginError(page: Page, expectedErrorText?: string): Promise<void> {
	// Wait for error message to appear
	const errorDiv = page.locator('div.preset-filled-error-500');
	await expect(errorDiv).toBeVisible();

	if (expectedErrorText) {
		await expect(errorDiv).toContainText(expectedErrorText);
	}
}

/**
 * Fill login form without submitting
 *
 * Useful for testing validation before submission
 *
 * @param page - Playwright page object
 * @param email - User email
 * @param password - User password
 *
 * @example
 * await fillLoginForm(page, 'test@example.com', '');
 */
export async function fillLoginForm(page: Page, email: string, password: string): Promise<void> {
	await page.getByPlaceholder('Enter your email').fill(email);
	await page.getByPlaceholder('Enter your password').fill(password);
}
