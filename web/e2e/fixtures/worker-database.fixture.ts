import { test as base } from '@playwright/test';

/**
 * Simple fixture that provides test and expect from Playwright.
 *
 * Uses shared database and server for all workers (started in CI or locally).
 * Tests ensure data isolation through:
 * - Timestamp-based unique identifiers for created data
 * - UUID-based unique identifiers where appropriate
 * - Unique email addresses for registration tests
 *
 * Usage:
 *   import { test, expect } from '../fixtures/worker-database.fixture';
 *
 *   test('my test', async ({ page }) => {
 *     await page.goto('/login');
 *   });
 */

export const test = base;
export { expect } from '@playwright/test';
