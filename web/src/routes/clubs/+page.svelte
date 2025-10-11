<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Users, Search, MapPinHouse, ExternalLink, Navigation, Plane, Map } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import type { ClubWithSoaring } from '$lib/types';

	let clubs: ClubWithSoaring[] = [];
	let loading = false;
	let error = '';
	let searchQuery = '';
	let filteredClubs: ClubWithSoaring[] = [];
	let searchInput = '';
	let showResults = false;
	let locationSearch = false;
	let latitude = '';
	let longitude = '';
	let radius = '50';

	onMount(async () => {
		const queryParams = page.url.searchParams;
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
			let endpoint = '/clubs';

			if (locationSearch && latitude && longitude && radius) {
				endpoint += `?latitude=${latitude}&longitude=${longitude}&radius=${radius}`;
			} else if (searchQuery) {
				endpoint += `?q=${encodeURIComponent(searchQuery)}`;
			}

			clubs = await serverCall<ClubWithSoaring[]>(endpoint);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search clubs: ${errorMessage}`;
			clubs = []; // Clear any previous results on error
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
					// Automatically trigger search after getting location
					searchClubs();
				},
				(error) => {
					console.error('Error getting location:', error);
				}
			);
		}
	}

	function formatAddress(club: ClubWithSoaring): string {
		if (!club.location) {
			return 'Address not available';
		}

		const parts = [];
		if (club.location.street1) parts.push(club.location.street1);
		if (club.location.street2) parts.push(club.location.street2);
		if (club.location.city) parts.push(club.location.city);
		if (club.location.state) parts.push(club.location.state);
		if (club.location.zip_code) parts.push(club.location.zip_code);
		return parts.join(', ') || 'Address not available';
	}

	function generateGoogleMapsUrl(club: ClubWithSoaring): string {
		if (club.location?.geolocation) {
			const { latitude, longitude } = club.location.geolocation;
			return `https://www.google.com/maps/search/?api=1&query=${latitude},${longitude}`;
		} else if (club.location) {
			// Fallback to address search if no coordinates
			const address = [
				club.location.street1,
				club.location.street2,
				club.location.city,
				club.location.state,
				club.location.zip_code
			]
				.filter(Boolean)
				.join(', ');
			return `https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(address)}`;
		}
		return '';
	}

	// Handle search input changes
	async function handleSearchInput(value: string) {
		searchInput = value;
		if (value.length > 0) {
			await searchClubsForFilter(value);
			showResults = true;
		} else {
			filteredClubs = [];
			showResults = false;
		}
	}

	// Search clubs for filtering
	async function searchClubsForFilter(query: string) {
		try {
			const endpoint = `/clubs?q=${encodeURIComponent(query)}`;
			filteredClubs = await serverCall<ClubWithSoaring[]>(endpoint);
		} catch (err) {
			console.error('Error searching clubs:', err);
			filteredClubs = [];
		}
	}

	// Navigate to selected club
	function selectClub(clubId: string) {
		goto(resolve(`/clubs/${clubId}`));
	}
</script>

<svelte:head>
	<title>Soaring Clubs - Glider Flights</title>
</svelte:head>

<div class="container mx-auto space-y-8 p-4">
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Users class="h-8 w-8" />
			Soaring Clubs
		</h1>
	</header>

	<!-- Search Section -->
	<section class="space-y-6 card p-6">
		<!-- Search Method Toggle -->
		<div class="flex justify-center gap-2">
			<button
				class="btn btn-sm {!locationSearch ? 'preset-filled-primary-500' : 'preset-tonal'}"
				onclick={() => (locationSearch = false)}
			>
				<Search class="mr-2 h-4 w-4" />
				Name Search
			</button>
			<button
				class="btn btn-sm {locationSearch ? 'preset-filled-primary-500' : 'preset-tonal'}"
				onclick={() => (locationSearch = true)}
			>
				<MapPinHouse class="mr-2 h-4 w-4" />
				Location Search
			</button>
		</div>

		<!-- Search Forms -->
		{#if !locationSearch}
			<div class="space-y-4">
				<div class="relative mx-auto max-w-2xl">
					<label class="label">
						<span>Search and Select Club</span>
						<input
							bind:value={searchInput}
							oninput={(e) => handleSearchInput((e.target as HTMLInputElement).value)}
							class="input"
							type="text"
							placeholder="Type to search clubs..."
							autocomplete="off"
						/>
					</label>

					<!-- Search Results -->
					{#if showResults && filteredClubs.length > 0}
						<div
							class="bg-surface-100-800-token border-surface-300-600-token absolute z-10 mt-1 max-h-60 w-full overflow-y-auto rounded-lg border shadow-lg"
						>
							{#each filteredClubs as club (club.id)}
								<button
									onclick={() => selectClub(club.id)}
									class="hover:bg-surface-200-700-token border-surface-200-700-token w-full border-b px-4 py-3 text-left transition-colors last:border-b-0"
								>
									<div class="font-medium text-primary-500">{club.name}</div>
									<div class="text-surface-600-300-token text-sm">{formatAddress(club)}</div>
								</button>
							{/each}
						</div>
					{:else if showResults && searchInput.length > 0}
						<div
							class="bg-surface-100-800-token border-surface-300-600-token absolute z-10 mt-1 w-full rounded-lg border p-4 shadow-lg"
						>
							<div class="text-surface-600-300-token text-center">
								No clubs found matching "{searchInput}"
							</div>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<div class="space-y-4">
				<div class="mx-auto grid max-w-2xl grid-cols-1 gap-4 md:grid-cols-3">
					<label class="label">
						<span>Latitude</span>
						<input
							bind:value={latitude}
							class="input"
							type="number"
							step="any"
							placeholder="e.g. 40.7128"
						/>
					</label>
					<label class="label">
						<span>Longitude</span>
						<input
							bind:value={longitude}
							class="input"
							type="number"
							step="any"
							placeholder="e.g. -74.0060"
						/>
					</label>
					<label class="label">
						<span>Radius (km)</span>
						<input
							bind:value={radius}
							class="input"
							type="number"
							min="1"
							max="1000"
							placeholder="50"
						/>
					</label>
				</div>
				<div class="flex justify-center">
					<button class="btn preset-filled-primary-500" onclick={getCurrentLocation}>
						<MapPinHouse class="mr-2 h-4 w-4" />
						Use My Location
					</button>
				</div>
			</div>
		{/if}
	</section>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Searching clubs...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert fill-error-500">
			<div class="alert-message">
				<h3 class="h3">Search Error</h3>
				<p>{error}</p>
			</div>
		</div>
	{/if}

	<!-- Results - Desktop Table -->
	{#if !loading && !error && clubs.length > 0}
		<section class="hidden card md:block">
			<header class="card-header">
				<h2 class="h2">Search Results</h2>
				<p class="text-surface-500-400-token">
					{clubs.length} club{clubs.length === 1 ? '' : 's'} found
				</p>
			</header>

			<div class="table-container">
				<table class="table-hover table">
					<thead>
						<tr>
							<th>Club Name</th>
							<th>Address</th>
							<th>Airport</th>
							<th>Coordinates</th>
							<th>Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each clubs as club (club.id)}
							<tr>
								<td>
									<a
										href={resolve(`/clubs/${club.id}`)}
										class="anchor font-medium text-primary-500 hover:text-primary-600"
									>
										{club.name}
									</a>
								</td>
								<td>
									<div class="flex items-start gap-2">
										<MapPinHouse class="mt-0.5 h-4 w-4 flex-shrink-0 text-surface-500" />
										<span class="text-sm">{formatAddress(club)}</span>
									</div>
								</td>
								<td>
									{#if club.home_base_airport_id}
										<div class="flex items-center gap-2">
											<Plane class="h-4 w-4 text-surface-500" />
											<span class="font-mono text-sm">{club.home_base_airport_id}</span>
										</div>
									{:else}
										<span class="text-surface-500">—</span>
									{/if}
								</td>
								<td>
									{#if club.location?.geolocation}
										<div class="flex items-center gap-2">
											<Navigation class="h-4 w-4 text-surface-500" />
											<span class="font-mono text-xs">
												{club.location.geolocation.latitude.toFixed(4)}, {club.location.geolocation.longitude.toFixed(
													4
												)}
											</span>
										</div>
									{:else}
										<span class="text-surface-500">—</span>
									{/if}
								</td>
								<td>
									<div class="flex gap-1">
										<a
											href={resolve(`/clubs/${club.id}`)}
											class="preset-tonal-surface-500 btn flex items-center gap-1 btn-sm"
										>
											<ExternalLink class="h-3 w-3" />
											View
										</a>
										{#if generateGoogleMapsUrl(club)}
											<a
												href={generateGoogleMapsUrl(club)}
												target="_blank"
												rel="noopener noreferrer"
												class="preset-tonal-primary-500 btn flex items-center gap-1 btn-sm"
											>
												<Map class="h-3 w-3" />
												Map
											</a>
										{/if}
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>

		<!-- Results - Mobile Cards -->
		<div class="block space-y-4 md:hidden">
			<div class="card-header">
				<h2 class="h2">Search Results</h2>
				<p class="text-surface-500-400-token">
					{clubs.length} club{clubs.length === 1 ? '' : 's'} found
				</p>
			</div>

			{#each clubs as club (club.id)}
				<article class="relative card p-4 transition-all duration-200 hover:shadow-lg">
					<!-- Club header -->
					<div
						class="border-surface-200-700-token mb-3 flex items-start justify-between border-b pb-3"
					>
						<a
							href={resolve(`/clubs/${club.id}`)}
							class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
						>
							{club.name}
						</a>
						<a
							href={resolve(`/clubs/${club.id}`)}
							class="relative z-10 flex-shrink-0"
							title="View club details"
						>
							<ExternalLink class="h-4 w-4 text-surface-400 hover:text-primary-500" />
						</a>
					</div>

					<!-- Club details -->
					<div class="space-y-2 text-sm">
						<div class="flex items-start gap-2">
							<MapPinHouse class="mt-0.5 h-4 w-4 flex-shrink-0 text-surface-500" />
							<span class="text-surface-600-300-token flex-1">{formatAddress(club)}</span>
						</div>

						{#if club.location?.geolocation}
							<div class="flex items-center gap-2">
								<Navigation class="h-4 w-4 flex-shrink-0 text-surface-500" />
								<span class="text-surface-600-300-token font-mono text-xs">
									{club.location.geolocation.latitude.toFixed(4)}, {club.location.geolocation.longitude.toFixed(
										4
									)}
								</span>
							</div>
						{/if}

						{#if club.home_base_airport_id}
							<div class="flex items-center gap-2">
								<Plane class="h-4 w-4 flex-shrink-0 text-surface-500" />
								<span class="text-surface-600-300-token">
									Airport: <span class="font-mono">{club.home_base_airport_id}</span>
								</span>
							</div>
						{/if}
					</div>

					<!-- Actions -->
					{#if generateGoogleMapsUrl(club)}
						<div class="mt-3 flex gap-2">
							<a
								href={generateGoogleMapsUrl(club)}
								target="_blank"
								rel="noopener noreferrer"
								class="preset-tonal-primary-500 btn flex-1 btn-sm"
							>
								<Map class="mr-1 h-3 w-3" />
								Maps
							</a>
							{#if club.location?.geolocation}
								<a
									href={`https://www.google.com/maps/dir/?api=1&destination=${club.location.geolocation.latitude},${club.location.geolocation.longitude}`}
									target="_blank"
									rel="noopener noreferrer"
									class="preset-tonal-secondary-500 btn flex-1 btn-sm"
								>
									<Navigation class="mr-1 h-3 w-3" />
									Directions
								</a>
							{/if}
						</div>
					{/if}
				</article>
			{/each}
		</div>
	{:else if !loading && !error && clubs.length === 0 && (searchQuery || (latitude && longitude))}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No clubs found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or expanding your search radius.
				</p>
			</div>
		</div>
	{/if}
</div>
