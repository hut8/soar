# CLAUDE.md - AI Assistant Guide for SOAR Project

This document provides essential guidance for AI assistants working on the SOAR (Soaring Observation And Records) project.

## Project Overview

SOAR is a comprehensive aircraft tracking and club management system built with:

- **Backend**: Rust with Axum web framework, PostgreSQL with PostGIS
- **Frontend**: SvelteKit with TypeScript, Tailwind CSS, Skeleton UI components
- **Real-time**: NATS messaging for live aircraft position updates
- **Data Sources**: APRS-IS integration, FAA aircraft registry, airport databases

## Critical Development Rules

### DOCUMENTATION PRIORITY
- **KEEP DOCUMENTATION UP TO DATE** - Documentation (including README.md, CLAUDE.md, and other docs) must be updated when features change
- When renaming services, commands, or changing architecture, update all relevant documentation
- Documentation changes should be part of the same PR that changes the implementation
- Outdated documentation is a bug - treat it with the same priority as code bugs

### NO BYPASSING QUALITY CONTROLS
- **NEVER commit directly to main** - The main branch is protected. ALWAYS create a feature/topic branch first
- **NEVER use `git commit --no-verify`** - All commits must pass pre-commit hooks
- **NEVER push to main** - Pushing to feature branches is okay, but never push directly to main
- **NEVER skip CI checks** - Local development must match GitHub Actions pipeline
- **ASK BEFORE removing large amounts of working code** - Get confirmation before major deletions
- **AVOID duplicate code** - Check for existing implementations before writing new code
- Pre-commit hooks run: `cargo fmt`, `cargo clippy`, `cargo test`, `npm lint`, `npm check`, `npm test`

### COMMIT AND DATABASE RULES
- **NEVER add Co-Authored-By lines** - Do not include Claude Code attribution in commits
- **AVOID raw SQL in Diesel** - Only use raw SQL if absolutely necessary, and ask first before using it
- Always prefer Diesel's query builder and type-safe methods over raw SQL
- **NEVER use CREATE INDEX CONCURRENTLY in Diesel migrations** - Diesel migrations run in transactions, which don't support CONCURRENTLY. Use regular CREATE INDEX instead

### DATABASE SAFETY RULES (CRITICAL)
- **Development Database**: `soar_dev` - This is where you work
- **Production Database**: `soar` - This is read-only for development purposes
- **NEVER run UPDATE, INSERT, or DELETE on production database (`soar`)** - Only run these via Diesel migrations
- **ONLY DDL queries (CREATE, ALTER, DROP) via migrations** - Never run DDL queries manually on production
- **SELECT queries are allowed on both databases** - For investigation and analysis
- **All data modifications must go through migrations** - This ensures they're tracked and reproducible
- **Deleting data before adding constraints** - You can include DELETE statements in the same migration before constraint creation. The constraint validates against the final state of the transaction, so the DELETE will complete first.

### METRICS AND MONITORING

**CRITICAL - Grafana Dashboard Synchronization:**
- **ALWAYS update Grafana dashboards when changing metrics** - Any metric rename, addition, or removal MUST be reflected in the corresponding dashboard files in `infrastructure/`
- **Verify dashboard queries after changes** - After updating code, search all dashboard files for the old metric name and update them
- **Dashboard locations:**
  - **Run command (`soar run`)** - Split into focused sub-dashboards:
    - `infrastructure/grafana-dashboard-run-core.json` - Core system (process, database, NATS publisher, latency)
    - `infrastructure/grafana-dashboard-run-ingestion.json` - Data ingestion (OGN, Beast/ADS-B)
    - `infrastructure/grafana-dashboard-run-routing.json` - Packet processing and routing
    - `infrastructure/grafana-dashboard-run-flights.json` - Aircraft and flight tracking
    - `infrastructure/grafana-dashboard-run-geocoding.json` - Pelias geocoding service
    - `infrastructure/grafana-dashboard-run-elevation.json` - Elevation processing and AGL
  - `infrastructure/grafana-dashboard-ingest-ogn.json` - OGN/APRS ingestion (`ingest-ogn` command)
  - `infrastructure/grafana-dashboard-ingest-adsb.json` - ADS-B Beast ingestion (`ingest-adsb` command)
  - `infrastructure/grafana-dashboard-web.json` - Web server (`web` command)
  - `infrastructure/grafana-dashboard-nats.json` - NATS/JetStream metrics
  - `infrastructure/grafana-dashboard-analytics.json` - Analytics API and cache performance

**Metric Standards:**
- **Naming convention** - Use dot notation (e.g., `aprs.aircraft.device_upsert_ms`)
- **Document metric changes** - Note metric name changes in PR description for ops team awareness
- **Remove obsolete dashboard queries** - If a metric is removed from code, remove it from dashboards too

**Recent Metric Changes:**
- `aprs.aircraft.aircraft_lookup_ms` → `aprs.aircraft.aircraft_upsert_ms` (2025-01-07, PR #312)
  - Updated in code and Grafana dashboard (2025-01-12)
- **REMOVED**: `aprs.elevation.dropped` and `nats_publisher.dropped_fixes` (2025-01-12)
  - These metrics were removed from dashboard as messages can no longer be dropped

**Grafana Alerting:**
- **Alert Configuration** - Managed via infrastructure as code in `infrastructure/grafana-provisioning/alerting/`
- **Email Notifications** - Alerts sent via SMTP (credentials from `/etc/soar/env` or `/etc/soar/env-staging`)
- **Template Files** - Use `.template` suffix for files with credential placeholders (e.g., `contact-points.yml.template`)
- **Deployment** - `soar-deploy` script automatically processes templates and installs configs
- **Documentation** - See `infrastructure/GRAFANA-ALERTING.md` for complete guide
- **Active Alerts:**
  - OGN Message Ingestion Rate Too Low (critical: < 1 msg/min for 2 minutes)
  - OGN Ingest Service Disconnected (critical: connection gauge = 0)
  - OGN NATS Publishing Errors (warning: error rate > 0.1/min for 3 minutes)
- **NEVER commit credentials** - Template files use placeholders, actual values extracted during deployment

### Frontend Development Standards

#### Static Site Generation (CRITICAL)
- **NO Server-Side Rendering (SSR) ANYWHERE** - The frontend MUST be compiled statically
- Use `export const ssr = false;` in `+page.ts` files to disable SSR for specific pages
- The compiled static site is embedded in the Rust binary for deployment
- All pages must work as a pure client-side Single Page Application (SPA)
- Authentication and route protection must be handled client-side

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
pub async fn get_aircraft(
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
    pub aircraft_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: String,
}
```

### Analytics Layer
SOAR includes a comprehensive analytics system for tracking flight statistics and performance metrics.

**Database Tables:**
- `flight_analytics_daily` - Daily flight aggregations with automatic triggers
- `flight_analytics_hourly` - Hourly flight statistics for recent trends
- Materialized automatically via PostgreSQL triggers on flight insert/update

**Backend Components:**
- `src/analytics.rs` - Core data models for analytics responses
- `src/analytics_repo.rs` - Database queries using Diesel ORM
- `src/analytics_cache.rs` - 60-second TTL cache using Moka
- `src/actions/analytics.rs` - REST API endpoints

**API Endpoints (`/data/analytics/...`):**
- `/flights/daily` - Daily flight counts and statistics
- `/flights/hourly` - Hourly flight trends
- `/flights/duration-distribution` - Flight duration buckets
- `/devices/outliers` - Devices with anomalous flight patterns (z-score > threshold)
- `/devices/top` - Top devices by flight count
- `/clubs/daily` - Club-level analytics
- `/airports/activity` - Airport usage statistics
- `/summary` - Dashboard summary with key metrics

**Caching Strategy:**
- All queries cached for 60 seconds to reduce database load
- Cache hit/miss metrics tracked in `analytics.cache.hit` and `analytics.cache.miss`
- Query latency tracked per endpoint (e.g., `analytics.query.daily_flights_ms`)

**Metrics & Monitoring:**
```rust
// Analytics API metrics
metrics::counter!("analytics.api.daily_flights.requests").increment(1);
metrics::counter!("analytics.api.errors").increment(1);
metrics::histogram!("analytics.query.daily_flights_ms").record(duration_ms);
metrics::counter!("analytics.cache.hit").increment(1);

// Background task updates these gauges every 60 seconds
metrics::gauge!("analytics.flights.today").set(summary.flights_today as f64);
metrics::gauge!("analytics.flights.last_7d").set(summary.flights_7d as f64);
metrics::gauge!("analytics.aircraft.active_7d").set(summary.active_aircraft_7d as f64);
```

**Grafana Dashboard:**
- Location: `infrastructure/grafana-dashboard-analytics.json`
- Tracks: API request rates, cache hit rates, query latency percentiles, error rates
- Background task runs every 60 seconds to update summary metrics

**Adding New Analytics:**
1. Add database query to `analytics_repo.rs`
2. Add caching method to `analytics_cache.rs` with metrics
3. Add API handler to `actions/analytics.rs` with request/error metrics
4. Register route in `web.rs`
5. Add metrics to `metrics.rs::initialize_analytics_metrics()`
6. Update Grafana dashboard with new metric queries

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

### MCP Server Setup (Optional - For Claude Code Database Access)

Claude Code can connect directly to the PostgreSQL database using the **pgEdge Postgres MCP Server**, enabling natural language database queries and schema introspection.

**Installation:**

1. **Clone and build the MCP server:**
   ```bash
   cd /tmp
   git clone https://github.com/pgEdge/pgedge-postgres-mcp.git
   cd pgedge-postgres-mcp
   go build -v -o bin/pgedge-postgres-mcp ./cmd/pgedge-pg-mcp-svr
   cp bin/pgedge-postgres-mcp ~/.local/bin/
   ```

2. **Configure for your project:**
   ```bash
   # Copy the example configuration
   cp .mcp.json.example .mcp.json

   # Edit .mcp.json with your settings
   # Update the "command" path to where you installed the binary
   # Update "PGUSER" to your PostgreSQL username
   ```

3. **Restart Claude Code** to load the MCP server

**What you get:**
- 🔍 Schema introspection - Query tables, columns, indexes, constraints
- 📊 Database queries - Execute SQL queries (read-only for safety)
- 📈 Performance metrics - Access `pg_stat_statements` and other stats
- 🧠 Natural language queries - Ask questions about the database in plain English

**Security Notes:**
- The MCP server runs in **read-only mode** by default
- `.mcp.json` is gitignored to prevent committing local paths and credentials
- Use `.pgpass` file for password management instead of storing in `.mcp.json`

**Documentation:** https://www.pgedge.com/blog/introducing-the-pgedge-postgres-mcp-server

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
    const response = await serverCall<AircraftResponse>('/devices');
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

    const aircraftStore = writable<Aircraft[]>([]);

    // Use $aircraftStore for reactive access
</script>
```

### API Integration
```typescript
//  Server communication
import { serverCall } from '$lib/api/server';

const response = await serverCall<AircraftListResponse>('/devices', {
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
    async fn test_aircraft_search() {
        // Test implementation
    }
}
```

### Frontend E2E Tests (Playwright)

#### Overview
- **Framework**: Playwright v1.56+
- **Test Directory**: `web/e2e/`
- **Documentation**: See `web/e2e/README.md` for comprehensive guide

#### Running Tests
```bash
cd web
npm test              # Run all E2E tests
npx playwright test --ui   # Interactive UI mode
npx playwright test --debug # Debug mode
```

#### Test Structure
```
e2e/
├── fixtures/           # Test fixtures and setup
│   ├── auth.fixture.ts # Pre-authenticated contexts
│   └── data.fixture.ts # Test data constants
├── utils/              # Reusable utilities
│   ├── auth.ts        # Login/logout helpers
│   └── navigation.ts  # Navigation helpers
├── auth/               # Authentication tests
│   ├── login.test.ts
│   ├── register.test.ts
│   └── logout.test.ts
└── devices/            # Aircraft tests
    ├── aircraft-list.test.ts
    └── aircraft-detail.test.ts
```

#### Writing E2E Tests

**Basic Test Pattern:**
```typescript
import { test, expect } from '@playwright/test';

test.describe('Feature Name', () => {
    test('should do something', async ({ page }) => {
        await page.goto('/page');
        await expect(page.getByRole('heading')).toBeVisible();
    });
});
```

**Using Authentication Fixture:**
```typescript
import { test, expect } from '../fixtures/auth.fixture';

test('authenticated test', async ({ authenticatedPage }) => {
    // Page is already logged in
    await authenticatedPage.goto('/devices');
});
```

**Using Helper Functions:**
```typescript
import { login } from '../utils/auth';
import { searchDevicesByRegistration } from '../utils/navigation';

test('search devices', async ({ page }) => {
    await login(page, 'test@example.com', 'password');
    await searchDevicesByRegistration(page, 'N12345');
});
```

#### Visual Regression Testing

Tests include screenshot comparison for visual regression detection:

```typescript
test('visual test', async ({ page }) => {
    await page.goto('/devices');
    // First run creates baseline, subsequent runs compare
    await expect(page).toHaveScreenshot('devices-page.png');
});
```

**Update Screenshots:**
```bash
npx playwright test --update-snapshots
```

#### Best Practices

1. **Use Semantic Locators**: Prefer `getByRole()`, `getByPlaceholder()` over CSS selectors
2. **Wait for Network**: Use `waitForLoadState('networkidle')` after navigation
3. **Handle Dynamic Content**: Use thresholds for screenshot comparisons with variable data
4. **Test Critical Paths**: Focus on user journeys, not implementation details
5. **Keep Tests Independent**: Each test should be able to run in isolation

#### Test Coverage Goals

- ✅ **Authentication**: Login, registration, password reset
- ✅ **Devices**: Search, list, detail views
- 🚧 **Flights**: Flight tracking, details (TODO)
- 🚧 **Clubs**: Club management (TODO)
- 🚧 **Admin**: Administrative functions (TODO)

## Performance Guidelines

1. **Database**: Use proper indexes, limit query results
2. **Frontend**: Lazy loading, virtual scrolling for large lists
3. **API**: Pagination for large datasets
4. **Real-time**: Efficient NATS subscription management

---

**Remember**: This project maintains high code quality standards. All changes must pass pre-commit hooks and CI/CD pipeline. When in doubt, check existing patterns and follow established conventions.
- The rust backend for this project is in src/ and the frontend is a Svelte 5 project in web/
- You should absolutely never use --no-verify
- When running clippy or cargo build, set the timeout to ten minutes
- Use a timeout of 10 minutes for running "cargo test" or "cargo clippy"

## Branch Protection Rules

**CRITICAL**: The `main` branch is protected and does not allow direct commits.

### Always Use Feature Branches
- **NEVER commit directly to main** - Always create a feature/topic branch first
- Use descriptive branch names:
  - `feature/description` for new features
  - `fix/description` for bug fixes
  - `refactor/description` for code refactoring
  - `docs/description` for documentation changes

### Proper Development Workflow
1. **Create a topic branch**: `git checkout -b feature/my-feature`
2. **Make changes and commit**: Only stage specific files you modified
3. **Push to remote**: `git push origin feature/my-feature`
4. **Create Pull Request**: Use GitHub UI to create PR for review

### If You Accidentally Commit to Main
If you accidentally commit to main, follow these steps to fix it:
1. `git reset --hard HEAD~N` (where N is the number of commits to undo)
2. `git checkout -b topic/branch-name commit-hash` (create branch for each commit)
3. `git checkout main` (return to main)

Example:
```bash
# You accidentally made 3 commits to main
git log --oneline -3  # See the commits
# c3c3c3c third commit
# b2b2b2b second commit
# a1a1a1a first commit

# Undo the commits on main
git reset --hard HEAD~3

# Create branches for each
git checkout -b feature/third-feature c3c3c3c
git checkout -b feature/second-feature b2b2b2b
git checkout -b fix/first-fix a1a1a1a

# Return to main
git checkout main
```
- Any time that you have enter a filename in the shell, use quotes. We are using zsh, and shell expansion treats our Svelte pages incorrectly if you do not because "[]" has a special meaning.
- Whenever you commit, you should also push with -u to set the upstream branch.
- When opening a PR, do not set it to squash commits. Set it to create a merge commit.

## Release Process

**IMPORTANT**: Version numbers are automatically derived from git tags. Do NOT manually edit version numbers in `Cargo.toml` or `package.json`.

### How Versioning Works

- **Rust**: Version is derived from git tags at build time using the `vergen` crate
- **SvelteKit**: Version is generated from git tags by `web/scripts/generate-version.js` during prebuild
- **Source of Truth**: Git tags (format: `v0.1.5`)
- **No Version Commits**: Version files (`Cargo.toml`, `package.json`) contain placeholder values only

### Creating a Release

Use the simplified release script:

```bash
# Semantic version bump (recommended)
./scripts/create-release patch    # 0.1.4 → 0.1.5
./scripts/create-release minor    # 0.1.4 → 0.2.0
./scripts/create-release major    # 0.1.4 → 1.0.0

# Explicit version (if needed)
./scripts/create-release v0.1.5

# Create as draft (for review before publishing)
./scripts/create-release patch --draft

# Create with custom release notes
./scripts/create-release patch --notes "Custom release notes here"
```

**What happens automatically:**
1. ✅ Script creates GitHub Release with tag `v0.1.5`
2. ✅ CI builds x64 and ARM64 static binaries (version derived from tag)
3. ✅ CI runs all tests and security audits
4. ✅ CI deploys to production automatically (via `ci.yml` `deploy-production` job)

### Manual Release via GitHub CLI

```bash
# Alternative: Use gh CLI directly
gh release create v0.1.5 --generate-notes
```

### Version Format

- **Tagged release**: `v0.1.4`
- **Development build**: `v0.1.4-2-ge930185` (2 commits after v0.1.4)
- **Dirty working tree**: `v0.1.4-dirty`
- **No git repo**: `0.0.0-dev`

### Checking Current Version

```bash
# Binary version (shows git-derived version)
./target/release/soar --version

# Git describe (shows current version from git)
git describe --tags --always --dirty
```

### Old Release Process (Deprecated)

The old `./scripts/release` script has been archived to `./scripts/archive/release-old`. It is no longer needed because:
- ❌ Required creating release branch → PR → auto-merge → tag push
- ❌ Manually edited version files and committed changes
- ❌ Complex multi-step process with potential for errors

The new process eliminates all of this complexity.
