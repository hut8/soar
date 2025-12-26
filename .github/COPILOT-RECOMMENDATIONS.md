# GitHub Copilot Optimization Recommendations

This document provides additional recommendations to make GitHub Copilot work more efficiently with the SOAR project.

## Files Created

1. **`.github/copilot-instructions.md`** - Project-specific coding patterns, conventions, and critical rules
2. **`.github/copilot-setup-steps.yml`** - Complete development environment setup guide

## Recommendations for Further Optimization

### 1. IDE Extensions for Better Copilot Integration

#### VS Code Extensions (Recommended)
- **GitHub Copilot** - Core AI pair programmer
- **GitHub Copilot Chat** - Conversational AI assistance
- **rust-analyzer** - Advanced Rust language support with inline type hints
- **Svelte for VS Code** - Svelte 5 syntax highlighting and IntelliSense
- **Tailwind CSS IntelliSense** - Autocomplete for Tailwind classes
- **PostgreSQL** - Database query assistance
- **Error Lens** - Inline error and warning display
- **Better Comments** - Enhanced comment highlighting
- **Code Spell Checker** - Catch typos in code and comments

#### VS Code Settings for Copilot
Add to `.vscode/settings.json`:
```json
{
  "github.copilot.enable": {
    "*": true,
    "yaml": true,
    "plaintext": false,
    "markdown": true,
    "rust": true,
    "typescript": true,
    "svelte": true
  },
  "github.copilot.advanced": {
    "debug.overrideEngine": "gpt-4",
    "inlineSuggestCount": 3
  },
  "editor.inlineSuggest.enabled": true,
  "editor.suggest.preview": true,
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.inlayHints.enable": true,
  "svelte.enable-ts-plugin": true
}
```

### 2. Workspace Configuration

Create `.vscode/settings.json` (if it doesn't exist):
```json
{
  "files.associations": {
    "*.rs": "rust",
    "*.svelte": "svelte",
    "Dockerfile*": "dockerfile"
  },
  "search.exclude": {
    "**/node_modules": true,
    "**/target": true,
    "**/.git": true,
    "**/web/build": true
  },
  "files.watcherExclude": {
    "**/target/**": true,
    "**/node_modules/**": true,
    "**/.git/objects/**": true
  },
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  },
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode",
    "editor.formatOnSave": true
  },
  "[svelte]": {
    "editor.defaultFormatter": "svelte.svelte-vscode",
    "editor.formatOnSave": true
  },
  "rust-analyzer.cargo.buildScripts.enable": true,
  "rust-analyzer.procMacro.enable": true
}
```

### 3. Add Type Hints Files for Better Context

#### Create `.github/copilot-context.yml`
```yaml
# Additional context for GitHub Copilot
project_type: full_stack_web_application
primary_languages:
  - rust
  - typescript
  - svelte
frameworks:
  - axum: web framework
  - diesel: ORM for PostgreSQL
  - sveltekit: frontend framework
  - tailwindcss: styling
architecture: microservices
patterns:
  - repository_pattern
  - event_driven
  - pub_sub_messaging
database: postgresql_with_postgis
```

### 4. Improve Code Discoverability

#### Add JSDoc/TSDoc comments to TypeScript files
GitHub Copilot uses these for better suggestions:

```typescript
/**
 * Fetches aircraft data from the API with optional filtering
 * @param {string} searchQuery - Search term to filter devices
 * @param {number} limit - Maximum number of results to return
 * @returns {Promise<AircraftResponse>} Array of aircraft matching the search
 * @example
 * const aircraft = await fetchAircraft('FLRDD', 50);
 */
export async function fetchAircraft(searchQuery: string, limit: number): Promise<AircraftResponse> {
    // implementation
}
```

#### Add doc comments to Rust functions
```rust
/// Processes an APRS aircraft position message and updates the database
///
/// # Arguments
/// * `packet` - Parsed APRS packet containing position data
/// * `conn` - Database connection pool
///
/// # Returns
/// * `Result<Fix>` - The processed fix record or an error
///
/// # Errors
/// Returns error if database operation fails or packet is invalid
pub async fn process_aircraft_position(
    packet: AprsPacket,
    conn: &PgConnection,
) -> Result<Fix> {
    // implementation
}
```

### 5. Create Example Files for Common Patterns

#### Add `.github/examples/api-endpoint.rs`
```rust
// Example: Creating a new API endpoint in SOAR

use axum::{
    extract::{State, Query},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<Device>,
    pub total: i64,
}

pub async fn search_devices(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = params.limit.unwrap_or(50);

    let devices = state.device_repo
        .search(&params.query, limit)
        .await
        .context("Failed to search devices")?;

    Ok(Json(SearchResponse {
        results: devices.clone(),
        total: devices.len() as i64,
    }))
}
```

#### Add `.github/examples/svelte-component.svelte`
```svelte
<!-- Example: Creating a new Svelte 5 component in SOAR -->

<script lang="ts">
    import { Search } from '@lucide/svelte';
    import { Button } from '@skeletonlabs/skeleton-svelte';
    import { serverCall } from '$lib/api/server';

    interface Props {
        initialQuery?: string;
    }

    let { initialQuery = '' }: Props = $props();
    let searchQuery = $state(initialQuery);
    let results = $state<Device[]>([]);
    let loading = $state(false);
    let error = $state<string | null>(null);

    async function handleSearch() {
        loading = true;
        error = null;

        try {
            const response = await serverCall<SearchResponse>('/devices/search', {
                method: 'GET',
                params: { query: searchQuery, limit: 50 }
            });
            results = response.results;
        } catch (err) {
            error = err instanceof Error ? err.message : 'Search failed';
        } finally {
            loading = false;
        }
    }
</script>

<div class="search-container">
    <div class="flex gap-2">
        <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search devices..."
            class="input"
            onkeydown={(e) => e.key === 'Enter' && handleSearch()}
        />
        <Button onclick={handleSearch} disabled={loading}>
            <Search class="h-4 w-4" />
            Search
        </Button>
    </div>

    {#if loading}
        <p>Loading...</p>
    {:else if error}
        <p class="text-error-500">{error}</p>
    {:else if results.length > 0}
        <div class="results-grid">
            {#each results as device (device.device_id)}
                <div class="device-card">
                    <!-- Device info -->
                </div>
            {/each}
        </div>
    {/if}
</div>
```

### 6. Git Configuration for Better Context

Add to `.gitattributes`:
```
# Ensure consistent line endings
* text=auto

# Mark generated files
target/ linguist-generated=true
web/build/ linguist-generated=true
web/.svelte-kit/ linguist-generated=true

# Language hints for GitHub
*.rs linguist-language=Rust
*.svelte linguist-language=Svelte
```

### 7. Add Schema Documentation

Create `.github/docs/database-schema.md` with entity relationship details:
```markdown
# Database Schema

## Core Entities

### devices
- Primary table for aircraft/gliders
- Includes FAA registration data
- Links to club memberships

### fixes
- Aircraft position reports
- Partitioned by timestamp for performance
- Includes calculated AGL altitude

### flights
- Detected flight segments
- Links device + start/end fixes
- Used for analytics

### receivers
- OGN/APRS ground stations
- Tracks coverage and reliability
```

This helps Copilot understand relationships when generating queries.

### 8. Use Copilot Chat Effectively

#### Slash Commands
- `/explain` - Explain selected code
- `/fix` - Suggest fixes for errors
- `/tests` - Generate test cases
- `/doc` - Generate documentation

#### Effective Prompts
Instead of: "Add a function"
Use: "Add a Diesel query function that finds all devices within 50km of a coordinate using PostGIS ST_DWithin"

Instead of: "Make this component"
Use: "Create a Svelte 5 component that displays aircraft on a map using Leaflet, with onclick handlers (not on:click) and real-time updates via WebSocket"

### 9. Project-Specific Copilot Patterns

Create `.github/copilot-patterns.md`:
```markdown
# SOAR-Specific Copilot Patterns

When Copilot suggests code, look for these patterns:

## ✅ Good Suggestions
- Uses Diesel query builder
- Includes proper error handling with anyhow
- Uses tracing for logging
- Svelte 5 syntax (onclick, not on:click)
- Static typing throughout

## ❌ Bad Suggestions to Reject
- Raw SQL queries
- Missing error contexts
- println! for logging
- Svelte 4 syntax (on:click)
- SSR-specific code
```

### 10. Continuous Improvement

#### Track Copilot Effectiveness
- Monitor which suggestions you accept vs. reject
- Update `.github/copilot-instructions.md` when patterns emerge
- Document new patterns in CLAUDE.md

#### Team Knowledge Sharing
- Share effective prompts in team chat
- Document non-obvious patterns
- Review Copilot suggestions in code reviews

### 11. Advanced: Custom Copilot Workspace Config

Create `.github/.copilot/workspace.yml` (experimental):
```yaml
# Workspace-specific Copilot configuration
context_files:
  - CLAUDE.md
  - README.md
  - src/schema.rs
  - web/src/lib/api/types.ts

preferred_languages:
  - rust
  - typescript

ignore_patterns:
  - "**/target/**"
  - "**/node_modules/**"
  - "**/*.log"
```

### 12. Performance Optimization Tips

1. **Close unused files** - Copilot uses open files as context
2. **Keep related files open** - When editing Rust, keep schema.rs open
3. **Use descriptive variable names** - Helps Copilot understand intent
4. **Write comments before code** - Describe what you want, then let Copilot suggest
5. **Break down complex functions** - Smaller functions = better suggestions

## Measuring Success

Track these metrics to see if optimizations help:
- Copilot suggestion acceptance rate
- Time to implement new features
- Code review feedback on Copilot-generated code
- Test pass rate for Copilot-generated code

## Support and Feedback

If you find patterns that work well or areas where Copilot struggles:
1. Update `.github/copilot-instructions.md` with new patterns
2. Share findings in team documentation
3. Consider contributing examples to this file

## References

- [GitHub Copilot Documentation](https://docs.github.com/en/copilot)
- [Copilot Best Practices](https://github.blog/2023-06-20-how-to-write-better-prompts-for-github-copilot/)
- [VS Code Copilot Settings](https://code.visualstudio.com/docs/editor/github-copilot)
