import { test as base, type Page } from '@playwright/test';
import { login } from '../utils/auth';

/**
 * Authentication fixture for Playwright tests
 *
 * Provides pre-authenticated browser contexts for tests that require a logged-in user.
 *
 * Usage:
 * import { test } from '../fixtures/auth.fixture';
 *
 * test('my authenticated test', async ({ authenticatedPage }) => {
 *   // This page is already logged in
 *   await authenticatedPage.goto('/aircraft');
 * });
 */

// Extend base test with authenticated page fixture
export const test = base.extend<{
	authenticatedPage: Page;
}>({
	authenticatedPage: async ({ page }, use) => {
		// TODO: Replace with actual test user credentials
		// These should ideally come from environment variables or test config
		const testEmail = process.env.TEST_USER_EMAIL || 'test@example.com';
		const testPassword = process.env.TEST_USER_PASSWORD || 'testpassword123';

		// Log in before each test using this fixture
		await login(page, testEmail, testPassword);

		// Provide the authenticated page to the test
		await use(page);

		// Cleanup after test (if needed)
		// For now, we'll just let the context close naturally
	}
});

export { expect } from '@playwright/test';
