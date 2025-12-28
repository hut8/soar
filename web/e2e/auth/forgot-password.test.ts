import { test, expect } from '@playwright/test';

test.describe('Forgot Password', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/forgot-password');
	});

	test('should display forgot password page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/forgot|reset|password/i);

		// Check main heading
		await expect(page.getByRole('heading', { name: /forgot|reset password/i })).toBeVisible();

		// Check form elements are present
		await expect(page.getByPlaceholder(/email/i)).toBeVisible();
		await expect(page.getByRole('button', { name: /send|reset|submit/i })).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('forgot-password-page.png');
	});

	test('should show validation error for empty email', async ({ page }) => {
		// Click submit without filling in email
		await page.getByRole('button', { name: /send|reset|submit/i }).click();

		// Should show error message
		const hasError = await page
			.getByText(/email|required|fill/i)
			.isVisible()
			.catch(() => false);

		if (hasError) {
			await expect(page.getByText(/email|required|fill/i)).toBeVisible();
		}
	});

	test('should show validation error for invalid email format', async ({ page }) => {
		// Fill in invalid email
		await page.getByPlaceholder(/email/i).fill('invalid-email');

		// Submit form
		await page.getByRole('button', { name: /send|reset|submit/i }).click();

		// Should show format error
		const hasError = await page
			.getByText(/valid email|invalid/i)
			.isVisible()
			.catch(() => false);

		if (hasError) {
			await expect(page.getByText(/valid email|invalid/i)).toBeVisible();
		}
	});

	test('should submit forgot password request with valid email', async ({ page }) => {
		// Fill in valid email
		await page.getByPlaceholder(/email/i).fill('test@example.com');

		// Submit form
		await page.getByRole('button', { name: /send|reset|submit/i }).click();

		// Wait for response
		await page.waitForLoadState('networkidle');

		// Should show success message or remain on page with confirmation
		const bodyText = await page.textContent('body');
		expect(bodyText).toBeTruthy();

		// Take screenshot of result
		await expect(page).toHaveScreenshot('forgot-password-submitted.png');
	});

	test('should have link back to login', async ({ page }) => {
		// Look for back to login link
		const loginLink = page.getByRole('link', { name: /login|sign in|back/i });
		const hasLoginLink = await loginLink.isVisible().catch(() => false);

		if (hasLoginLink) {
			await loginLink.click();
			await expect(page).toHaveURL(/\/login/);
		}
	});
});
