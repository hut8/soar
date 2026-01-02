<script lang="ts">
	import { Radio, Plane, Antenna, Check, Activity, Map, Navigation } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import {
		getAircraftTypeOgnDescription,
		getAircraftTypeColor,
		getAircraftTitle,
		formatAircraftAddress
	} from '$lib/formatters';
	import type { Aircraft } from '$lib/types';

	let { aircraft }: { aircraft: Aircraft } = $props();

	// Build the operations map URL with location parameters from latest fix
	let mapUrl = $derived(
		aircraft.latitude && aircraft.longitude
			? `/operations?lat=${aircraft.latitude}&lng=${aircraft.longitude}&zoom=13`
			: null
	);

	// Build the flight detail URL from current fix flight ID
	// currentFix is JsonValue, so we need to check if it's an object with flightId
	let flightUrl = $derived.by(() => {
		if (
			!aircraft.currentFix ||
			typeof aircraft.currentFix !== 'object' ||
			Array.isArray(aircraft.currentFix)
		) {
			return null;
		}
		const fixObj = aircraft.currentFix as Record<string, unknown>;
		return fixObj.flightId ? `/flights/${fixObj.flightId}` : null;
	});

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
				{#if flagUrl()}
					<img src={flagUrl()} alt="" class="inline-block h-5 rounded-sm" />
				{:else}
					<Radio class="h-5 w-5 text-primary-500" />
				{/if}
				<h3 class="text-lg font-semibold">{getAircraftTitle(aircraft)}</h3>
			</div>
		</div>

		<!-- Aircraft Details (matching detail page header) -->
		<div class="mb-4 space-y-2">
			{#if aircraft.icaoModelCode}
				<div class="flex items-center gap-2">
					<Plane class="h-4 w-4 text-surface-500" />
					<div>
						<p class="text-surface-600-300-token text-xs">ICAO Model Code</p>
						<p class="font-mono text-sm">{aircraft.icaoModelCode}</p>
					</div>
				</div>
			{/if}
			<div class="flex items-center gap-2">
				<Antenna class="h-4 w-4 text-surface-500" />
				<div>
					<p class="text-surface-600-300-token text-xs">Address</p>
					<p class="font-mono text-sm">
						{formatAircraftAddress(aircraft.addressType, aircraft.address)}
					</p>
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
