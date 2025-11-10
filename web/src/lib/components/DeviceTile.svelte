<script lang="ts">
	import { Radio, Plane, Antenna, Check, X, Activity, Map, Navigation } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import {
		getAircraftTypeOgnDescription,
		getAircraftTypeColor,
		getDeviceTitle
	} from '$lib/formatters';
	import type { Device } from '$lib/types';

	let { device }: { device: Device } = $props();

	// Get the most recent fix with location data
	let latestFix = $derived(
		device.fixes && device.fixes.length > 0
			? device.fixes.find((fix) => fix.latitude && fix.longitude)
			: null
	);

	// Check if device has an active flight (most recent fix is active and has a flight_id)
	let hasActiveFlight = $derived(latestFix?.active && latestFix?.flight_id ? true : false);

	// Build the operations map URL with location parameters
	let mapUrl = $derived(
		latestFix ? `/operations?lat=${latestFix.latitude}&lng=${latestFix.longitude}&zoom=13` : null
	);

	// Build the flight detail URL
	let flightUrl = $derived(
		hasActiveFlight && latestFix?.flight_id ? `/flights/${latestFix.flight_id}` : null
	);
</script>

<div class="card preset-tonal-primary p-4">
	<a href={resolve(`/devices/${device.id}`)} class="group block transition-all hover:scale-[1.02]">
		<!-- Header Section -->
		<div class="mb-4 flex items-start justify-between">
			<div class="flex items-center gap-2">
				<Radio class="h-5 w-5 text-primary-500" />
				<h3 class="text-lg font-semibold">{getDeviceTitle(device)}</h3>
			</div>
		</div>

		<!-- Registration and Model -->
		<div class="mb-4 space-y-2">
			<div class="flex items-center gap-2">
				<Plane class="h-4 w-4 text-surface-500" />
				<div>
					<p class="text-surface-600-300-token text-xs">Registration</p>
					<p class="text-sm font-semibold">
						{device.registration || 'Unknown'}
					</p>
				</div>
			</div>
			<div class="flex items-center gap-2">
				<Antenna class="h-4 w-4 text-surface-500" />
				<div>
					<p class="text-surface-600-300-token text-xs">Aircraft Model</p>
					<p class="text-sm">{device.aircraft_model || 'Unknown'}</p>
				</div>
			</div>
			{#if device.competition_number}
				<div class="flex items-center gap-2">
					<Activity class="h-4 w-4 text-surface-500" />
					<div>
						<p class="text-surface-600-300-token text-xs">Competition Number</p>
						<p class="font-mono text-sm">{device.competition_number}</p>
					</div>
				</div>
			{/if}
		</div>

		<!-- Status Badges -->
		<div class="flex flex-wrap gap-2">
			<span
				class="badge text-xs {device.tracked
					? 'preset-filled-success-500'
					: 'preset-filled-surface-500'}"
			>
				{#if device.tracked}
					<Check class="mr-1 h-3 w-3" />
				{:else}
					<X class="mr-1 h-3 w-3" />
				{/if}
				{device.tracked ? 'Tracked' : 'Not Tracked'}
			</span>
			<span
				class="badge text-xs {device.identified
					? 'preset-filled-primary-500'
					: 'preset-filled-surface-500'}"
			>
				{#if device.identified}
					<Check class="mr-1 h-3 w-3" />
				{:else}
					<X class="mr-1 h-3 w-3" />
				{/if}
				{device.identified ? 'Identified' : 'Unidentified'}
			</span>
			{#if device.from_ddb}
				<span class="badge preset-filled-success-500 text-xs">
					<Check class="mr-1 h-3 w-3" />
					OGN DB
				</span>
			{/if}
			{#if device.aircraft_type_ogn}
				<span class="badge {getAircraftTypeColor(device.aircraft_type_ogn)} text-xs">
					{getAircraftTypeOgnDescription(device.aircraft_type_ogn)}
				</span>
			{/if}
		</div>
	</a>

	<!-- Action Buttons -->
	<div class="mt-4 flex flex-wrap gap-2">
		{#if mapUrl}
			<a
				href={mapUrl}
				class="btn flex-1 preset-filled-secondary-500 btn-sm"
				onclick={(e) => e.stopPropagation()}
			>
				<Map class="h-4 w-4" />
				<span>View on Map</span>
			</a>
		{/if}
		{#if flightUrl}
			<a
				href={flightUrl}
				class="btn flex-1 preset-filled-primary-500 btn-sm"
				onclick={(e) => e.stopPropagation()}
			>
				<Navigation class="h-4 w-4" />
				<span>Current Flight</span>
			</a>
		{/if}
	</div>
</div>
