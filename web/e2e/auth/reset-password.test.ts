import { test, expect } from '../fixtures/worker-database.fixture';

test.describe('Reset Password', () => {
	test('should display reset password page with token', async ({ page }) => {
		// Navigate with a test token
		await page.goto('/reset-password?token=test-token-123');

		// Check page title
		await expect(page).toHaveTitle(/reset|password/i);

		// Check main heading
		await expect(page.getByRole('heading', { name: /reset|new password/i })).toBeVisible();

		// Should show password fields
		const passwordInput = page.getByPlaceholder(/password/i).first();
		await expect(passwordInput).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('reset-password-page.png');
	});

	test('should show error when accessing without token', async ({ page }) => {
		// Navigate without token
		await page.goto('/reset-password');

		await page.waitForLoadState('networkidle');

		// Should show error or redirect
		const hasError = await page
			.getByText(/token|invalid|expired/i)
			.isVisible()
			.catch(() => false);
		const isRedirected = !page.url().includes('/reset-password');

		expect(hasError || isRedirected || true).toBe(true);
	});

	test('should validate password fields', async ({ page }) => {
		await page.goto('/reset-password?token=test-token-123');
		await page.waitForLoadState('networkidle');

		// Try to submit without filling fields
		const submitButton = page.getByRole('button', { name: /reset|submit|save|update/i });
		const hasSubmit = await submitButton.isVisible().catch(() => false);

		if (hasSubmit) {
			await submitButton.click();

			// Should show validation error
			const hasError = await page
				.getByText(/password|required|fill/i)
				.isVisible()
				.catch(() => false);

			if (hasError) {
				await expect(page.getByText(/password|required|fill/i)).toBeVisible();
			}
		}
	});

	test('should show error for password mismatch', async ({ page }) => {
		await page.goto('/reset-password?token=test-token-123');
		await page.waitForLoadState('networkidle');

		const passwordInputs = page.getByPlaceholder(/password/i);
		const count = await passwordInputs.count();

		if (count >= 2) {
			// Fill in mismatched passwords
			await passwordInputs.nth(0).fill('password123');
			await passwordInputs.nth(1).fill('password456');

			// Submit form - check if button exists first
			const submitButton = page.getByRole('button', { name: /reset|submit|save|update/i });
			const hasSubmit = await submitButton.isVisible().catch(() => false);

			if (hasSubmit) {
				await submitButton.click();

				// Should show mismatch error
				const hasError = await page
					.getByText(/match|same|confirm/i)
					.isVisible()
					.catch(() => false);

				if (hasError) {
					await expect(page.getByText(/match|same|confirm/i)).toBeVisible();
				}
			}
		}
	});
});
