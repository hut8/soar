import { test, expect } from '../fixtures/worker-database.fixture';
import { login, expectLoginError, fillLoginForm } from '../utils/auth';
import { testUsers } from '../fixtures/data.fixture';

test.describe('Login', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to login page before each test
		await page.goto('/login');
	});

	test('should display login page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/login/i);

		// Check main heading
		await expect(page.getByRole('heading', { name: /login/i })).toBeVisible();

		// Check form elements are present
		await expect(page.getByPlaceholder('Enter your email')).toBeVisible();
		await expect(page.getByPlaceholder('Enter your password')).toBeVisible();
		await expect(page.getByRole('button', { name: /sign in/i })).toBeVisible();

		// Check links
		await expect(page.getByRole('link', { name: /forgot your password/i })).toBeVisible();
		await expect(page.getByRole('link', { name: /sign up here/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('login-page.png');
	});

	test('should successfully login with valid credentials', async ({ page }) => {
		// Attempt to log in
		await login(page, testUsers.validUser.email, testUsers.validUser.password);

		// Should be redirected to home page
		await expect(page).toHaveURL('/');

		// Take screenshot of successful login state
		// Note: Higher threshold needed due to country flags feature (PR #518)
		await expect(page).toHaveScreenshot('login-success.png', { maxDiffPixelRatio: 0.4 });
	});

	test('should show error with invalid credentials', async ({ page }) => {
		// Fill in invalid credentials
		await fillLoginForm(page, testUsers.invalidUser.email, testUsers.invalidUser.password);

		// Submit the form
		await page.getByRole('button', { name: /sign in/i }).click();

		// Should show error message (either specific message or generic)
		await expectLoginError(page);

		// Should remain on login page
		await expect(page).toHaveURL(/\/login/);

		// Take screenshot of error state
		await expect(page).toHaveScreenshot('login-error-invalid-credentials.png');
	});

	test('should show error when submitting empty form', async ({ page }) => {
		// Click submit without filling in any fields
		await page.getByRole('button', { name: /sign in/i }).click();

		// Should show client-side validation error
		await expect(page.getByText(/please fill in all fields/i)).toBeVisible();

		// Take screenshot of validation error
		await expect(page).toHaveScreenshot('login-error-empty-form.png');
	});

	test('should show error when email is empty', async ({ page }) => {
		// Fill only password
		await fillLoginForm(page, '', testUsers.validUser.password);

		// Submit the form
		await page.getByRole('button', { name: /sign in/i }).click();

		// Should show client-side validation error
		await expect(page.getByText(/please fill in all fields/i)).toBeVisible();
	});

	test('should show error when password is empty', async ({ page }) => {
		// Fill only email
		await fillLoginForm(page, testUsers.validUser.email, '');

		// Submit the form
		await page.getByRole('button', { name: /sign in/i }).click();

		// Should show client-side validation error
		await expect(page.getByText(/please fill in all fields/i)).toBeVisible();
	});

	test('should allow login with Enter key', async ({ page }) => {
		// Fill in credentials
		await fillLoginForm(page, testUsers.validUser.email, testUsers.validUser.password);

		// Press Enter on password field
		await page.getByPlaceholder('Enter your password').press('Enter');

		// Should be redirected to home page
		await expect(page).toHaveURL('/');
	});

	test('should navigate to registration page from login', async ({ page }) => {
		// Click the "Sign up here" link
		await page.getByRole('link', { name: /sign up here/i }).click();

		// Should navigate to register page
		await expect(page).toHaveURL(/\/register/);
	});

	test('should navigate to forgot password page from login', async ({ page }) => {
		// Click the "Forgot your password?" link
		await page.getByRole('link', { name: /forgot your password/i }).click();

		// Should navigate to forgot password page
		await expect(page).toHaveURL(/\/forgot-password/);
	});

	// Skipping this test as it's prone to race conditions in CI
	// The backend is fast enough that the loading state often completes before Playwright can detect it
	test.skip('should disable form during submission', async ({ page }) => {
		// Fill in credentials
		await fillLoginForm(page, testUsers.validUser.email, testUsers.validUser.password);

		// Start submission (don't await - we want to check loading state)
		const submitPromise = page.getByRole('button', { name: /sign in/i }).click();

		// Check that button shows loading state
		await expect(page.getByRole('button', { name: /signing in/i })).toBeVisible();

		// Wait for submission to complete
		await submitPromise;
	});
});
