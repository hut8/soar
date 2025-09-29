# CLAUDE.md - AI Assistant Guide for SOAR Project

This document provides essential guidance for AI assistants working on the SOAR (Soaring Observation And Records) project.

## Project Overview

SOAR is a comprehensive aircraft tracking and club management system built with:

- **Backend**: Rust with Axum web framework, PostgreSQL with PostGIS
- **Frontend**: SvelteKit with TypeScript, Tailwind CSS, Skeleton UI components
- **Real-time**: NATS messaging for live aircraft position updates
- **Data Sources**: APRS-IS integration, FAA aircraft registry, airport databases

## Critical Development Rules

### NO BYPASSING QUALITY CONTROLS
- **NEVER use `git commit --no-verify`** - All commits must pass pre-commit hooks
- **NEVER use `git push`** - Only commit changes, never push to remote
- **NEVER skip CI checks** - Local development must match GitHub Actions pipeline
- **ASK BEFORE removing large amounts of working code** - Get confirmation before major deletions
- **AVOID duplicate code** - Check for existing implementations before writing new code
- Pre-commit hooks run: `cargo fmt`, `cargo clippy`, `cargo test`, `npm lint`, `npm check`, `npm test`

### COMMIT AND DATABASE RULES
- **NEVER add Co-Authored-By lines** - Do not include Claude Code attribution in commits
- **AVOID raw SQL in Diesel** - Only use raw SQL if absolutely necessary, and ask first before using it
- Always prefer Diesel's query builder and type-safe methods over raw SQL

### Frontend Development Standards

#### Svelte 5 Syntax (REQUIRED)
```svelte
<!--  CORRECT: Use Svelte 5 event handlers -->
<button onclick={handleClick}>Click me</button>
<input oninput={handleInput} onkeydown={handleKeydown} />

<!-- L WRONG: Don't use Svelte 4 syntax -->
<button on:click={handleClick}>Click me</button>
<input on:input={handleInput} on:keydown={handleKeydown} />
```

#### Icons (REQUIRED)
```svelte
<!--  CORRECT: Use @lucide/svelte exclusively -->
import { Search, User, Settings, ChevronDown } from '@lucide/svelte';

<!-- L WRONG: Don't use other icon libraries -->
```

#### Component Libraries
- **Skeleton UI**: Use `@skeletonlabs/skeleton-svelte` components (Svelte 5 compatible)
- **Tailwind CSS**: Use utility-first CSS approach
- **TypeScript**: Full type safety required

### Backend Development Standards

#### Rust Code Quality (REQUIRED)
- **ALWAYS run `cargo fmt`** after editing Rust files to ensure consistent formatting
- **Pre-commit hooks automatically run `cargo fmt`** - but format manually for immediate feedback
- **Use `cargo clippy`** to catch common issues and improve code quality
- All Rust code must pass formatting, clippy, and tests before commit

#### Rust Patterns
```rust
//  Use anyhow::Result for error handling
use anyhow::Result;

//  Use tracing for logging
use tracing::{info, warn, error, debug};

//  Proper async function signatures
pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    // Handler implementation
}
```

#### Database Patterns
```rust
//  Use Diesel ORM patterns
use diesel::prelude::*;

//  PostGIS integration
use postgis_diesel::geography::Geography;
```

## Technology-Specific Documentation

# tailwindcss.com llms.txt

> Tailwind CSS offers a utility-first CSS framework that enables developers to create custom designs quickly and efficiently, promoting consistency and maintainability without the complexities of traditional CSS.

- [Tailwind CSS Order Utilities](https://tailwindcss.com/docs/order): Explains how to use Tailwind CSS order utilities for flex and grid layouts.
- [Font Family Documentation](https://tailwindcss.com/docs/font-family): Guide on how to use and customize font families in Tailwind CSS.
- [Box Decoration Break Guide](https://tailwindcss.com/docs/box-decoration-break): Explain how to use the box decoration break utilities in Tailwind CSS for styling elements.
- [List Style Position Guide](https://tailwindcss.com/docs/list-style-position): To explain how to use Tailwind CSS utilities for setting list style positions in web design.
- [Aspect Ratio Documentation](https://tailwindcss.com/docs/aspect-ratio): Guide on using aspect ratio utilities in Tailwind CSS for responsive design.
- [Grid Template Columns](https://tailwindcss.com/docs/grid-template-columns): This page details how to use grid-template-columns in Tailwind CSS for creating responsive grid layouts.
- [Tailwind CSS Modifiers](https://tailwindcss.com/docs/hover-focus-and-other-states): This page explains how to use Tailwind CSS modifiers for hover, focus, and other states.
- [Tailwind CSS Flex Guide](https://tailwindcss.com/docs/flex): Guide on using Tailwind CSS flex utilities for responsive design and customization.
- [Tailwind CSS Gap Utilities](https://tailwindcss.com/docs/gap): Explain the usage of gap utilities in Tailwind CSS for layout spacing.
- [Join Tailwind CSS Discord](https://tailwindcss.com/discord): Join the Tailwind CSS community on Discord.
- [Text Alignment Guide](https://tailwindcss.com/docs/text-align): Guide to using text alignment utilities in Tailwind CSS.
- [Align Self Documentation](https://tailwindcss.com/docs/align-self): Guide on using Tailwind CSS 'align-self' utility class for flexible item alignment in layouts.
- [Tailwind CSS IntelliSense Setup](https://tailwindcss.com/docs/intellisense): Provide guidance on setting up Tailwind CSS IntelliSense for better coding experience in various editors.
- [Tailwind CSS Padding Guide](https://tailwindcss.com/docs/padding): Detailing the usage and classes for padding in Tailwind CSS.
- [Grid Auto Flow](https://tailwindcss.com/docs/grid-auto-flow): Guide on using Tailwind CSS classes for controlling grid auto-placement in layouts.
- [Upgrade Guide](https://tailwindcss.com/docs/upgrade-guide): Guide for upgrading to Tailwind CSS v3.0, detailing new features and necessary changes.
- [Min-Height Documentation](https://tailwindcss.com/docs/min-height): Explains how to use min-height utilities in Tailwind CSS for styling elements effectively.
- [Tailwind CSS Margin Documentation](https://tailwindcss.com/docs/margin): This page documents margin utilities in Tailwind CSS for styling components.
- [Place Items in Tailwind CSS](https://tailwindcss.com/docs/place-items): Explain the usage of the 'place-items' utility in Tailwind CSS for grid item alignment.
- [Max Height Documentation](https://tailwindcss.com/docs/max-height): Provides documentation for setting maximum height using Tailwind CSS utilities.
- [Text Decoration in Tailwind](https://tailwindcss.com/docs/text-decoration): Explains how to use text decoration utilities in Tailwind CSS for styling text.
- [Text Decoration Thickness](https://tailwindcss.com/docs/text-decoration-thickness): Explain how to set text decoration thickness in Tailwind CSS.
- [Flex Direction Documentation](https://tailwindcss.com/docs/flex-direction): This page explains the flex-direction utilities in Tailwind CSS for arranging flex items.
- [Tailwind CSS Overview](https://tailwindcss.com/): Promotes Tailwind CSS as a utility-first CSS framework to enhance web development efficiency and customization.
- [Z-Index Documentation](https://tailwindcss.com/docs/z-index): Provides guidelines for using z-index utilities in Tailwind CSS for stacking elements.
- [Grid Auto Rows Documentation](https://tailwindcss.com/docs/grid-auto-rows): Provide documentation for using grid auto rows in Tailwind CSS.
- [Justify Items Documentation](https://tailwindcss.com/docs/justify-items): To explain the usage of the justify-items utility in Tailwind CSS for grid item alignment.
- [Place Self Documentation](https://tailwindcss.com/docs/place-self): This page details the usage of the 'place-self' utility classes in Tailwind CSS for grid item alignment.
- [Grid Row Documentation](https://tailwindcss.com/docs/grid-row): To provide documentation on using grid row utilities in Tailwind CSS for layout customization.
- [Grid Template Rows](https://tailwindcss.com/docs/grid-template-rows): Explains the usage of grid-template-rows utilities in Tailwind CSS for creating responsive grid layouts.
- [Tailwind CSS Plugins Guide](https://tailwindcss.com/docs/plugins): Guide for creating and using Tailwind CSS plugins to enhance styling capabilities.
- [Grid Column Documentation](https://tailwindcss.com/docs/grid-column): To provide documentation for using grid column utilities in Tailwind CSS, including classes and customization options.
- [Responsive Design Guide](https://tailwindcss.com/docs/responsive-design): Explain how to implement responsive design using Tailwind CSS utility classes across different breakpoints.
- [Text Decoration Color Guide](https://tailwindcss.com/docs/text-decoration-color): Explain how to customize text decoration colors in Tailwind CSS.
- [Customizing Spacing in Tailwind](https://tailwindcss.com/docs/customizing-spacing): Guide to customizing Tailwind CSS spacing settings in the configuration file.
- [Justify Self in Tailwind CSS](https://tailwindcss.com/docs/justify-self): Explain how to use the justify-self utility in Tailwind CSS for grid item alignment.
- [Font Weight Documentation](https://tailwindcss.com/docs/font-weight): Detailing font weight utilities in Tailwind CSS for styling text.
- [Tailwind CSS Directives](https://tailwindcss.com/docs/functions-and-directives): This page explains the functions and directives used in Tailwind CSS for styling applications.
- [Tailwind CSS Resources](https://tailwindcss.com/resources): Provide design resources and community support for Tailwind CSS users.
- [Font Size Documentation](https://tailwindcss.com/docs/font-size): This page details how to use font size utilities in Tailwind CSS for responsive typography styling.
- [Tailwind CSS Sizing Guide](https://tailwindcss.com/docs/size): Explain how to use sizing utilities in Tailwind CSS for fixed, percentage, and customizable sizes.
- [Tailwind CSS Configuration Guide](https://tailwindcss.com/docs/configuration): Guide for configuring Tailwind CSS in web projects.
- [Overscroll Behavior Guide](https://tailwindcss.com/docs/overscroll-behavior): Explains how to utilize overscroll behavior utilities in Tailwind CSS for controlling scrolling effects.
- [Tailwind CSS Positioning](https://tailwindcss.com/docs/position): This page explains different CSS positioning utilities in Tailwind CSS for effective layout design.
- [Optimizing Tailwind CSS](https://tailwindcss.com/docs/optimizing-for-production): Guide for optimizing Tailwind CSS for production use, focusing on file size and performance enhancement techniques.
- [Dark Mode Implementation](https://tailwindcss.com/docs/dark-mode): Guide to implementing dark mode in Tailwind CSS.
- [Utility-First CSS Explained](https://tailwindcss.com/docs/utility-first): Explains the utility-first CSS approach for styling with Tailwind CSS.
- [Letter Spacing Documentation](https://tailwindcss.com/docs/letter-spacing): This page outlines how to use and customize letter spacing utilities in Tailwind CSS.
- [Tailwind CSS Updates](https://tailwindcss.com/blog): Showcases updates, releases, and announcements related to Tailwind CSS and its ecosystem.
- [Tailwind CSS Width Utilities](https://tailwindcss.com/docs/width): This page outlines the width utility classes available in Tailwind CSS for setting element widths.
- [Tailwind CSS Clear Utility](https://tailwindcss.com/docs/clear): This page details the usage of the 'clear' utility in Tailwind CSS for managing floated elements.
- [Tailwind CSS Browser Support](https://tailwindcss.com/docs/browser-support): Details supported browsers and features for Tailwind CSS usage.
- [Line Height Documentation](https://tailwindcss.com/docs/line-height): Explains how to use and customize line-height utilities in Tailwind CSS.
- [Flex Basis Documentation](https://tailwindcss.com/docs/flex-basis): Describes how to use the flex-basis utility in Tailwind CSS for flex item sizing.
- [Break Before Utilities](https://tailwindcss.com/docs/break-before): Explain the use of 'break-before' utilities in Tailwind CSS for controlling element breaks in layouts.
- [Preflight in Tailwind CSS](https://tailwindcss.com/docs/preflight): Explains Preflight styles in Tailwind CSS for consistent design across browsers.
- [Tailwind CSS Overflow Guide](https://tailwindcss.com/docs/overflow): This page explains how to use overflow utilities in Tailwind CSS.
- [Max Width Utilities](https://tailwindcss.com/docs/max-width): Guide on using max-width utilities in Tailwind CSS for responsive design.
- [Flex Wrap Documentation](https://tailwindcss.com/docs/flex-wrap): Provide documentation on Tailwind CSS's flex-wrap utility and its usage in responsive design.
- [Editor Setup Guide](https://tailwindcss.com/docs/editor-setup): Guide users on setting up Tailwind CSS in various code editors.
- [Flex Grow Documentation](https://tailwindcss.com/docs/flex-grow): Explains how to use the flex-grow utility in Tailwind CSS for responsive design.
- [Float Utilities Documentation](https://tailwindcss.com/docs/float): Guide on using float utilities in Tailwind CSS for layout design.
- [Box Sizing Documentation](https://tailwindcss.com/docs/box-sizing): Explain the box-sizing utilities in Tailwind CSS for layout design.
- [Break After Utility](https://tailwindcss.com/docs/break-after): Explain how to use the 'break-after' utility in Tailwind CSS for controlling column and page breaks.
- [Reusing Styles in Tailwind](https://tailwindcss.com/docs/reusing-styles): Learn strategies for reusing styles in Tailwind CSS projects effectively.
- [Align Items in Tailwind](https://tailwindcss.com/docs/align-items): This page explains how to use the align-items utility in Tailwind CSS for styling flexbox layouts.
- [Configuring Tailwind Screens](https://tailwindcss.com/docs/screens): Guide on configuring screen breakpoints in Tailwind CSS.
- [Visibility in Tailwind CSS](https://tailwindcss.com/docs/visibility): Explains how to control element visibility using Tailwind CSS utilities.
- [Tailwind CSS Display Utilities](https://tailwindcss.com/docs/display): This page explains the display utilities in Tailwind CSS for layout control.
- [Font Smoothing Guide](https://tailwindcss.com/docs/font-smoothing): Explains how to implement font smoothing in Tailwind CSS.
- [Tailwind CSS Place Content](https://tailwindcss.com/docs/place-content): This page explains how to use the 'place-content' utility in Tailwind CSS for layout control.
- [Tailwind CSS Presets Guide](https://tailwindcss.com/docs/presets): Explains how to use presets in Tailwind CSS for project customization and management.
- [Line Clamp Documentation](https://tailwindcss.com/docs/line-clamp): To explain how to use the line clamp utility in Tailwind CSS for truncating multi-line text.
- [Grid Auto Columns Guide](https://tailwindcss.com/docs/grid-auto-columns): Explains the usage of grid auto columns in Tailwind CSS.
- [Align Content in Tailwind](https://tailwindcss.com/docs/align-content): Explains the usage of the align-content property in Tailwind CSS with examples.
- [Integrating Tailwind with Preprocessors](https://tailwindcss.com/docs/using-with-preprocessors): Guide on integrating Tailwind CSS with preprocessors like Sass, Less, and Stylus, highlighting best practices and limitations.
- [Font Style Documentation](https://tailwindcss.com/docs/font-style): Provides details on using font style utilities in Tailwind CSS.
- [Height Utility Classes](https://tailwindcss.com/docs/height): This page outlines height utility classes in Tailwind CSS, offering guidance on usage and customization.
- [List Style Image Documentation](https://tailwindcss.com/docs/list-style-image): Guide for using list style image utilities in Tailwind CSS.
- [Tailwind CSS Isolation Guide](https://tailwindcss.com/docs/isolation): Provides guidance on using Tailwind CSS's isolation utilities to manage stacking contexts in web design.
- [Content Configuration Guide](https://tailwindcss.com/docs/content-configuration): Guide for configuring content paths in Tailwind CSS projects to generate necessary styles.
- [Object Position Utilities](https://tailwindcss.com/docs/object-position): Explains how to use Tailwind CSS utilities for object positioning in web design.
- [Catalyst UI Kit Update](https://tailwindcss.com/blog/2024-05-24-catalyst-application-layouts): Announcing updates and features for the Catalyst UI kit for React, including new layouts and components.
- [Text Decoration Styles](https://tailwindcss.com/docs/text-decoration-style): Details how to apply different text decoration styles using Tailwind CSS utilities.
- [Adding Custom Styles](https://tailwindcss.com/docs/adding-custom-styles): Guide on customizing and adding styles in Tailwind CSS projects.
- [Spacing Utilities Overview](https://tailwindcss.com/docs/space): Describes Tailwind CSS utilities for managing spacing between elements in a layout.
- [Break Inside Utility Guide](https://tailwindcss.com/docs/break-inside): Guide on using break-inside utilities in Tailwind CSS for layout control.
- [Tailwind CSS Theme Customization](https://tailwindcss.com/docs/theme): Guide for customizing the Tailwind CSS theme configuration in projects.
- [Tailwind CSS Columns Guide](https://tailwindcss.com/docs/columns): Guide on using column utilities in Tailwind CSS for layout design.
- [Justify Content in Tailwind](https://tailwindcss.com/docs/justify-content): To explain the use of justify-content utility classes in Tailwind CSS for flex and grid layouts.
- [Tailwind CSS Installation Guide](https://tailwindcss.com/docs/installation): Guide for installing and setting up Tailwind CSS.
- [Customizing Tailwind Colors](https://tailwindcss.com/docs/customizing-colors): Guide for customizing color palettes in Tailwind CSS.
- [Flex Shrink Documentation](https://tailwindcss.com/docs/flex-shrink): This page details the usage and options for the 'flex-shrink' utility in Tailwind CSS.
- [Texto en Tailwind CSS](https://tailwindcss.com/docs/text-color): Explains how to set and customize text color in Tailwind CSS.
- [List Style Type Guide](https://tailwindcss.com/docs/list-style-type): Guide on using Tailwind CSS for different list style types in web development.
- [Positioning in Tailwind CSS](https://tailwindcss.com/docs/top-right-bottom-left): Explains how to use Tailwind CSS utilities for positioning elements with top, right, bottom, and left classes.
- [Container Class Documentation](https://tailwindcss.com/docs/container): Explains how to use the Tailwind CSS container class for responsive design.
- [Min Width Documentation](https://tailwindcss.com/docs/min-width): Describes how to use the minimum width utility in Tailwind CSS.
- [Font Variant Numeric Guide](https://tailwindcss.com/docs/font-variant-numeric): Provide documentation for using font variant numeric utilities in Tailwind CSS.
- [Tailwind CSS Showcase](https://tailwindcss.com/showcase): Showcases various websites built with Tailwind CSS to inspire developers.
- [Object Fit Utilities](https://tailwindcss.com/docs/object-fit): Explains usage of Tailwind CSS utilities for controlling object fit in responsive design.

### Svelte 5 + Skeleton UI

Reference the official Skeleton UI documentation for Svelte 5 components:
- **Skeleton UI Svelte 5 Guide**: https://www.skeleton.dev/llms-svelte.txt

## Project Architecture

### Database Layer (PostgreSQL + PostGIS)
```sql
--  Spatial data patterns
CREATE TABLE airports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    location GEOGRAPHY(POINT, 4326) NOT NULL,
    elevation_ft INTEGER
);

--  Indexes for spatial queries
CREATE INDEX CONCURRENTLY idx_airports_location ON airports USING GIST (location);
```

### API Layer (Rust + Axum)
```rust
//  Route structure
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub nats_client: Arc<async_nats::Client>,
}

//  Handler patterns
pub async fn get_devices(
    State(state): State<AppState>,
    Query(params): Query<DeviceSearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    // Implementation
}
```

### Frontend Layer (SvelteKit + TypeScript)
```svelte
<!--  Component structure -->
<script lang="ts">
    import { Search, Filter } from '@lucide/svelte';
    import { Segment } from '@skeletonlabs/skeleton-svelte';

    let searchQuery = '';

    function handleSearch() {
        // Implementation using onclick, not on:click
    }
</script>

<button onclick={handleSearch} class="btn variant-filled-primary">
    <Search class="h-4 w-4" />
    Search
</button>
```

### Real-time Features (NATS)
```rust
//  NATS message patterns
#[derive(Serialize, Deserialize)]
pub struct LiveFix {
    pub device_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: String,
}
```

## Code Quality Standards

### Pre-commit Hooks (REQUIRED)
All changes must pass these checks locally:

1. **Rust Quality**:
   - `cargo fmt --check` (formatting)
   - `cargo clippy --all-targets --all-features -- -D warnings` (linting)
   - `cargo test --verbose` (unit tests)
   - `cargo audit` (security audit)

2. **Frontend Quality**:
   - `npm run lint` (ESLint + Prettier)
   - `npm run check` (TypeScript validation)
   - `npm test` (Playwright E2E tests)
   - `npm run build` (build verification)

3. **File Quality**:
   - No trailing whitespace
   - Proper file endings
   - Valid YAML/JSON/TOML syntax

### Development Workflow
```bash
#  Proper development cycle
git checkout -b feature/new-feature
# Make changes
pre-commit run --all-files  # Verify quality
git add .
git commit -m "feat: add new feature"  # Pre-commit runs automatically
git push origin feature/new-feature
```

## Common Patterns

### Error Handling
```rust
//  Rust error handling
use anyhow::{Context, Result};

pub async fn process_data() -> Result<ProcessedData> {
    let data = fetch_data()
        .await
        .context("Failed to fetch data")?;

    Ok(process(data))
}
```

```typescript
//  TypeScript error handling
try {
    const response = await serverCall<DeviceResponse>('/devices');
    devices = response.devices || [];
} catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Unknown error';
    error = `Failed to load devices: ${errorMessage}`;
}
```

### State Management
```svelte
<!--  Svelte stores -->
<script lang="ts">
    import { writable } from 'svelte/store';

    const deviceStore = writable<Device[]>([]);

    // Use $deviceStore for reactive access
</script>
```

### API Integration
```typescript
//  Server communication
import { serverCall } from '$lib/api/server';

const response = await serverCall<DeviceListResponse>('/devices', {
    method: 'GET',
    params: { limit: 50 }
});
```

## Security Requirements

1. **Input Validation**: All user inputs must be validated
2. **SQL Injection Prevention**: Use Diesel ORM query builder
3. **XSS Prevention**: Proper HTML escaping in Svelte
4. **Authentication**: JWT tokens for API access
5. **HTTPS Only**: All production traffic encrypted

## Testing Requirements

### Rust Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_search() {
        // Test implementation
    }
}
```

### Frontend Tests
```typescript
// Playwright E2E tests
import { test, expect } from '@playwright/test';

test('device search functionality', async ({ page }) => {
    await page.goto('/devices');
    await expect(page.locator('h1')).toContainText('Aircraft Devices');
});
```

## Performance Guidelines

1. **Database**: Use proper indexes, limit query results
2. **Frontend**: Lazy loading, virtual scrolling for large lists
3. **API**: Pagination for large datasets
4. **Real-time**: Efficient NATS subscription management

---

**Remember**: This project maintains high code quality standards. All changes must pass pre-commit hooks and CI/CD pipeline. When in doubt, check existing patterns and follow established conventions.
- The rust backend for this project is in src/ and the frontend is a Svelte 5 project in web/
