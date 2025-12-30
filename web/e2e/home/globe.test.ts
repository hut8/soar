import { test, expect } from '../fixtures/worker-database.fixture';

test.describe('Globe Page', () => {
	// Increase timeout for globe tests as Cesium loads asynchronously (5.5MB)
	test.setTimeout(90000);

	test.beforeEach(async ({ page }) => {
		await page.goto('/globe');
	});

	test('should display globe page with correct elements', async ({ page }) => {
		// Check page title
		await expect(page).toHaveTitle(/globe|3d|soar/i);

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Wait for EITHER error state OR success state to appear
		// Cesium loads asynchronously, so we need to wait for initialization to complete
		const errorMessage = page.locator('.error-message');
		const cesiumContainer = page.locator('.cesium-container');

		// Wait for loading to finish (either error or success state appears)
		await Promise.race([
			errorMessage.waitFor({ state: 'visible', timeout: 60000 }),
			cesiumContainer.waitFor({ state: 'visible', timeout: 60000 })
		]).catch(() => {
			// If both timeout, that's okay, we'll check below
		});

		// Check if error state is showing (WebGL may not be available in CI)
		const hasError = await errorMessage.isVisible();
		if (hasError) {
			const errorText = await errorMessage.textContent();
			// WebGL initialization failure is expected in headless CI environments
			if (errorText?.includes('WebGL') || errorText?.includes('initialization failed')) {
				test.skip();
				return;
			}
			throw new Error(`Globe page showed error: ${errorText}`);
		}

		// Verify Cesium container is visible
		await expect(cesiumContainer).toBeVisible();

		// Give time for 3D rendering to initialize
		await page.waitForTimeout(2000);

		// Verify Cesium viewer is present (the main 3D canvas)
		const cesiumViewer = page.locator('.cesium-viewer');
		await expect(cesiumViewer).toBeVisible();
	});

	test('should load without JavaScript errors', async ({ page }) => {
		const errors: string[] = [];

		// Collect console errors
		page.on('pageerror', (error) => {
			errors.push(error.message);
		});

		await page.goto('/globe');
		await page.waitForLoadState('networkidle');

		// Wait for EITHER error state OR success state to appear
		// Cesium loads asynchronously, so we need to wait for initialization to complete
		const errorMessage = page.locator('.error-message');
		const cesiumContainer = page.locator('.cesium-container');

		// Wait for loading to finish (either error or success state appears)
		await Promise.race([
			errorMessage.waitFor({ state: 'visible', timeout: 60000 }),
			cesiumContainer.waitFor({ state: 'visible', timeout: 60000 })
		]).catch(() => {
			// If both timeout, that's okay, we'll check below
		});

		// Check if error state is showing (WebGL may not be available in CI)
		const hasError = await errorMessage.isVisible();
		if (hasError) {
			const errorText = await errorMessage.textContent();
			// WebGL initialization failure is expected in headless CI environments
			if (errorText?.includes('WebGL') || errorText?.includes('initialization failed')) {
				test.skip();
				return;
			}
			throw new Error(`Globe page showed error: ${errorText}`);
		}

		// Verify Cesium container is visible
		await expect(cesiumContainer).toBeVisible();

		// Wait for potential rendering
		await page.waitForTimeout(2000);

		// Should not have critical errors (some warnings may be acceptable)
		const criticalErrors = errors.filter((e) => e.toLowerCase().includes('error'));
		expect(criticalErrors.length).toBe(0);
	});
});
