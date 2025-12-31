<script lang="ts">
	import { Radio, Plane, Antenna, Check, X, Activity, Map, Navigation } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import {
		getAircraftTypeOgnDescription,
		getAircraftTypeColor,
		getAircraftTitle
	} from '$lib/formatters';
	import type { Aircraft } from '$lib/types';

	let { aircraft }: { aircraft: Aircraft } = $props();

	// Build the operations map URL with location parameters from latest fix
	let mapUrl = $derived(
		aircraft.latitude && aircraft.longitude
			? `/operations?lat=${aircraft.latitude}&lng=${aircraft.longitude}&zoom=13`
			: null
	);

	// Build the flight detail URL from active flight ID
	let flightUrl = $derived(aircraft.activeFlightId ? `/flights/${aircraft.activeFlightId}` : null);

	// Get country code for flag display
	const countryCode = $derived(() => {
		const code = aircraft.countryCode;
		return code && code.trim() !== '' ? code.toUpperCase() : null;
	});

	// Flag SVG URL from hampusborgos/country-flags repository
	const flagUrl = $derived(() => {
		const code = countryCode();
		return code
			? `https://cdn.jsdelivr.net/gh/hampusborgos/country-flags@main/svg/${code}.svg`
			: null;
	});
</script>

<div class="card preset-tonal-primary p-4">
	<a
		href={resolve(`/aircraft/${aircraft.id}`)}
		class="group block transition-all hover:scale-[1.02]"
	>
		<!-- Header Section -->
		<div class="mb-4 flex items-start justify-between">
			<div class="flex items-center gap-2">
				<Radio class="h-5 w-5 text-primary-500" />
				{#if flagUrl()}
					<img src={flagUrl()} alt="" class="inline-block h-4 rounded-sm" />
				{/if}
				<h3 class="text-lg font-semibold">{getAircraftTitle(aircraft)}</h3>
			</div>
		</div>

		<!-- Registration and Model -->
		<div class="mb-4 space-y-2">
			<div class="flex items-center gap-2">
				<Plane class="h-4 w-4 text-surface-500" />
				<div>
					<p class="text-surface-600-300-token text-xs">Registration</p>
					<p class="text-sm font-semibold">
						{aircraft.registration || 'Unknown'}
					</p>
				</div>
			</div>
			<div class="flex items-center gap-2">
				<Antenna class="h-4 w-4 text-surface-500" />
				<div>
					<p class="text-surface-600-300-token text-xs">Aircraft Model</p>
					<p class="text-sm">{aircraft.aircraftModel || 'Unknown'}</p>
				</div>
			</div>
			{#if aircraft.competitionNumber}
				<div class="flex items-center gap-2">
					<Activity class="h-4 w-4 text-surface-500" />
					<div>
						<p class="text-surface-600-300-token text-xs">Competition Number</p>
						<p class="font-mono text-sm">{aircraft.competitionNumber}</p>
					</div>
				</div>
			{/if}
		</div>

		<!-- Status Badges -->
		<div class="flex flex-wrap gap-2">
			<span
				class="badge text-xs {aircraft.tracked
					? 'preset-filled-success-500'
					: 'preset-filled-surface-500'}"
			>
				{#if aircraft.tracked}
					<Check class="mr-1 h-3 w-3" />
				{:else}
					<X class="mr-1 h-3 w-3" />
				{/if}
				{aircraft.tracked ? 'Tracked' : 'Not Tracked'}
			</span>
			<span
				class="badge text-xs {aircraft.identified
					? 'preset-filled-primary-500'
					: 'preset-filled-surface-500'}"
			>
				{#if aircraft.identified}
					<Check class="mr-1 h-3 w-3" />
				{:else}
					<X class="mr-1 h-3 w-3" />
				{/if}
				{aircraft.identified ? 'Identified' : 'Unidentified'}
			</span>
			{#if aircraft.fromOgnDdb}
				<span class="badge preset-filled-success-500 text-xs">
					<Check class="mr-1 h-3 w-3" />
					OGN DB
				</span>
			{/if}
			{#if aircraft.aircraftTypeOgn}
				<span class="badge {getAircraftTypeColor(aircraft.aircraftTypeOgn)} text-xs">
					{getAircraftTypeOgnDescription(aircraft.aircraftTypeOgn)}
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
