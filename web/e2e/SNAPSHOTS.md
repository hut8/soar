# Visual Regression Testing - Snapshot Baselines

This document explains how to generate and update screenshot baselines for visual regression testing.

## Overview

Several E2E tests use Playwright's `toHaveScreenshot()` to capture and compare screenshots for visual regression testing. These tests will fail in CI until baseline snapshots are generated and committed to the repository.

## Current Status

⚠️ **Baseline snapshots need to be generated**

The following tests require baseline snapshots:

- `aircraft-detail.test.ts`: 3 screenshot tests
- `aircraft-list.test.ts`: 2 screenshot tests
- `auth/forgot-password.test.ts`: 1 screenshot test

## Generating Baseline Snapshots

### Prerequisites

1. **Database setup**: Ensure PostgreSQL is running with the test database
2. **Build backend**: `cargo build --release` (from project root)
3. **Build frontend**: `npm run build` (from web/ directory)
4. **Seed test data**: `../target/release/soar seed-test-data`

### Generate Snapshots

Run Playwright tests with the update-snapshots flag:

```bash
cd web
npm test -- --update-snapshots
```

This will:

1. Run all E2E tests
2. Generate baseline PNG images in `e2e/**/*-snapshots/` directories
3. Save snapshots with platform-specific naming (e.g., `*-chromium-linux.png`)

### Review Snapshots

Before committing, review the generated snapshots:

```bash
# Snapshots are saved in test directories
find e2e -name "*-snapshots" -type d
```

Verify that:

- ✅ Screenshots show the correct UI state
- ✅ No sensitive data is visible
- ✅ Layout looks correct

### Commit Snapshots

Once reviewed, commit the snapshots:

```bash
git add web/e2e/**/*-snapshots/
git commit -m "test: add baseline snapshots for visual regression testing"
git push
```

## Updating Snapshots

When UI changes intentionally modify the appearance of tested pages:

```bash
cd web
npm test -- --update-snapshots
git add web/e2e/**/*-snapshots/
git commit -m "test: update snapshots after UI changes"
```

## Snapshot Naming Convention

Playwright automatically names snapshots based on:

- Test file name
- Test description
- Browser (e.g., `chromium`)
- Platform (e.g., `linux`, `darwin`)

Example: `aircraft-detail-authenticatedPage-chromium-linux.png`

## Platform Differences

Snapshots may differ across platforms due to:

- Font rendering
- Anti-aliasing
- Browser versions

For CI consistency, generate snapshots on:

- **Platform**: Ubuntu 22.04 (matches CI)
- **Browser**: Chromium (default in CI)

Or run in Docker to match CI environment exactly.

## Troubleshooting

### Tests Still Failing After Committing Snapshots

1. Verify snapshots are in correct directories
2. Check platform naming matches CI (should be `*-linux.png`)
3. Ensure snapshots were committed and pushed

### Snapshots Look Different Locally vs CI

- Use `maxDiffPixelRatio` option in test to allow minor differences
- Generate snapshots in Docker to match CI environment
- Consider using `toHaveScreenshot()` config options

## References

- [Playwright Visual Comparisons](https://playwright.dev/docs/test-snapshots)
- [Playwright toHaveScreenshot()](https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-screenshot-1)
