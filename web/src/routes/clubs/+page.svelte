<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { Users, Search, MapPinHouse } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { ClubSelector } from '$lib';

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

	$: queryParams = page.url.searchParams;

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
			let url = '/clubs?';

			if (locationSearch && latitude && longitude && radius) {
				url += `latitude=${latitude}&longitude=${longitude}&radius=${radius}`;
			} else if (searchQuery) {
				url += `q=${encodeURIComponent(searchQuery)}`;
			} else {
				url = '/clubs';
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
			const response = await fetch(`/clubs/${clubId}`);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const allClubs = await response.json();
			// Find the specific club by ID
			const selectedClubData = allClubs.find((club: Club) => club.id === clubId);

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

<div class="container mx-auto p-4 space-y-8">
	<header class="text-center space-y-2">
		<h1 class="h1 flex items-center justify-center gap-2">
			<Users class="w-8 h-8" />
			Soaring Clubs
		</h1>
	</header>

	<!-- Search Section -->
	<section class="card p-6 space-y-6">
		<!-- Search Method Toggle -->
		<div class="flex justify-center gap-2">
			<button
				class="btn btn-sm {!locationSearch ? 'preset-filled' : 'preset-soft'}"
				on:click={() => (locationSearch = false)}
			>
				<Search class="w-4 h-4 mr-2" />
				Name Search
			</button>
			<button
				class="btn btn-sm {locationSearch ? 'preset-filled' : 'preset-soft'}"
				on:click={() => (locationSearch = true)}
			>
				<MapPinHouse class="w-4 h-4 mr-2" />
				Location Search
			</button>
		</div>

		<!-- Search Forms -->
		{#if !locationSearch}
			<div class="space-y-4">
				<div class="max-w-2xl mx-auto">
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
				<div class="grid grid-cols-1 md:grid-cols-3 gap-4 max-w-2xl mx-auto">
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
				<div class="flex justify-center gap-4">
					<button class="btn variant-soft" on:click={getCurrentLocation}>
						<MapPinHouse class="w-4 h-4 mr-2" />
						Use My Location
					</button>
					<button class="btn variant-filled" on:click={searchClubs}>
						<Search class="w-4 h-4 mr-2" />
						Search Nearby
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
		<div class="alert variant-filled-error">
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
				<p class="text-surface-500-400-token">{clubs.length} club{clubs.length === 1 ? '' : 's'} found</p>
			</header>

			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
				{#each clubs as club (club.id)}
					<article class="card p-6 space-y-4 hover:scale-[1.02] transition-transform duration-200">
						<header>
							<h3 class="h3 text-primary-500">{club.name}</h3>
						</header>

						<div class="space-y-3 text-sm">
							<div class="flex items-start gap-3">
								<MapPinHouse class="w-4 h-4 mt-0.5 text-surface-500" />
								<span class="flex-1">{formatAddress(club)}</span>
							</div>

							{#if club.base_location}
								<div class="flex items-center gap-3">
									<div class="w-4 h-4 text-surface-500">🗺️</div>
									<span class="font-mono text-xs">
										{club.base_location.latitude.toFixed(4)}, {club.base_location.longitude.toFixed(4)}
									</span>
								</div>
							{/if}

							{#if club.home_base_airport_id}
								<div class="flex items-center gap-3">
									<div class="w-4 h-4 text-surface-500">✈️</div>
									<span>Airport ID: {club.home_base_airport_id}</span>
								</div>
							{/if}
						</div>

						<footer class="pt-2">
							<span class="badge variant-filled-success">Active Club</span>
						</footer>
					</article>
				{/each}
			</div>
		</section>
	{:else if !loading && !error && clubs.length === 0 && (searchQuery || (latitude && longitude))}
		<div class="card p-12 text-center space-y-4">
			<div class="text-6xl opacity-50">🔍</div>
			<div class="space-y-2">
				<h3 class="h3">No clubs found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or expanding your search radius.
				</p>
			</div>
		</div>
	{/if}
</div>
