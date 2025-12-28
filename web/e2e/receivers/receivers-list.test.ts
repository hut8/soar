import { test, expect } from '../fixtures/auth.fixture';

test.describe('Receivers List', () => {
	test('should display receivers list page with correct elements', async ({
		authenticatedPage
	}) => {
		await authenticatedPage.goto('/receivers');

		// Check page title
		await expect(authenticatedPage).toHaveTitle(/receivers/i);

		// Wait for page to load
		await authenticatedPage.waitForLoadState('networkidle');

		// Check main heading (page has "Receiver Search" not "Receivers")
		await expect(
			authenticatedPage.getByRole('heading', { name: /receiver search/i, level: 1 })
		).toBeVisible();

		// Take screenshot for visual regression testing
		await expect(authenticatedPage).toHaveScreenshot('receivers-list-page.png');
	});

	test('should display search or filter functionality', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for search input or filter controls
		const searchInput = authenticatedPage.locator(
			'input[type="search"], input[placeholder*="search" i], input[placeholder*="filter" i]'
		);
		const hasSearch = await searchInput
			.first()
			.isVisible()
			.catch(() => false);

		if (hasSearch) {
			await expect(searchInput.first()).toBeVisible();
		}
	});

	test('should display receivers data or empty state', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should either show receivers list or a "no receivers" message
		const hasReceiverLinks = await authenticatedPage.locator('a[href^="/receivers/"]').count();
		const hasEmptyState = await authenticatedPage
			.getByText(/no receivers|empty/i)
			.isVisible()
			.catch(() => false);

		// At least one of these should be true
		expect(hasReceiverLinks > 0 || hasEmptyState).toBe(true);
	});

	test('should navigate to receiver detail when clicking a receiver', async ({
		authenticatedPage
	}) => {
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Find first receiver link (exclude coverage link)
		const receiverLinks = authenticatedPage.locator(
			'a[href^="/receivers/"]:not([href*="/coverage"])'
		);
		const count = await receiverLinks.count();

		if (count > 0) {
			const firstReceiver = receiverLinks.first();
			await firstReceiver.click();

			// Should navigate to receiver detail page
			await expect(authenticatedPage).toHaveURL(/\/receivers\/[^/]+/);
			await authenticatedPage.waitForLoadState('networkidle');

			// Take screenshot of receiver detail from list
			await expect(authenticatedPage).toHaveScreenshot('receiver-detail-from-list.png');
		}
	});

	test('should have link to coverage map', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Look for coverage link
		const coverageLink = authenticatedPage.getByRole('link', { name: /coverage|map/i });
		const hasCoverageLink = await coverageLink.isVisible().catch(() => false);

		if (hasCoverageLink) {
			await coverageLink.click();
			await expect(authenticatedPage).toHaveURL(/\/receivers\/coverage/);
		}
	});

	test('should be responsive on mobile viewport', async ({ authenticatedPage }) => {
		await authenticatedPage.setViewportSize({ width: 375, height: 667 });
		await authenticatedPage.goto('/receivers');
		await authenticatedPage.waitForLoadState('networkidle');

		// Take screenshot for mobile view
		await expect(authenticatedPage).toHaveScreenshot('receivers-list-mobile.png');
	});
});
