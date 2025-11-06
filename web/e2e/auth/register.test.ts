import { test, expect } from '@playwright/test';
import { testUsers } from '../fixtures/data.fixture';

test.describe('Registration', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to registration page before each test
		await page.goto('/register');
	});

	test('should display registration page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/register/i);

		// Check main heading
		await expect(page.getByRole('heading', { name: /create account/i })).toBeVisible();

		// Check form elements are present
		await expect(page.getByPlaceholder('First name')).toBeVisible();
		await expect(page.getByPlaceholder('Last name')).toBeVisible();
		await expect(page.getByPlaceholder(/email/i)).toBeVisible();
		await expect(page.getByPlaceholder('Password', { exact: true })).toBeVisible();
		await expect(page.getByPlaceholder('Confirm password')).toBeVisible();
		await expect(page.getByRole('button', { name: /create account/i })).toBeVisible();

		// Check link to login page
		await expect(page.getByRole('link', { name: /sign in/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('register-page.png');
	});

	test('should successfully register a new user', async ({ page }) => {
		// Fill in the registration form with new user data
		const timestamp = Date.now(); // Use timestamp to ensure unique email
		const uniqueEmail = `test${timestamp}@example.com`;

		await page.getByPlaceholder('First name').fill('Test');
		await page.getByPlaceholder('Last name').fill('User');
		await page.getByPlaceholder(/email/i).fill(uniqueEmail);
		await page.getByPlaceholder('Password', { exact: true }).fill('password123');
		await page.getByPlaceholder('Confirm password').fill('password123');

		// Submit the form
		await page.getByRole('button', { name: /create account/i }).click();

		// Should be redirected to login page with success message
		await expect(page).toHaveURL(/\/login/);
		await expect(page.getByText(/registration successful.*check your email/i)).toBeVisible();

		// Take screenshot of success state
		await expect(page).toHaveScreenshot('register-success.png');
	});

	test('should show error when passwords do not match', async ({ page }) => {
		// Fill in form with mismatched passwords
		await page.getByPlaceholder('First name').fill('Test');
		await page.getByPlaceholder('Last name').fill('User');
		await page.getByPlaceholder(/email/i).fill('test@example.com');
		await page.getByPlaceholder('Password', { exact: true }).fill('password123');
		await page.getByPlaceholder('Confirm password').fill('differentpassword');

		// Submit the form
		await page.getByRole('button', { name: /create account/i }).click();

		// Should show error message
		const errorDiv = page.locator('div.preset-filled-error-500');
		await expect(errorDiv).toBeVisible();
		await expect(errorDiv).toContainText(/passwords do not match/i);

		// Take screenshot of error state
		await expect(page).toHaveScreenshot('register-error-password-mismatch.png');
	});

	test('should show error when required fields are empty', async ({ page }) => {
		// Click submit without filling in any fields
		await page.getByRole('button', { name: /create account/i }).click();

		// Should show error message
		const errorDiv = page.locator('div.preset-filled-error-500');
		await expect(errorDiv).toBeVisible();
		await expect(errorDiv).toContainText(/fill in all required fields/i);

		// Take screenshot of validation error
		await expect(page).toHaveScreenshot('register-error-required-fields.png');
	});

	test('should show error when password is too short', async ({ page }) => {
		// Fill in form with short password
		await page.getByPlaceholder('First name').fill('Test');
		await page.getByPlaceholder('Last name').fill('User');
		await page.getByPlaceholder(/email/i).fill('test@example.com');
		await page.getByPlaceholder('Password', { exact: true }).fill('short');
		await page.getByPlaceholder('Confirm password').fill('short');

		// Submit the form
		await page.getByRole('button', { name: /create account/i }).click();

		// Should show error message
		const errorDiv = page.locator('div.preset-filled-error-500');
		await expect(errorDiv).toBeVisible();
		await expect(errorDiv).toContainText(/at least 8 characters/i);

		// Take screenshot of validation error
		await expect(page).toHaveScreenshot('register-error-password-short.png');
	});

	test('should show error when email already exists', async ({ page }) => {
		// Try to register with existing user email
		await page.getByPlaceholder('First name').fill('Test');
		await page.getByPlaceholder('Last name').fill('User');
		await page.getByPlaceholder(/email/i).fill(testUsers.validUser.email);
		await page.getByPlaceholder('Password', { exact: true }).fill('password123');
		await page.getByPlaceholder('Confirm password').fill('password123');

		// Submit the form
		await page.getByRole('button', { name: /create account/i }).click();

		// Should show error message about existing account
		const errorDiv = page.locator('div.preset-filled-error-500');
		await expect(errorDiv).toBeVisible();
		await expect(errorDiv).toContainText(/account.*already exists/i);

		// Take screenshot of error state
		await expect(page).toHaveScreenshot('register-error-email-exists.png');
	});

	test('should navigate to login page from registration', async ({ page }) => {
		// Click the "Sign in" link
		await page.getByRole('link', { name: /sign in/i }).click();

		// Should navigate to login page
		await expect(page).toHaveURL(/\/login/);
	});

	test('should disable form during submission', async ({ page }) => {
		// Fill in form
		const timestamp = Date.now();
		await page.getByPlaceholder('First name').fill('Test');
		await page.getByPlaceholder('Last name').fill('User');
		await page.getByPlaceholder(/email/i).fill(`test${timestamp}@example.com`);
		await page.getByPlaceholder('Password', { exact: true }).fill('password123');
		await page.getByPlaceholder('Confirm password').fill('password123');

		// Start submission (don't await - we want to check loading state)
		const submitPromise = page.getByRole('button', { name: /create account/i }).click();

		// Check that button shows loading state
		await expect(page.getByRole('button', { name: /creating account/i })).toBeVisible();

		// Wait for submission to complete
		await submitPromise;
	});
});
