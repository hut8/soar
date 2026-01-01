<script lang="ts">
	import { Search, Plane, MapPin } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { serverCall } from '$lib/api/server';
	import { formatSnakeCase } from '$lib/formatters';
	import type { Airport, DataListResponse } from '$lib/types';

	let airports: Airport[] = [];
	let loading = false;
	let error = '';
	let searchQuery = '';
	let searchTimeout: ReturnType<typeof setTimeout> | null = null;

	async function searchAirports() {
		if (!searchQuery.trim()) {
			airports = [];
			return;
		}

		loading = true;
		error = '';

		try {
			const endpoint = `/airports?q=${encodeURIComponent(searchQuery.trim())}`;
			const response = await serverCall<DataListResponse<Airport>>(endpoint);
			airports = response.data;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search airports: ${errorMessage}`;
			console.error('Error searching airports:', err);
			airports = [];
		} finally {
			loading = false;
		}
	}

	function handleInput() {
		// Clear existing timeout
		if (searchTimeout) {
			clearTimeout(searchTimeout);
		}

		// Set new timeout for search (300ms delay)
		searchTimeout = setTimeout(() => {
			searchAirports();
		}, 300);
	}

	function formatCoordinates(lat: string | null, lng: string | null): string {
		if (!lat || !lng) return '—';
		const latNum = parseFloat(lat);
		const lngNum = parseFloat(lng);
		const latDir = latNum >= 0 ? 'N' : 'S';
		const lngDir = lngNum >= 0 ? 'E' : 'W';
		return `${Math.abs(latNum).toFixed(4)}°${latDir}, ${Math.abs(lngNum).toFixed(4)}°${lngDir}`;
	}

	function getGoogleMapsUrl(lat: string | null, lng: string | null): string {
		if (!lat || !lng) return '#';
		return `https://www.google.com/maps?q=${lat},${lng}`;
	}

	function formatLocation(airport: Airport): string {
		const parts: string[] = [];

		if (airport.municipality) {
			parts.push(airport.municipality);
		}

		if (airport.isoRegion) {
			// Extract state/province from iso_region (format: US-CA -> CA)
			const region = airport.isoRegion.split('-').pop() || airport.isoRegion;
			parts.push(region);
		}

		if (airport.isoCountry) {
			parts.push(airport.isoCountry);
		}

		return parts.length > 0 ? parts.join(', ') : '—';
	}

	function getAirportCode(airport: Airport): string {
		return airport.icaoCode || airport.iataCode || airport.gpsCode || airport.localCode || '—';
	}
</script>

<svelte:head>
	<title>Airports - Airport Search</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Plane class="h-8 w-8" />
			Airport Search
		</h1>
	</header>

	<!-- Search Section -->
	<section class="space-y-4 card p-6">
		<h3 class="mb-3 flex items-center gap-2 text-lg font-semibold">
			<Search class="h-5 w-5" />
			Search Airports
		</h3>
		<div class="space-y-3 rounded-lg border p-3">
			<div class="relative">
				<input
					class="input"
					placeholder="Search by name, city, or code (e.g., KJFK, New York)"
					bind:value={searchQuery}
					oninput={handleInput}
				/>
				{#if loading}
					<div
						class="absolute top-1/2 right-3 h-4 w-4 -translate-y-1/2 animate-spin rounded-full border-2 border-primary-500 border-t-transparent"
					></div>
				{/if}
			</div>

			<!-- Error message display -->
			{#if error}
				<div class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600">
					{error}
				</div>
			{/if}
		</div>
	</section>

	<!-- Results Table -->
	{#if !loading && airports.length > 0}
		<section class="card">
			<header class="card-header">
				<h2 class="h2">Search Results</h2>
				<p class="text-surface-500-400-token">
					{airports.length} airport{airports.length === 1 ? '' : 's'} found
				</p>
			</header>

			<!-- Desktop: Table -->
			<div class="hidden md:block">
				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Code</th>
								<th>Name</th>
								<th>Type</th>
								<th>Location</th>
								<th>Coordinates</th>
								<th>Elevation</th>
							</tr>
						</thead>
						<tbody>
							{#each airports as airport (airport.id)}
								<tr>
									<td>
										<a
											href={resolve(`/airports/${airport.id}`)}
											class="anchor font-mono text-primary-500 hover:text-primary-600"
										>
											{getAirportCode(airport)}
										</a>
									</td>
									<td class="font-semibold">{airport.name}</td>
									<td>
										<span class="badge preset-tonal">
											{formatSnakeCase(airport.airportType)}
										</span>
									</td>
									<td>
										<div class="flex items-center gap-1">
											<MapPin class="h-4 w-4 text-surface-500" />
											<span>{formatLocation(airport)}</span>
										</div>
									</td>
									<td class="font-mono text-sm">
										<a
											href={getGoogleMapsUrl(airport.latitudeDeg, airport.longitudeDeg)}
											target="_blank"
											rel="noopener noreferrer"
											class="text-primary-500 underline hover:text-primary-700"
										>
											{formatCoordinates(airport.latitudeDeg, airport.longitudeDeg)}
										</a>
									</td>
									<td>
										{#if airport.elevationFt !== null}
											<span>{airport.elevationFt} ft</span>
										{:else}
											<span class="text-surface-500">—</span>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>

			<!-- Mobile: Cards -->
			<div class="space-y-4 p-4 md:hidden">
				{#each airports as airport (airport.id)}
					<a
						href={resolve(`/airports/${airport.id}`)}
						class="block card p-4 hover:ring-2 hover:ring-primary-500"
					>
						<div class="mb-2 flex items-start justify-between gap-2">
							<div class="flex-1">
								<div class="font-mono text-sm text-primary-500">{getAirportCode(airport)}</div>
								<div class="font-semibold">{airport.name}</div>
							</div>
							<span class="badge preset-tonal text-xs">
								{formatSnakeCase(airport.airportType)}
							</span>
						</div>

						<dl class="space-y-2 text-sm">
							<div class="flex items-start gap-2">
								<MapPin class="mt-0.5 h-4 w-4 flex-shrink-0 text-surface-500" />
								<span class="text-surface-600-300-token">{formatLocation(airport)}</span>
							</div>
							<div class="text-surface-600-300-token font-mono text-xs">
								{formatCoordinates(airport.latitudeDeg, airport.longitudeDeg)}
							</div>
							{#if airport.elevationFt !== null}
								<div class="text-surface-600-300-token text-xs">
									Elevation: {airport.elevationFt} ft
								</div>
							{/if}
						</dl>
					</a>
				{/each}
			</div>
		</section>
	{:else if !loading && airports.length === 0 && searchQuery}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No airports found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or search for a different airport.
				</p>
			</div>
		</div>
	{/if}
</div>
