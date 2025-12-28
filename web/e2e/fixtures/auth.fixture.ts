import { test as base, type Page } from './worker-database.fixture';
import { login } from '../utils/auth';

/**
 * Authentication fixture for Playwright tests
 *
 * Provides pre-authenticated browser contexts for tests that require a logged-in user.
 * This extends the worker-database fixture, so tests also get:
 * - Isolated database per worker
 * - Dedicated web server per worker
 * - Auto-configured base URL
 *
 * Usage:
 * import { test } from '../fixtures/auth.fixture';
 *
 * test('my authenticated test', async ({ authenticatedPage }) => {
 *   // This page is already logged in and uses the worker's isolated database
 *   await authenticatedPage.goto('/aircraft');
 * });
 */

// Extend worker-database test with authenticated page fixture
export const test = base.extend<{
	authenticatedPage: Page;
}>({
	authenticatedPage: async ({ page }, use) => {
		// Use test user credentials from environment or defaults
		const testEmail = process.env.TEST_USER_EMAIL || 'test@example.com';
		const testPassword = process.env.TEST_USER_PASSWORD || 'testpassword123';

		// Log in before each test using this fixture
		// The login function will use the worker's base URL via the page fixture
		await login(page, testEmail, testPassword);

		// Provide the authenticated page to the test
		await use(page);

		// Cleanup after test (if needed)
		// For now, we'll just let the context close naturally
		// The worker-scoped fixtures will handle database and server cleanup
	}
});

export { expect } from './worker-database.fixture';
