---
applyTo: "web/e2e/**/*.spec.ts"
---

# Playwright E2E Test Standards

## Test Structure

Write isolated, maintainable tests following these guidelines:

```typescript
import { test, expect } from '@playwright/test';

test.describe('Device Search', () => {
    test.beforeEach(async ({ page }) => {
        // Navigate to the page before each test
        await page.goto('/devices');
    });

    test('should filter devices by search query', async ({ page }) => {
        // Arrange
        await page.fill('[placeholder="Search devices"]', 'FLRDD');

        // Act
        await page.click('button[type="submit"]');

        // Assert
        await expect(page.locator('.device-card')).toHaveCount(1);
        await expect(page.locator('.device-card').first()).toContainText('FLRDD');
    });

    test('should display all devices when search is empty', async ({ page }) => {
        await page.fill('[placeholder="Search devices"]', '');
        await page.click('button[type="submit"]');

        const deviceCards = page.locator('.device-card');
        await expect(deviceCards).not.toHaveCount(0);
    });
});
```

## Locator Best Practices

**Use stable locators in this order of preference:**

1. **Role-based selectors** (most stable)
   ```typescript
   await page.getByRole('button', { name: 'Search' }).click();
   await page.getByRole('textbox', { name: 'Search devices' }).fill('FLRDD');
   await page.getByRole('heading', { name: 'Aircraft Tracker' });
   ```

2. **Text-based selectors**
   ```typescript
   await page.getByText('Submit').click();
   await page.getByLabel('Email address').fill('user@example.com');
   ```

3. **Test ID selectors**
   ```typescript
   await page.getByTestId('device-card').click();
   ```

4. **CSS selectors** (use sparingly, less stable)
   ```typescript
   await page.locator('.device-card').click();
   ```

## Auto-wait and Assertions

**Leverage Playwright's built-in auto-waiting:**

```typescript
// ✅ CORRECT: Playwright waits automatically
await expect(page.locator('.loading')).toBeVisible();
await expect(page.locator('.result')).toContainText('Success');

// ❌ WRONG: Don't use setTimeout
await page.waitForTimeout(1000); // Avoid this!
```

## Handling Dynamic Content

```typescript
// Wait for network requests
await page.waitForResponse(resp => 
    resp.url().includes('/api/devices') && resp.status() === 200
);

// Wait for specific state
await page.waitForSelector('.device-card', { state: 'visible' });

// Wait for loading to disappear
await expect(page.locator('.loading-spinner')).not.toBeVisible();
```

## Test Data Management

```typescript
test.describe('Flight Creation', () => {
    let testDevice: Device;

    test.beforeEach(async ({ page, request }) => {
        // Set up test data
        const response = await request.post('/api/devices', {
            data: {
                device_id: 'TEST123',
                name: 'Test Device'
            }
        });
        testDevice = await response.json();
    });

    test.afterEach(async ({ request }) => {
        // Clean up test data
        await request.delete(`/api/devices/${testDevice.device_id}`);
    });

    test('should create flight for device', async ({ page }) => {
        await page.goto(`/devices/${testDevice.device_id}`);
        await page.click('button:has-text("Start Flight")');
        await expect(page.locator('.flight-status')).toContainText('Active');
    });
});
```

## Cross-browser Testing

```typescript
// Run tests across multiple browsers
import { devices } from '@playwright/test';

test.describe('Responsive Design', () => {
    test('should display mobile menu on small screens', async ({ page }) => {
        await page.setViewportSize(devices['iPhone 12'].viewport);
        await page.goto('/');
        
        await expect(page.locator('.mobile-menu-button')).toBeVisible();
        await expect(page.locator('.desktop-nav')).not.toBeVisible();
    });
});
```

## Error Handling and Debugging

```typescript
test('should handle API errors gracefully', async ({ page }) => {
    // Intercept and mock failing API call
    await page.route('**/api/devices', route => {
        route.fulfill({
            status: 500,
            body: JSON.stringify({ error: 'Internal Server Error' })
        });
    });

    await page.goto('/devices');
    await expect(page.locator('.error-message')).toContainText('Failed to load devices');
});

// Use screenshots for debugging
test('visual verification', async ({ page }) => {
    await page.goto('/devices');
    await expect(page).toHaveScreenshot('devices-page.png');
});
```

## Page Object Pattern

```typescript
// pages/devices-page.ts
export class DevicesPage {
    constructor(private page: Page) {}

    async goto() {
        await this.page.goto('/devices');
    }

    async searchDevices(query: string) {
        await this.page.fill('[placeholder="Search devices"]', query);
        await this.page.click('button[type="submit"]');
    }

    async getDeviceCards() {
        return this.page.locator('.device-card');
    }

    async expectDeviceCount(count: number) {
        await expect(this.getDeviceCards()).toHaveCount(count);
    }
}

// Use in tests
test('search devices', async ({ page }) => {
    const devicesPage = new DevicesPage(page);
    await devicesPage.goto();
    await devicesPage.searchDevices('FLRDD');
    await devicesPage.expectDeviceCount(1);
});
```

## Common Pitfalls to Avoid

1. ❌ Using `page.waitForTimeout()` - use auto-wait instead
2. ❌ Using fragile CSS selectors - prefer role-based selectors
3. ❌ Not cleaning up test data - use `afterEach` hooks
4. ❌ Tests depending on each other - each test should be independent
5. ❌ Not handling loading states - wait for content to load
6. ❌ Hardcoding viewport sizes - use Playwright's device descriptors

## CI/CD Integration

```typescript
// playwright.config.ts should include:
{
  use: {
    baseURL: process.env.BASE_URL || 'http://localhost:4173',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    trace: 'on-first-retry',
  },
  workers: process.env.CI ? 1 : undefined,
  retries: process.env.CI ? 2 : 0,
}
```

## Test Naming Conventions

```typescript
// ✅ CORRECT: Descriptive test names
test('should display error message when API returns 500', async ({ page }) => {});
test('should navigate to device details when card is clicked', async ({ page }) => {});

// ❌ WRONG: Vague test names
test('test 1', async ({ page }) => {});
test('check devices', async ({ page }) => {});
```

## Before Committing

- Run `npm test` to execute all E2E tests
- Ensure tests pass in headless mode
- Check that screenshots and videos are not committed (should be in `.gitignore`)
- Verify test coverage for critical user flows
