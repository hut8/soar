<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Users, Search, MapPinHouse, ExternalLink, Navigation, Plane } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { ClubSelector } from '$lib';
	import { serverCall } from '$lib/api/server';

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
	let selectedClub: string[] = [];
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

			clubs = await serverCall<Club[]>(endpoint);
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
					// Automatically trigger search after getting location
					searchClubs();
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

	// Handle club selection from ClubSelector
	function handleClubSelection(e: { value: string[] }) {
		selectedClub = e.value;
		if (selectedClub.length > 0) {
			// Find the selected club and display it
			displaySelectedClub(selectedClub[0]);
		}
	}

	// Handle search input changes from ClubSelector
	function handleSearchInput(e: { inputValue: string }) {
		searchQuery = e.inputValue;
	}

	// Display a specific club when selected
	async function displaySelectedClub(clubId: string) {
		loading = true;
		error = '';

		try {
			const selectedClubData = await serverCall<Club>(`/clubs/${clubId}`);
			if (selectedClubData) {
				clubs = [selectedClubData];
			} else {
				clubs = [];
			}
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load selected club: ${errorMessage}`;
			console.error('Error loading selected club:', err);
		} finally {
			loading = false;
		}
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
				class="btn btn-sm {!locationSearch ? 'preset-filled' : 'preset-soft'}"
				on:click={() => (locationSearch = false)}
			>
				<Search class="mr-2 h-4 w-4" />
				Name Search
			</button>
			<button
				class="btn btn-sm {locationSearch ? 'preset-filled' : 'preset-soft'}"
				on:click={() => (locationSearch = true)}
			>
				<MapPinHouse class="mr-2 h-4 w-4" />
				Location Search
			</button>
		</div>

		<!-- Search Forms -->
		{#if !locationSearch}
			<div class="space-y-4">
				<div class="mx-auto max-w-2xl">
					<ClubSelector
						value={selectedClub}
						onValueChange={handleClubSelection}
						onInputValueChange={handleSearchInput}
						label="Search and Select Club"
						placeholder="Type to search clubs or select from dropdown..."
					/>
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
					<button class="btn preset-filled-primary-500" on:click={getCurrentLocation}>
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

	<!-- Results -->
	{#if !loading && !error && clubs.length > 0}
		<section class="space-y-6">
			<header class="text-center">
				<h2 class="h2">Search Results</h2>
				<p class="text-surface-500-400-token">
					{clubs.length} club{clubs.length === 1 ? '' : 's'} found
				</p>
			</header>

			<div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
				{#each clubs as club (club.id)}
					<article class="space-y-4 card p-6 transition-transform duration-200 hover:scale-[1.02]">
						<header>
							<h3 class="h3 text-primary-500">{club.name}</h3>
						</header>

						<div class="space-y-3 text-sm">
							<div class="flex items-start gap-3">
								<MapPinHouse class="mt-0.5 h-4 w-4 text-surface-500" />
								<span class="flex-1">{formatAddress(club)}</span>
							</div>

							{#if club.base_location}
								<div class="flex items-center gap-3">
									<Navigation class="h-4 w-4 text-surface-500" />
									<span class="font-mono text-xs">
										{club.base_location.latitude.toFixed(4)}, {club.base_location.longitude.toFixed(
											4
										)}
									</span>
								</div>
							{/if}

							{#if club.home_base_airport_id}
								<div class="flex items-center gap-3">
									<Plane class="h-4 w-4 text-surface-500" />
									<span>Airport ID: {club.home_base_airport_id}</span>
								</div>
							{/if}
						</div>

						<footer class="border-surface-200-700-token border-t pt-4">
							<a href={resolve(`/clubs/${club.id}`)} class="variant-soft btn w-full btn-sm">
								<ExternalLink class="mr-2 h-4 w-4" />
								View Details
							</a>
						</footer>
					</article>
				{/each}
			</div>
		</section>
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
