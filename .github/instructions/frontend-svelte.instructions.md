---
applyTo: "web/src/**/*.{ts,svelte}"
---

# Frontend (SvelteKit 5 + TypeScript) Standards

## CRITICAL: Svelte 5 Syntax

**ALWAYS use Svelte 5 inline event handlers, NEVER Svelte 4 syntax**

```svelte
<!-- ✅ CORRECT: Use Svelte 5 inline event handlers -->
<button onclick={handleClick}>Click me</button>
<input oninput={handleInput} onkeydown={handleKeydown} />
<div onmouseenter={handleHover} onmouseleave={handleLeave} />

<!-- ❌ WRONG: Don't use Svelte 4 syntax -->
<button on:click={handleClick}>Click me</button>
<input on:input={handleInput} />
```

## Component Structure

### Svelte 5 Runes

```svelte
<script lang="ts">
    import { Search } from '@lucide/svelte';
    import { Button } from '@skeletonlabs/skeleton-svelte';
    import { serverCall } from '$lib/api/server';

    interface Props {
        initialQuery?: string;
        limit?: number;
    }

    // Props with defaults
    let { initialQuery = '', limit = 50 }: Props = $props();

    // State
    let searchQuery = $state(initialQuery);
    let results = $state<Device[]>([]);
    let loading = $state(false);
    let error = $state<string | null>(null);

    // Derived state
    let hasResults = $derived(results.length > 0);

    // Effects
    $effect(() => {
        if (searchQuery) {
            handleSearch();
        }
    });

    async function handleSearch() {
        loading = true;
        error = null;

        try {
            const response = await serverCall<SearchResponse>('/devices/search', {
                method: 'GET',
                params: { query: searchQuery, limit }
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

    {#if loading}
        <p>Loading...</p>
    {:else if error}
        <p class="text-error-500">{error}</p>
    {:else if hasResults}
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

## Icons

**ALWAYS use @lucide/svelte for icons, NEVER use other icon libraries**

```svelte
<script lang="ts">
    import { Search, User, Settings, ChevronDown, X, Check } from '@lucide/svelte';
</script>

<Search class="h-4 w-4" />
<User class="h-6 w-6 text-primary-500" />
<Settings class="h-5 w-5" />
```

## UI Components

**Use Skeleton UI components from @skeletonlabs/skeleton-svelte**

```svelte
<script lang="ts">
    import { Button, Modal, Card } from '@skeletonlabs/skeleton-svelte';
</script>

<Button variant="filled-primary">Primary Action</Button>
<Button variant="ghost">Secondary Action</Button>

<Modal bind:open={modalOpen}>
    <h2>Modal Title</h2>
    <p>Modal content</p>
</Modal>
```

## Static Site Generation (CRITICAL)

**NO Server-Side Rendering (SSR) - frontend must be fully static**

```typescript
// +page.ts
export const ssr = false;
export const prerender = true;

// All pages work as pure client-side SPA
// Authentication handled client-side
```

## API Communication

```typescript
import { serverCall } from '$lib/api/server';

// GET request
const response = await serverCall<AircraftResponse>('/devices', {
    method: 'GET',
    params: { limit: 50 }
});

// POST request
const result = await serverCall<CreateResponse>('/devices', {
    method: 'POST',
    body: { device_id: 'FLRDD1234', name: 'Glider 1' }
});
```

## Error Handling

```typescript
try {
    const response = await serverCall<AircraftResponse>('/devices');
    devices = response.devices || [];
} catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Unknown error';
    error = `Failed to load devices: ${errorMessage}`;
    console.error('Device fetch error:', err);
}
```

## TypeScript Types

```typescript
// Define interfaces for API responses
interface Device {
    device_id: string;
    name: string;
    aircraft_type?: string;
    registration?: string;
}

interface AircraftResponse {
    devices: Device[];
    total: number;
}

// Use type guards
function isDevice(obj: unknown): obj is Device {
    return (
        typeof obj === 'object' &&
        obj !== null &&
        'device_id' in obj &&
        typeof (obj as Device).device_id === 'string'
    );
}
```

## Styling with Tailwind CSS

```svelte
<div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold text-surface-900 dark:text-surface-50">
        Aircraft Tracker
    </h1>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each devices as device}
            <div class="card p-4 variant-ghost-surface">
                <h2 class="h3">{device.name}</h2>
                <p class="text-surface-600 dark:text-surface-400">
                    {device.device_id}
                </p>
            </div>
        {/each}
    </div>
</div>
```

## File Naming Conventions

- **Routes**: `+page.svelte`, `+page.ts`, `+layout.svelte`, `+layout.ts`
- **Components**: `PascalCase.svelte` (e.g., `DeviceCard.svelte`)
- **Utilities**: `kebab-case.ts` (e.g., `api-client.ts`)
- **Types**: `kebab-case.ts` (e.g., `device-types.ts`)

## Common Pitfalls to Avoid

1. ❌ Using Svelte 4 syntax (`on:click`) instead of Svelte 5 (`onclick`)
2. ❌ Enabling SSR in SvelteKit pages (must be static only)
3. ❌ Using icon libraries other than `@lucide/svelte`
4. ❌ Not handling errors in API calls
5. ❌ Missing TypeScript types for API responses
6. ❌ Not using `$state`, `$derived`, or `$props` runes in Svelte 5

## Before Committing

- Run `npm run lint` to check for linting errors
- Run `npm run check` to verify TypeScript types
- Run `npm test` to run E2E tests
- Test in browser to ensure functionality works
