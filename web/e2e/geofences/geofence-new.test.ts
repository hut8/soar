import { test, expect } from '../fixtures/auth.fixture';

test.describe('Geofence New Page', () => {
	test('should load without errors', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/geofences/new');
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should not show the SvelteKit error page
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).not.toContain('500');
		expect(bodyText).not.toContain('An error occurred');

		// Should display the "Create Geofence" heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /create geofence/i })
		).toBeVisible();
	});

	test('should render the geofence editor form', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/geofences/new');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have the name input
		await expect(authenticatedPage.getByPlaceholder('Enter geofence name')).toBeVisible();

		// Should have the airport selector
		await expect(
			authenticatedPage.getByPlaceholder('Search airports by name or identifier...')
		).toBeVisible();

		// Should have the Cesium 3D preview container
		await expect(authenticatedPage.locator('.cesium-viewer')).toBeAttached();
	});
});
