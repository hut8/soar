import { test, expect } from '@playwright/test';
import { testUsers } from '../fixtures/data.fixture';

interface MailpitRecipient {
	Address: string;
	Name?: string;
}

interface MailpitMessage {
	ID: string;
	To: MailpitRecipient[];
	Subject: string;
	Text: string;
}

interface MailpitMessagesResponse {
	messages?: MailpitMessage[];
}

/**
 * Helper function to query Mailpit API for emails sent to a specific address
 * @param email - The recipient email address to search for
 * @returns The most recent email message or null if not found
 */
async function getLatestEmailFromMailpit(email: string): Promise<MailpitMessage | null> {
	// Always use localhost - Mailpit container exposes port 8025
	const mailpitUrl = 'http://localhost:8025';
	const response = await fetch(`${mailpitUrl}/api/v1/messages?limit=50`);

	if (!response.ok) {
		throw new Error(`Mailpit API returned ${response.status}: ${await response.text()}`);
	}

	const data = (await response.json()) as MailpitMessagesResponse;

	// Find the most recent email sent to this address
	const message = data.messages?.find((msg) =>
		msg.To?.some((recipient) => recipient.Address === email)
	);

	if (!message) {
		return null;
	}

	// Fetch the full message details
	const messageResponse = await fetch(`${mailpitUrl}/api/v1/message/${message.ID}`);
	if (!messageResponse.ok) {
		throw new Error(`Mailpit message API returned ${messageResponse.status}`);
	}

	return (await messageResponse.json()) as MailpitMessage;
}

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

	test.skip('should successfully register a new user', async ({ page }) => {
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

		// Wait a bit for the API call to complete
		await page.waitForTimeout(2000);

		// Debug: Check if there's an error message
		const errorDiv = page.locator('div.preset-filled-error-500');
		const hasError = await errorDiv.isVisible();
		if (hasError) {
			const errorText = await errorDiv.textContent();
			console.log('Registration error displayed:', errorText);
		}

		// Debug: Check current URL
		console.log('Current URL after registration:', page.url());

		// Should be redirected to login page with success message
		await expect(page).toHaveURL(/\/login/);
		await expect(page.getByText(/registration successful.*check your email/i)).toBeVisible();

		// Verify that verification email was sent via Mailpit
		// Wait a bit for email to be processed
		await page.waitForTimeout(1000);

		const email = await getLatestEmailFromMailpit(uniqueEmail);

		// Debug logging to help diagnose Mailpit issues
		if (!email) {
			const mailpitUrl = 'http://localhost:8025';
			const debugResponse = await fetch(`${mailpitUrl}/api/v1/messages?limit=50`);
			const debugData = await debugResponse.json();
			console.log('Mailpit debug - Total messages:', debugData.messages?.length || 0);
			console.log('Mailpit debug - Looking for email:', uniqueEmail);
			if (debugData.messages && debugData.messages.length > 0) {
				console.log('Mailpit debug - First message To:', debugData.messages[0].To);
			}
		}

		expect(email).not.toBeNull();
		expect(email.Subject).toContain('Verify Your Email Address');
		expect(email.Text).toContain('verify your email');
		expect(email.Text).toContain('/verify-email?token=');

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

	test.skip('should disable form during submission', async ({ page }) => {
		// This test is skipped because the registration completes too quickly in E2E environment
		// to reliably catch the loading state. The button text changes from "Create Account" to
		// "Creating Account..." for such a short time that Playwright cannot consistently detect it.

		// Fill in form
		const timestamp = Date.now();
		await page.getByPlaceholder('First name').fill('Test');
		await page.getByPlaceholder('Last name').fill('User');
		await page.getByPlaceholder(/email/i).fill(`test${timestamp}@example.com`);
		await page.getByPlaceholder('Password', { exact: true }).fill('password123');
		await page.getByPlaceholder('Confirm password').fill('password123');

		// Submit and complete
		await page.getByRole('button', { name: /create account/i }).click();
	});
});
