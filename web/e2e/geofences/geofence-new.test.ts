import { test, expect } from '../fixtures/auth.fixture';

test.describe('Geofence Pages', () => {
	// Cesium loads asynchronously and may not work in headless CI
	test.setTimeout(60000);

	test('new geofence page should load without errors', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/geofences/new');
		await authenticatedPage.waitForLoadState('networkidle');

		// Page should not show the SvelteKit error page
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).not.toContain('An error occurred');

		// Should display the "Create Geofence" heading
		await expect(
			authenticatedPage.getByRole('heading', { name: /create geofence/i })
		).toBeVisible();
	});

	test('new geofence page should render editor form', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/geofences/new');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should have the name input
		await expect(authenticatedPage.getByPlaceholder('Enter geofence name')).toBeVisible();

		// Should have the airport selector
		await expect(
			authenticatedPage.getByPlaceholder('Search airports by name or identifier...')
		).toBeVisible();

		// Should have the 3D preview container div (even if Cesium/WebGL fails to init)
		await expect(authenticatedPage.getByText('3D Preview')).toBeVisible();
	});

	test('geofence list page should load', async ({ authenticatedPage }) => {
		await authenticatedPage.goto('/geofences');
		await authenticatedPage.waitForLoadState('networkidle');

		// Should display the geofences heading
		await expect(
			authenticatedPage.getByRole('heading', { level: 1, name: /geofences/i })
		).toBeVisible();

		// Page should not show an error
		const bodyText = await authenticatedPage.textContent('body');
		expect(bodyText).not.toContain('An error occurred');
	});
});
