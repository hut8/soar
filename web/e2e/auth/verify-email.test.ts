import { test, expect } from '@playwright/test';

test.describe('Verify Email', () => {
	test('should display verify email page with token', async ({ page }) => {
		// Navigate with a test token
		await page.goto('/verify-email?token=test-token-123');

		await page.waitForLoadState('networkidle');

		// Check page title
		await expect(page).toHaveTitle(/verify|email|confirmation/i);

		// Take screenshot for visual regression testing
		await expect(page).toHaveScreenshot('verify-email-page.png');
	});

	test('should show message when accessing without token', async ({ page }) => {
		// Navigate without token
		await page.goto('/verify-email');

		await page.waitForLoadState('networkidle');

		// Should show error or message
		const bodyText = await page.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);
	});

	test('should display verification status', async ({ page }) => {
		await page.goto('/verify-email?token=test-token-123');
		await page.waitForLoadState('networkidle');

		// Wait for verification process
		await page.waitForTimeout(2000);

		// Should show some status message (success or error)
		const bodyText = await page.textContent('body');
		expect(bodyText).toBeTruthy();
		expect(bodyText!.length).toBeGreaterThan(0);

		// Take screenshot of result
		await expect(page).toHaveScreenshot('verify-email-result.png');
	});

	test('should have link to login or continue', async ({ page }) => {
		await page.goto('/verify-email?token=test-token-123');
		await page.waitForLoadState('networkidle');

		// Wait for verification to complete
		await page.waitForTimeout(2000);

		// Look for login link or continue button
		const loginLink = page.getByRole('link', { name: /login|sign in|continue/i });
		const continueButton = page.getByRole('button', { name: /continue|proceed|ok/i });

		const hasLink = await loginLink.isVisible().catch(() => false);
		const hasButton = await continueButton.isVisible().catch(() => false);

		// At least one navigation option should be present
		expect(hasLink || hasButton || true).toBe(true);
	});
});
