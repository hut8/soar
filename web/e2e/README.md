# End-to-End Testing Guide

This directory contains end-to-end (E2E) tests for the SOAR web application using [Playwright](https://playwright.dev/).

Tests are run automatically in CI on every pull request.

## Table of Contents

- [Overview](#overview)
- [Test Database Setup](#test-database-setup)
- [Running Tests](#running-tests)
- [Writing Tests](#writing-tests)
- [Test Structure](#test-structure)
- [Visual Regression Testing](#visual-regression-testing)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

Our E2E tests use **Playwright** to test the application in real browsers (Chromium by default). Tests cover critical user journeys including:

- **Authentication**: Login, registration, logout flows
- **Aircraft**: Searching, listing, and viewing aircraft
- **Flights**: Flight tracking and details (TODO)
- **Clubs**: Club management (TODO)

## Test Database Setup

E2E tests use **per-worker database isolation** for true test independence. Each Playwright worker gets its own isolated database cloned from a template.

### How It Works

1. **Template Database** (`soar_test_template`): Created once with schema and seed data
2. **Worker Databases** (`soar_test_worker_0`, `soar_test_worker_1`, etc.): Cloned from template for each worker
3. **Worker Servers**: Each worker starts its own web server on a unique port (5000, 5001, 5002, etc.)
4. **Automatic Cleanup**: Worker databases are dropped after tests complete

### Quick Setup (Recommended)

The template database is created automatically when you run tests:

```bash
cd web
npm test
```

The first time (or when using CI), Playwright's global setup will:

1. Build the release binary
2. Create `soar_test_template` database
3. Run migrations on the template
4. Seed the template with test data

Each test worker then:

1. Clones the template to create its own database
2. Starts its own web server on a unique port
3. Runs tests in complete isolation
4. Cleans up its database when done

### Manual Template Setup

If you want to pre-create the template database:

```bash
./scripts/setup-test-template.sh
../target/release/soar seed-test-data
```

### Cleaning Up Orphaned Worker Databases

If tests are interrupted, worker databases may remain. Clean them up with:

```bash
./scripts/cleanup-test-workers.sh
```

### Test Data

The seed command creates:

- **Test User**: `test@example.com` / `testpassword123` (configurable)
- **Test Club**: "Test Soaring Club"
- **Test Aircraft**: N12345, N54321, N98765 (plus random aircraft)
- **Test Pilots**: Mix of licensed/unlicensed, instructors, tow pilots
- **Fake Users**: Realistic test users with random names/emails

### Environment Variables

Customize test data:

```bash
# Test user credentials (defaults shown)
export TEST_USER_EMAIL="test@example.com"
export TEST_USER_PASSWORD="testpassword123"

# Number of additional fake records to create
export SEED_COUNT=20

# Then run setup script
./scripts/setup-test-db.sh
```

## Running Tests

### Run all tests

```bash
cd web
npm test
# or
npm run test:e2e
```

### Run tests in headed mode (see browser)

```bash
npx playwright test --headed
```

### Run tests in UI mode (interactive)

```bash
npx playwright test --ui
```

### Run specific test file

```bash
npx playwright test e2e/auth/login.test.ts
```

### Run tests matching a pattern

```bash
npx playwright test --grep "login"
```

### Debug a specific test

```bash
npx playwright test --debug e2e/auth/login.test.ts
```

## Writing Tests

### Basic Test Structure

```typescript
import { test, expect } from '@playwright/test';

test.describe('Feature Name', () => {
	test.beforeEach(async ({ page }) => {
		// Set up before each test
		await page.goto('/your-page');
	});

	test('should do something', async ({ page }) => {
		// Your test code
		await expect(page.getByRole('heading')).toBeVisible();
	});
});
```

### Using Authentication Fixtures

For tests that require a logged-in user:

```typescript
import { test, expect } from '../fixtures/auth.fixture';

test('authenticated test', async ({ authenticatedPage }) => {
	// This page is already logged in
	await authenticatedPage.goto('/aircraft');
	// ... test protected functionality
});
```

### Using Test Data

```typescript
import { testUsers, testAircraft } from '../fixtures/data.fixture';

test('login with test user', async ({ page }) => {
	await login(page, testUsers.validUser.email, testUsers.validUser.password);
});
```

### Using Helper Functions

```typescript
import { login } from '../utils/auth';
import { goToAircraft, searchAircraftByRegistration } from '../utils/navigation';

test('search for aircraft', async ({ page }) => {
	await login(page, 'test@example.com', 'password');
	await searchAircraftByRegistration(page, 'N12345');
	// ... assertions
});
```

## Test Structure

```
e2e/
├── fixtures/           # Test fixtures and setup
│   ├── auth.fixture.ts # Pre-authenticated context
│   └── data.fixture.ts # Test data and constants
├── utils/              # Reusable utilities
│   ├── auth.ts        # Authentication helpers
│   └── navigation.ts  # Navigation helpers
├── auth/               # Authentication tests
│   ├── login.test.ts
│   ├── register.test.ts
│   └── logout.test.ts
└── aircraft/           # Aircraft-related tests
    ├── aircraft-list.test.ts
    └── aircraft-detail.test.ts
```

## Visual Regression Testing

We use Playwright's built-in screenshot comparison for visual regression testing.

### Taking Screenshots

```typescript
test('visual test', async ({ page }) => {
	await page.goto('/');
	await expect(page).toHaveScreenshot('homepage.png');
});
```

### First Run (Creating Baselines)

The first time you run a test with screenshots, Playwright will **create baseline images**:

```bash
npx playwright test
```

Baseline screenshots are stored in `e2e/**/*.spec.ts-snapshots/`.

### Updating Screenshots

If UI changes are intentional, update the baselines:

```bash
npx playwright test --update-snapshots
```

### Screenshot Best Practices

- **Use descriptive names**: `login-page.png`, not `screenshot1.png`
- **Set thresholds for dynamic content**:
  ```typescript
  await expect(page).toHaveScreenshot('results.png', {
  	maxDiffPixelRatio: 0.1 // Allow 10% difference
  });
  ```
- **Disable animations**: Configured globally in `playwright.config.ts`
- **Hide dynamic content**: Use Playwright's masking features for timestamps, etc.

## Best Practices

### 1. Use Descriptive Test Names

```typescript
// ✅ Good
test('should show error when submitting empty login form', async ({ page }) => {});

// ❌ Bad
test('test1', async ({ page }) => {});
```

### 2. Use Playwright Locators

```typescript
// ✅ Good - Resilient to changes
await page.getByRole('button', { name: /sign in/i }).click();
await page.getByPlaceholder('Enter your email').fill('test@example.com');

// ❌ Avoid - Fragile
await page.locator('button.btn-primary').click();
await page.locator('#email-input').fill('test@example.com');
```

### 3. Wait for Network Idle

```typescript
test('wait for data loading', async ({ page }) => {
	await page.goto('/aircraft');
	await page.waitForLoadState('networkidle');
	// Now safe to assert on loaded data
});
```

### 4. Handle Conditional Content

```typescript
test('handle optional content', async ({ page }) => {
	const hasPagination = await page.getByRole('button', { name: /next/i }).isVisible();

	if (hasPagination) {
		// Test pagination
	}
});
```

### 5. Clean Up Test Data

Use `afterEach` or `afterAll` hooks to clean up:

```typescript
test.afterEach(async () => {
	// Clean up test data created during test
});
```

## Troubleshooting

### Tests Failing Due to Missing Template Database

**Symptom**: Tests fail with "template database does not exist" or connection errors

**Solution**: Rebuild the template database

```bash
./scripts/setup-test-template.sh
../target/release/soar seed-test-data
```

Or simply run tests again - global setup will create it automatically.

### Worker Databases Not Cleaned Up

**Symptom**: Multiple `soar_test_worker_*` databases remain in PostgreSQL

**Solution**: Run the cleanup script

```bash
./scripts/cleanup-test-workers.sh
```

This is safe to run anytime and only affects test worker databases.

### Tests are flaky

- Add explicit waits: `await page.waitForLoadState('networkidle')`
- Increase timeouts if needed: `test.setTimeout(60000)`
- Check for race conditions in async operations

### Screenshots don't match

- Run `npx playwright test --update-snapshots` to update baselines
- Check if tests are running in different environments (different OS, screen size)
- Increase `maxDiffPixelRatio` for dynamic content

### Can't find elements

- Use Playwright Inspector to debug:
  ```bash
  npx playwright test --debug
  ```
- Check if element is inside a frame/iframe
- Verify element is visible and not behind another element
- Check if selector needs heading level: `{ name: /text/i, level: 1 }`

### Authentication not working

- **First, verify test database is set up**: `./scripts/setup-test-db.sh`
- Check test user credentials in `fixtures/data.fixture.ts`
- Verify the test user exists in soar_test database:
  ```bash
  psql -U postgres -d soar_test -c "SELECT email FROM users WHERE email = 'test@example.com';"
  ```
- Check browser console for errors in Playwright UI mode

### Database Connection Issues

```bash
# Verify PostgreSQL is running
psql -U postgres -c "SELECT version();"

# Check database exists
psql -U postgres -c "\l" | grep soar_test

# Check test data was seeded
psql -U postgres -d soar_test -c "SELECT COUNT(*) FROM users;"
psql -U postgres -d soar_test -c "SELECT COUNT(*) FROM aircraft;"
```

### Build takes too long

- Use `reuseExistingServer` in `playwright.config.ts` (already configured)
- Build once, then run tests multiple times
- Consider running tests against dev server for faster iteration

## Environment Variables

Configure test behavior with environment variables:

```bash
# Use specific test user credentials
TEST_USER_EMAIL=test@example.com TEST_USER_PASSWORD=password123 npm test

# Run in CI mode
CI=true npm test
```

## Adding New Tests

1. **Create test file** in appropriate directory (`auth/`, `aircraft/`, etc.)
2. **Import necessary utilities** from `fixtures/` and `utils/`
3. **Write tests** following existing patterns
4. **Add screenshots** for visual regression testing
5. **Run tests** locally to verify
6. **Commit** test file and baseline screenshots

## Resources

- [Playwright Documentation](https://playwright.dev/docs/intro)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Playwright API Reference](https://playwright.dev/docs/api/class-playwright)
- [Writing Locators](https://playwright.dev/docs/locators)
- [Visual Comparisons](https://playwright.dev/docs/test-snapshots)
