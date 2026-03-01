import { test, expect } from '../fixtures/worker-database.fixture';

test.describe('Spotter Page', () => {
	// Increase timeout for spotter tests as Cesium loads asynchronously
	test.setTimeout(90000);

	test('should display location picker on initial load', async ({ page }) => {
		await page.goto('/spotter');
		await page.waitForLoadState('networkidle');

		// Location picker modal should be visible on first load
		const modal = page.locator('[role="dialog"]');
		await expect(modal).toBeVisible({ timeout: 10000 });

		// Should have the title
		await expect(page.locator('#location-picker-title')).toHaveText('Choose Location');

		// Should have search input
		const searchInput = page.locator('input[placeholder*="Search airport"]');
		await expect(searchInput).toBeVisible();

		// Should have "Use My Location" button
		await expect(page.getByText('Use My Location')).toBeVisible();

		// Should have the map container
		const mapContainer = page.locator('.map-container');
		await expect(mapContainer).toBeVisible();
	});

	test('should skip location picker when URL params are provided', async ({ page }) => {
		await page.goto('/spotter?lat=39.83&lon=-104.67');
		await page.waitForLoadState('networkidle');

		// Wait for either Cesium to load or an error
		const loadingSpinner = page.locator('.loading-spinner');
		const errorMessage = page.locator('.error-message');
		const cesiumContainer = page.locator('.cesium-container');

		await loadingSpinner.waitFor({ state: 'hidden', timeout: 60000 }).catch(() => {
			// Spinner might not be present
		});

		// Check if error state is showing (WebGL may not be available in CI)
		const errorCount = await errorMessage.count();
		if (errorCount > 0 && (await errorMessage.isVisible())) {
			const errorText = await errorMessage.textContent();
			if (errorText?.includes('WebGL') || errorText?.includes('Failed to load')) {
				test.skip();
			}
			throw new Error(`Spotter page showed unexpected error: ${errorText}`);
		}

		// Cesium container should be visible
		await expect(cesiumContainer).toBeVisible({ timeout: 10000 });

		// Location picker should NOT be visible (skipped via URL params)
		const locationPickerModal = page.locator('#location-picker-title');
		await expect(locationPickerModal).not.toBeVisible();
	});

	test('should have close button', async ({ page }) => {
		await page.goto('/spotter');
		await page.waitForLoadState('networkidle');

		const closeButton = page.locator('.btn-close');
		await expect(closeButton).toBeVisible();
	});

	test('should have correct page title', async ({ page }) => {
		await page.goto('/spotter');
		await expect(page).toHaveTitle(/spotter|soar/i);
	});

	test('should load without JavaScript errors', async ({ page }) => {
		const errors: string[] = [];

		page.on('pageerror', (error) => {
			errors.push(error.message);
		});

		await page.goto('/spotter?lat=39.83&lon=-104.67');
		await page.waitForLoadState('networkidle');

		const loadingSpinner = page.locator('.loading-spinner');
		const errorMessage = page.locator('.error-message');

		await loadingSpinner.waitFor({ state: 'hidden', timeout: 60000 }).catch(() => {});

		const errorCount = await errorMessage.count();
		if (errorCount > 0 && (await errorMessage.isVisible())) {
			const errorText = await errorMessage.textContent();
			if (errorText?.includes('WebGL') || errorText?.includes('Failed to load')) {
				test.skip();
			}
		}

		await page.waitForTimeout(2000);

		const criticalErrors = errors.filter((e) => e.toLowerCase().includes('error'));
		expect(criticalErrors.length).toBe(0);
	});
});
