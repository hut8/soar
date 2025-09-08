<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { ProgressRadial } from '@skeletonlabs/skeleton';

	interface Point {
		latitude: number;
		longitude: number;
	}

	interface Club {
		id: string;
		name: string;
		is_soaring?: boolean;
		home_base_airport_id?: number;
		location_id?: string;
		street1?: string;
		street2?: string;
		city?: string;
		state?: string;
		zip_code?: string;
		region_code?: string;
		county_mail_code?: string;
		country_mail_code?: string;
		base_location?: Point;
		created_at: string;
		updated_at: string;
	}

	let clubs: Club[] = [];
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
			await searchClubs();
		} else if (lat && lng && r) {
			latitude = lat;
			longitude = lng;
			radius = r;
			locationSearch = true;
			await searchClubs();
		}
	});

	async function searchClubs() {
		loading = true;
		error = '';

		try {
			let url = 'http://localhost:1337/clubs?';

			if (locationSearch && latitude && longitude && radius) {
				url += `latitude=${latitude}&longitude=${longitude}&radius=${radius}`;
			} else if (searchQuery) {
				url += `q=${encodeURIComponent(searchQuery)}`;
			} else {
				url = 'http://localhost:1337/clubs';
			}

			const response = await fetch(url);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			clubs = await response.json();
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search clubs: ${errorMessage}`;
			console.error('Error searching clubs:', err);
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

	function formatAddress(club: Club): string {
		const parts = [];
		if (club.street1) parts.push(club.street1);
		if (club.street2) parts.push(club.street2);
		if (club.city) parts.push(club.city);
		if (club.state) parts.push(club.state);
		if (club.zip_code) parts.push(club.zip_code);
		return parts.join(', ') || 'Address not available';
	}
</script>

<svelte:head>
	<title>Soaring Clubs - Glider Flights</title>
</svelte:head>

<div class="space-y-6">
	<header class="space-y-4 text-center">
		<h1 class="h1">Soaring Clubs</h1>
		<p class="text-surface-600-300-token">Find active soaring clubs in your area</p>
	</header>

	<!-- Search Section -->
	<section class="card space-y-4 p-6">
		<h2 class="h2">Search Clubs</h2>

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
					placeholder="Search clubs by name..."
					on:keydown={(e) => e.key === 'Enter' && searchClubs()}
				/>
				<div class="flex justify-center">
					<button class="variant-filled-primary btn" on:click={searchClubs}> Search Clubs </button>
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
					<button class="variant-filled-primary btn" on:click={searchClubs}>
						Search Nearby Clubs
					</button>
				</div>
			</div>
		{/if}
	</section>

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-8">
			<ProgressRadial width="w-8" />
			<span class="ml-2">Searching clubs...</span>
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
	{#if !loading && !error && clubs.length > 0}
		<section class="space-y-4">
			<h2 class="h2">Results ({clubs.length} clubs found)</h2>

			<div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
				{#each clubs as club (club.id)}
					<div class="card space-y-4 p-6">
						<header class="card-header">
							<h3 class="h3">{club.name}</h3>
						</header>

						<div class="space-y-2">
							<div class="flex items-start space-x-2">
								<span class="text-surface-500">📍</span>
								<span class="text-sm">{formatAddress(club)}</span>
							</div>

							{#if club.base_location}
								<div class="flex items-center space-x-2">
									<span class="text-surface-500">🗺️</span>
									<span class="text-sm">
										{club.base_location.latitude.toFixed(4)}, {club.base_location.longitude.toFixed(
											4
										)}
									</span>
								</div>
							{/if}

							{#if club.home_base_airport_id}
								<div class="flex items-center space-x-2">
									<span class="text-surface-500">✈️</span>
									<span class="text-sm">Airport: {club.home_base_airport_id}</span>
								</div>
							{/if}
						</div>

						<footer class="card-footer">
							<span class="variant-filled-success badge">Active Soaring Club</span>
						</footer>
					</div>
				{/each}
			</div>
		</section>
	{:else if !loading && !error && clubs.length === 0 && (searchQuery || (latitude && longitude))}
		<div class="py-8 text-center">
			<h3 class="h3">No clubs found</h3>
			<p class="text-surface-600-300-token">Try adjusting your search criteria or search radius.</p>
		</div>
	{/if}
</div>
