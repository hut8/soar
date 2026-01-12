<script lang="ts">
	import { onMount } from 'svelte';
	import { Plane, ExternalLink } from '@lucide/svelte';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { serverCall } from '$lib/api/server';
	import type { Aircraft } from '$lib/types';
	import AircraftLink from '$lib/components/AircraftLink.svelte';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'TowAircraftLink']);

	export let aircraftId: string;
	export let size: 'sm' | 'md' | 'lg' = 'md';

	let aircraft: Aircraft | null = null;
	let loading = true;

	// Size classes
	const sizeClasses = {
		sm: 'text-xs',
		md: 'text-sm',
		lg: 'text-base'
	};

	const iconSizes = {
		sm: 'h-3 w-3',
		md: 'h-4 w-4',
		lg: 'h-5 w-5'
	};

	onMount(async () => {
		// Try to get aircraft from registry cache first
		const registry = AircraftRegistry.getInstance();
		aircraft = registry.getAircraft(aircraftId);

		if (aircraft) {
			loading = false;
			return;
		}

		// If not in cache, fetch from server
		try {
			aircraft = await serverCall<Aircraft>(`/aircraft/${aircraftId}`);
			loading = false;
		} catch (error) {
			logger.error('Failed to load aircraft {aircraftId}: {error}', { aircraftId, error });
			loading = false;
		}
	});
</script>

{#if loading}
	<span class="text-surface-500 {sizeClasses[size]}">Loading...</span>
{:else if aircraft}
	<AircraftLink {aircraft} {size} openInNewTab={true} showIcon={true} />
{:else}
	<a
		href="/aircraft/{aircraftId}"
		target="_blank"
		rel="noopener noreferrer"
		class="inline-flex items-center gap-1 anchor {sizeClasses[size]}"
		title="View towplane aircraft"
	>
		<Plane class={iconSizes[size]} />
		<ExternalLink class={iconSizes[size]} />
	</a>
{/if}
