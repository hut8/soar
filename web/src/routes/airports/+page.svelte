<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { ProgressRadial } from '@skeletonlabs/skeleton';

	interface Location {
		latitude: number;
		longitude: number;
	}

	interface Airport {
		id: number;
		ident: string;
		airport_type?: string;
		name?: string;
		latitude_deg?: number;
		longitude_deg?: number;
		elevation_ft?: number;
		continent?: string;
		iso_country?: string;
		iso_region?: string;
		municipality?: string;
		scheduled_service: boolean;
		icao_code?: string;
		iata_code?: string;
		gps_code?: string;
		local_code?: string;
		home_link?: string;
		wikipedia_link?: string;
		keywords?: string;
		location?: Location;
	}

	let airports: Airport[] = [];
	let loading = false;
	let error = '';
	let searchQuery = '';
	let locationSearch = false;
	let latitude = '';
	let longitude = '';
	let radius = '50';

	$: queryParams = $page.url.searchParams;

	onMount(async () => {
		const q = queryParams.get('q');
		const lat = queryParams.get('latitude');
		const lng = queryParams.get('longitude');
		const r = queryParams.get('radius');

		if (q) {
			searchQuery = q;
			locationSearch = false;
			await searchAirports();
		} else if (lat && lng && r) {
			latitude = lat;
			longitude = lng;
			radius = r;
			locationSearch = true;
			await searchAirports();
		}
	});

	async function searchAirports() {
		loading = true;
		error = '';

		try {
			let url = 'http://localhost:1337/airports?';

			if (locationSearch && latitude && longitude && radius) {
				url += `latitude=${latitude}&longitude=${longitude}&radius=${radius}`;
			} else if (searchQuery) {
				url += `q=${encodeURIComponent(searchQuery)}`;
			} else {
				url = 'http://localhost:1337/airports';
			}

			const response = await fetch(url);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			airports = await response.json();
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search airports: ${errorMessage}`;
			console.error('Error searching airports:', err);
		} finally {
			loading = false;
		}
	}

	function getCurrentLocation() {
		if (navigator.geolocation) {
			navigator.geolocation.getCurrentPosition(
				(position) => {
					latitude = position.coords.latitude.toString();
					longitude = position.coords.longitude.toString();
				},
				(error) => {
					console.error('Error getting location:', error);
				}
			);
		}
	}

	function formatAirportType(type: string | undefined): string {
		switch (type) {
			case 'large_airport':
				return 'Large Airport';
			case 'medium_airport':
				return 'Medium Airport';
			case 'small_airport':
				return 'Small Airport';
			case 'heliport':
				return 'Heliport';
			case 'seaplane_base':
				return 'Seaplane Base';
			case 'balloonport':
				return 'Balloonport';
			case 'closed':
				return 'Closed';
			default:
				return type || 'Unknown';
		}
	}

	function formatLocation(airport: Airport): string {
		const parts = [];
		if (airport.municipality) parts.push(airport.municipality);
		if (airport.iso_region) parts.push(airport.iso_region);
		if (airport.iso_country) parts.push(airport.iso_country);
		return parts.join(', ') || 'Location not available';
	}
</script>

<svelte:head>
	<title>Airports - Glider Flights</title>
</svelte:head>

<div class="space-y-6">
	<header class="space-y-4 text-center">
		<h1 class="h1">Airports</h1>
		<p class="text-surface-600-300-token">
			Find airports and airfields suitable for gliding operations
		</p>
	</header>

	<!-- Search Section -->
	<section class="card space-y-4 p-6">
		<h2 class="h2">Search Airports</h2>

		<!-- Search Method Toggle -->
		<div class="flex justify-center space-x-2">
			<button
				class="btn btn-sm {!locationSearch ? 'variant-filled-secondary' : 'variant-ghost-surface'}"
				on:click={() => (locationSearch = false)}
			>
				🔍 Name Search
			</button>
			<button
				class="btn btn-sm {locationSearch ? 'variant-filled-secondary' : 'variant-ghost-surface'}"
				on:click={() => (locationSearch = true)}
			>
				📍 Location Search
			</button>
		</div>

		<!-- Search Forms -->
		{#if !locationSearch}
			<div class="space-y-4">
				<input
					bind:value={searchQuery}
					class="input"
					type="text"
					placeholder="Search airports by name or identifier..."
					on:keydown={(e) => e.key === 'Enter' && searchAirports()}
				/>
				<div class="flex justify-center">
					<button class="variant-filled-primary btn" on:click={searchAirports}>
						Search Airports
					</button>
				</div>
			</div>
		{:else}
			<div class="space-y-4">
				<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
					<input
						bind:value={latitude}
						class="input"
						type="number"
						step="any"
						placeholder="Latitude"
					/>
					<input
						bind:value={longitude}
						class="input"
						type="number"
						step="any"
						placeholder="Longitude"
					/>
					<input
						bind:value={radius}
						class="input"
						type="number"
						min="1"
						max="1000"
						placeholder="Radius (km)"
					/>
				</div>
				<div class="flex justify-center space-x-2">
					<button class="variant-ghost-surface btn" on:click={getCurrentLocation}>
						📱 Use My Location
					</button>
					<button class="variant-filled-primary btn" on:click={searchAirports}>
						Search Nearby Airports
					</button>
				</div>
			</div>
		{/if}
	</section>

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-8">
			<ProgressRadial width="w-8" />
			<span class="ml-2">Searching airports...</span>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert variant-filled-error">
			<div class="alert-message">
				<h3 class="h3">Error</h3>
				<p>{error}</p>
			</div>
		</div>
	{/if}

	<!-- Results -->
	{#if !loading && !error && airports.length > 0}
		<section class="space-y-4">
			<h2 class="h2">Results ({airports.length} airports found)</h2>

			<div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
				{#each airports as airport (airport.id)}
					<div class="card space-y-4 p-6">
						<header class="card-header">
							<h3 class="h3">{airport.name}</h3>
							{#if airport.ident}
								<p class="font-mono text-sm text-primary-500">{airport.ident}</p>
							{/if}
						</header>

						<div class="space-y-2">
							<div class="flex items-start space-x-2">
								<span class="text-surface-500">📍</span>
								<span class="text-sm">{formatLocation(airport)}</span>
							</div>

							{#if airport.location}
								<div class="flex items-center space-x-2">
									<span class="text-surface-500">🗺️</span>
									<span class="text-sm">
										{airport.location.latitude.toFixed(4)}, {airport.location.longitude.toFixed(4)}
									</span>
								</div>
							{/if}

							{#if airport.elevation_ft}
								<div class="flex items-center space-x-2">
									<span class="text-surface-500">⛰️</span>
									<span class="text-sm">Elevation: {airport.elevation_ft} ft</span>
								</div>
							{/if}

							{#if airport.airport_type}
								<div class="flex items-center space-x-2">
									<span class="text-surface-500">🏷️</span>
									<span class="text-sm">{formatAirportType(airport.airport_type)}</span>
								</div>
							{/if}
						</div>

						<footer class="card-footer">
							<div class="flex flex-wrap gap-2">
								{#if airport.airport_type === 'small_airport'}
									<span class="variant-filled-secondary badge">Small Airport</span>
								{:else if airport.airport_type === 'medium_airport'}
									<span class="variant-filled-primary badge">Medium Airport</span>
								{:else if airport.airport_type === 'large_airport'}
									<span class="variant-filled-success badge">Large Airport</span>
								{:else if airport.airport_type === 'heliport'}
									<span class="variant-filled-warning badge">Heliport</span>
								{:else if airport.airport_type === 'seaplane_base'}
									<span class="variant-filled-tertiary badge">Seaplane Base</span>
								{:else if airport.airport_type === 'closed'}
									<span class="variant-filled-error badge">Closed</span>
								{:else}
									<span class="variant-soft badge">Airport</span>
								{/if}
							</div>
						</footer>
					</div>
				{/each}
			</div>
		</section>
	{:else if !loading && !error && airports.length === 0 && (searchQuery || (latitude && longitude))}
		<div class="py-8 text-center">
			<h3 class="h3">No airports found</h3>
			<p class="text-surface-600-300-token">Try adjusting your search criteria or search radius.</p>
		</div>
	{/if}
</div>
