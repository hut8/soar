import { type Page } from '@playwright/test';

/**
 * Navigation utilities for E2E tests
 *
 * These helpers provide common navigation patterns used across tests
 */

/**
 * Navigate to the aircraft page
 *
 * @param page - Playwright page object
 * @returns Promise that resolves when navigation is complete
 */
export async function goToAircraft(page: Page): Promise<void> {
	await page.goto('/aircraft');
	await page.waitForLoadState('networkidle');
}

/**
 * Navigate to a specific aircraft detail page
 *
 * @param page - Playwright page object
 * @param deviceId - Aircraft ID
 * @returns Promise that resolves when navigation is complete
 */
export async function goToDeviceDetail(page: Page, deviceId: string): Promise<void> {
	await page.goto(`/aircraft/${deviceId}`);
	await page.waitForLoadState('networkidle');
}

/**
 * Navigate to the flights page
 *
 * @param page - Playwright page object
 * @returns Promise that resolves when navigation is complete
 */
export async function goToFlights(page: Page): Promise<void> {
	await page.goto('/flights');
	await page.waitForLoadState('networkidle');
}

/**
 * Navigate to the home page
 *
 * @param page - Playwright page object
 * @returns Promise that resolves when navigation is complete
 */
export async function goToHome(page: Page): Promise<void> {
	await page.goto('/');
	await page.waitForLoadState('networkidle');
}

/**
 * Search for aircraft by registration
 *
 * @param page - Playwright page object
 * @param registration - Aircraft registration to search for
 * @returns Promise that resolves when search is complete
 */
export async function searchAircraftByRegistration(
	page: Page,
	registration: string
): Promise<void> {
	// Make sure we're on the aircraft page
	await goToAircraft(page);

	// Fill in the search input (registration is the default search type)
	// Use locator with :visible to only interact with the visible input (mobile or desktop)
	await page.locator('input[placeholder*="Aircraft registration"]:visible').fill(registration);

	// Click the search button
	await page.getByRole('button', { name: /search aircraft/i }).click();

	// Wait for results to load
	await page.waitForLoadState('networkidle');
}

/**
 * Search for aircraft by aircraft address
 *
 * @param page - Playwright page object
 * @param address - Aircraft address to search for
 * @param addressType - Address type ('I' for ICAO, 'O' for OGN, 'F' for FLARM)
 * @returns Promise that resolves when search is complete
 */
export async function searchAircraftByAddress(
	page: Page,
	address: string,
	addressType: 'I' | 'O' | 'F' = 'I'
): Promise<void> {
	// Make sure we're on the aircraft page
	await goToAircraft(page);

	// Click the "Aircraft Address" search type (need to switch from default "Registration")
	await page.getByRole('button', { name: /aircraft address/i }).click();

	// Select the address type (ICAO, OGN, or FLARM)
	const addressTypeLabels = { I: 'ICAO', O: 'OGN', F: 'FLARM' };
	await page.getByRole('button', { name: addressTypeLabels[addressType] }).click();

	// Fill in the aircraft address
	// Use locator with :visible to only interact with the visible input (mobile or desktop)
	await page.locator('input[placeholder="Aircraft address"]:visible').fill(address);

	// Click the search button
	await page.getByRole('button', { name: /search aircraft/i }).click();

	// Wait for results to load
	await page.waitForLoadState('networkidle');
}
