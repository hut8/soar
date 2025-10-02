<script lang="ts">
	import { Search, Plane, MapPin } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { serverCall } from '$lib/api/server';

	interface Airport {
		id: number;
		ident: string;
		airport_type: string;
		name: string;
		latitude_deg: string | null;
		longitude_deg: string | null;
		elevation_ft: number | null;
		continent: string | null;
		iso_country: string | null;
		iso_region: string | null;
		municipality: string | null;
		scheduled_service: boolean;
		icao_code: string | null;
		iata_code: string | null;
		gps_code: string | null;
		local_code: string | null;
	}

	let airports: Airport[] = [];
	let loading = false;
	let error = '';
	let searchQuery = '';

	async function searchAirports() {
		if (!searchQuery.trim()) {
			error = 'Please enter a search query';
			return;
		}

		loading = true;
		error = '';

		try {
			const endpoint = `/airports?q=${encodeURIComponent(searchQuery.trim())}`;
			airports = await serverCall<Airport[]>(endpoint);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search airports: ${errorMessage}`;
			console.error('Error searching airports:', err);
			airports = [];
		} finally {
			loading = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			searchAirports();
		}
	}

	function formatCoordinates(lat: string | null, lng: string | null): string {
		if (!lat || !lng) return '—';
		return `${parseFloat(lat).toFixed(4)}, ${parseFloat(lng).toFixed(4)}`;
	}

	function getAirportCode(airport: Airport): string {
		return airport.icao_code || airport.iata_code || airport.gps_code || airport.local_code || '—';
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
			<input
				class="input"
				placeholder="Search by name, city, or code (e.g., KJFK, New York)"
				bind:value={searchQuery}
				onkeydown={handleKeydown}
				oninput={() => (error = '')}
			/>

			<button
				class="btn w-full preset-filled-primary-500"
				onclick={searchAirports}
				disabled={loading}
			>
				{#if loading}
					<div
						class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
					></div>
					Searching...
				{:else}
					<Search class="mr-2 h-4 w-4" />
					Search Airports
				{/if}
			</button>

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
									<span class="variant-soft badge">
										{airport.airport_type}
									</span>
								</td>
								<td>
									<div class="flex items-center gap-1">
										<MapPin class="h-4 w-4 text-surface-500" />
										<span>
											{airport.municipality || '—'}
											{#if airport.iso_region}
												<span class="text-surface-500">, {airport.iso_region}</span>
											{/if}
										</span>
									</div>
								</td>
								<td class="font-mono text-sm">
									{formatCoordinates(airport.latitude_deg, airport.longitude_deg)}
								</td>
								<td>
									{#if airport.elevation_ft !== null}
										<span>{airport.elevation_ft} ft</span>
									{:else}
										<span class="text-surface-500">—</span>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
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
