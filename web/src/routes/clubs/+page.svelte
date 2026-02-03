<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Users, Search, MapPinHouse, ExternalLink, Plane } from '@lucide/svelte';
	import { Progress, SegmentedControl } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import { getLogger } from '$lib/logging';
	import type { ClubWithSoaring, DataListResponse } from '$lib/types';

	const logger = getLogger(['soar', 'ClubsPage']);

	interface PlaceLocation {
		lat(): number;
		lng(): number;
	}

	interface PlaceResult {
		location?: PlaceLocation;
	}

	interface PlaceAutocompleteElement extends HTMLElement {
		value?: PlaceResult;
	}

	let clubs = $state<ClubWithSoaring[]>([]);
	let loading = $state(false);
	let error = $state('');
	let searchQuery = $state('');
	let searchType = $state<'name' | 'location'>('name');
	let autocompleteElement = $state<google.maps.places.PlaceAutocompleteElement | null>(null);
	let selectedLatitude = $state<number | null>(null);
	let selectedLongitude = $state<number | null>(null);
	let radius = $state('50');
	let geolocating = $state(false);
	let geolocateError = $state('');

	// Handle place selection from autocomplete
	function handlePlaceSelect(event: Event) {
		logger.debug('Place select event: {event}', { event });

		const target = event.target as PlaceAutocompleteElement;
		let place: PlaceResult | null | undefined = null;

		// Method 1: Check if place is on the event itself
		const eventWithPlace = event as Event & { place?: PlaceResult };
		if (eventWithPlace.place) {
			place = eventWithPlace.place;
			logger.debug('Place from event: {place}', { place });
		}
		// Method 2: Check the target's value property
		else if (target?.value) {
			place = target.value;
			logger.debug('Place from target.value: {place}', { place });
		}
		// Method 3: Check autocompleteElement
		else if (autocompleteElement) {
			place = (autocompleteElement as PlaceAutocompleteElement).value;
			logger.debug('Place from autocompleteElement: {place}', { place });
		}

		if (place?.location) {
			selectedLatitude = place.location.lat();
			selectedLongitude = place.location.lng();
			logger.debug('Coordinates set: {lat}, {lng}', {
				lat: selectedLatitude,
				lng: selectedLongitude
			});
		} else {
			logger.warn('No location found in place object: {place}', { place });
		}
	}

	async function loadGoogleMapsScript(): Promise<void> {
		setOptions({
			key: GOOGLE_MAPS_API_KEY,
			v: 'weekly'
		});

		// Import the places library for autocomplete
		await importLibrary('places');
	}

	onMount(async () => {
		// Load Google Maps script when component mounts
		loadGoogleMapsScript();

		const queryParams = page.url.searchParams;
		const q = queryParams.get('q');
		const lat = queryParams.get('latitude');
		const lng = queryParams.get('longitude');
		const r = queryParams.get('radius');

		if (q) {
			searchQuery = q;
			searchType = 'name';
			await searchClubs();
		} else if (lat && lng && r) {
			selectedLatitude = parseFloat(lat);
			selectedLongitude = parseFloat(lng);
			radius = r;
			searchType = 'location';
			await searchClubs();
		}
	});

	async function searchClubs() {
		loading = true;
		error = '';

		try {
			let endpoint = '/clubs';

			if (
				searchType === 'location' &&
				selectedLatitude !== null &&
				selectedLongitude !== null &&
				radius
			) {
				const params = new URLSearchParams({
					latitude: selectedLatitude.toString(),
					longitude: selectedLongitude.toString(),
					radius: radius.toString()
				});
				endpoint = `/clubs?${params}`;
			} else if (searchQuery) {
				const params = new URLSearchParams({ q: searchQuery });
				endpoint = `/clubs?${params}`;
			}

			const response = await serverCall<DataListResponse<ClubWithSoaring>>(endpoint);
			clubs = response.data || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search clubs: ${errorMessage}`;
			clubs = []; // Clear any previous results on error
			logger.error('Error searching clubs: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	// Handle search input changes for autocomplete behavior
	async function handleSearchInput() {
		if (searchQuery.length > 0) {
			await searchClubs();
		} else {
			clubs = [];
		}
	}

	function getCurrentLocation() {
		if (!navigator.geolocation) {
			geolocateError = 'Geolocation is not supported by your browser';
			return;
		}

		geolocating = true;
		geolocateError = '';
		navigator.geolocation.getCurrentPosition(
			(position) => {
				selectedLatitude = position.coords.latitude;
				selectedLongitude = position.coords.longitude;
				geolocating = false;

				// Display coordinates in the autocomplete input
				if (autocompleteElement) {
					const lat = position.coords.latitude.toFixed(4);
					const lng = position.coords.longitude.toFixed(4);
					(autocompleteElement as unknown as { value: string }).value = `${lat}, ${lng}`;
				}

				// Automatically trigger search after getting location
				searchClubs();
			},
			(err) => {
				geolocating = false;
				if (err.code === err.PERMISSION_DENIED) {
					geolocateError =
						'Location access was denied. Please allow location access and try again.';
				} else if (err.code === err.POSITION_UNAVAILABLE) {
					geolocateError = 'Location information is unavailable.';
				} else if (err.code === err.TIMEOUT) {
					geolocateError = 'Location request timed out. Please try again.';
				} else {
					geolocateError = 'Unable to determine your location.';
				}
				logger.error('Error getting location: {error}', { error: err });
			}
		);
	}

	function formatDistance(meters: number | undefined | null): string {
		if (meters == null) return '—';
		const km = meters / 1000;
		const mi = km * 0.621371;
		if (mi < 1) return `${mi.toFixed(1)} mi`;
		return `${Math.round(mi)} mi`;
	}

	let isLocationSearch = $derived(searchType === 'location');

	function formatAddress(club: ClubWithSoaring): string {
		if (!club.location) {
			return 'Address not available';
		}

		const parts = [];
		if (club.location.street1) parts.push(club.location.street1);
		if (club.location.street2) parts.push(club.location.street2);
		if (club.location.city) parts.push(club.location.city);
		if (club.location.state) parts.push(club.location.state);
		if (club.location.zipCode) parts.push(club.location.zipCode);
		return parts.join(', ') || 'Address not available';
	}
</script>

<svelte:head>
	<title>Soaring Clubs - Glider Flights - SOAR</title>
	<meta
		name="description"
		content="Discover soaring clubs worldwide on SOAR. Browse gliding clubs, view their aircraft fleets, home base airports, and join the soaring community."
	/>
	<link rel="canonical" href="https://glider.flights/clubs" />
</svelte:head>

<div class="container mx-auto space-y-8 p-4">
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Users class="h-8 w-8" />
			Soaring Clubs
		</h1>
	</header>

	<!-- Search Section -->
	<section class="space-y-4 card p-6">
		<h3 class="mb-3 flex items-center gap-2 text-lg font-semibold">
			<Search class="h-5 w-5" />
			Search Clubs
		</h3>
		<div class="space-y-3 rounded-lg border p-3">
			<!-- Mobile: Vertical layout (segment above inputs) -->
			<div class="space-y-3 md:hidden">
				<!-- Search type selector -->
				<SegmentedControl
					name="search-type"
					value={searchType}
					orientation="vertical"
					onValueChange={(event: { value: string | null }) => {
						if (event.value && (event.value === 'name' || event.value === 'location')) {
							searchType = event.value;
							error = '';
						}
					}}
				>
					<SegmentedControl.Control>
						<SegmentedControl.Indicator />
						<SegmentedControl.Item value="name">
							<SegmentedControl.ItemText>
								<div class="flex flex-row items-center">
									<Search size={16} />
									<span class="ml-1">Search</span>
								</div>
							</SegmentedControl.ItemText>
							<SegmentedControl.ItemHiddenInput />
						</SegmentedControl.Item>
						<SegmentedControl.Item value="location">
							<SegmentedControl.ItemText>
								<div class="flex flex-row items-center">
									<MapPinHouse size={16} />
									<span class="ml-1">Location</span>
								</div>
							</SegmentedControl.ItemText>
							<SegmentedControl.ItemHiddenInput />
						</SegmentedControl.Item>
					</SegmentedControl.Control>
				</SegmentedControl>

				{#if searchType === 'name'}
					<input
						bind:value={searchQuery}
						oninput={handleSearchInput}
						class="input"
						type="text"
						placeholder="Type to search clubs..."
						autocomplete="off"
					/>
				{:else if searchType === 'location'}
					<div class="space-y-3">
						<gmp-place-autocomplete
							bind:this={autocompleteElement}
							placeholder="Enter a city or location"
							ongmpplaceselect={handlePlaceSelect}
							class="google-autocomplete"
						></gmp-place-autocomplete>

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

						<button class="btn w-full preset-filled-primary-500" onclick={searchClubs}>
							<Search class="mr-2 h-4 w-4" />
							Search
						</button>

						<button
							class="preset-tonal-surface-500 btn w-full"
							onclick={getCurrentLocation}
							disabled={geolocating}
						>
							{#if geolocating}
								<Progress class="mr-2 h-4 w-4" />
								Getting location...
							{:else}
								<MapPinHouse class="mr-2 h-4 w-4" />
								Use My Location
							{/if}
						</button>

						{#if geolocateError}
							<p class="text-sm text-error-500">{geolocateError}</p>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Desktop: Horizontal layout (segment to the left of inputs) -->
			<div class="hidden md:block">
				<div class="grid grid-cols-[200px_1fr] items-start gap-4">
					<!-- Search type selector -->
					<SegmentedControl
						name="search-type-desktop"
						value={searchType}
						orientation="vertical"
						onValueChange={(event: { value: string | null }) => {
							if (event.value && (event.value === 'name' || event.value === 'location')) {
								searchType = event.value;
								error = '';
							}
						}}
					>
						<SegmentedControl.Control>
							<SegmentedControl.Indicator />
							<SegmentedControl.Item value="name">
								<SegmentedControl.ItemText>
									<div class="flex flex-row items-center">
										<Search size={16} />
										<span class="ml-1">Search</span>
									</div>
								</SegmentedControl.ItemText>
								<SegmentedControl.ItemHiddenInput />
							</SegmentedControl.Item>
							<SegmentedControl.Item value="location">
								<SegmentedControl.ItemText>
									<div class="flex flex-row items-center">
										<MapPinHouse size={16} />
										<span class="ml-1">Location</span>
									</div>
								</SegmentedControl.ItemText>
								<SegmentedControl.ItemHiddenInput />
							</SegmentedControl.Item>
						</SegmentedControl.Control>
					</SegmentedControl>

					<!-- Input area -->
					<div>
						{#if searchType === 'name'}
							<input
								bind:value={searchQuery}
								oninput={handleSearchInput}
								class="input"
								type="text"
								placeholder="Type to search clubs..."
								autocomplete="off"
							/>
						{:else if searchType === 'location'}
							<div class="space-y-3">
								<div class="flex gap-3">
									<div class="flex-1">
										<gmp-place-autocomplete
											bind:this={autocompleteElement}
											placeholder="Enter a city or location"
											ongmpplaceselect={handlePlaceSelect}
											class="google-autocomplete"
										></gmp-place-autocomplete>
									</div>
									<label class="label w-32">
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

								<div class="flex gap-3">
									<button class="btn flex-1 preset-filled-primary-500" onclick={searchClubs}>
										<Search class="mr-2 h-4 w-4" />
										Search
									</button>
									<button
										class="preset-tonal-surface-500 btn"
										onclick={getCurrentLocation}
										disabled={geolocating}
									>
										{#if geolocating}
											<Progress class="mr-2 h-4 w-4" />
											Getting location...
										{:else}
											<MapPinHouse class="mr-2 h-4 w-4" />
											Use My Location
										{/if}
									</button>
								</div>

								{#if geolocateError}
									<p class="text-sm text-error-500">{geolocateError}</p>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			</div>

			<!-- Error message display -->
			{#if error}
				<div class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600">
					{error}
				</div>
			{/if}
		</div>
	</section>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<Progress class="h-8 w-8" />
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
							{#if isLocationSearch}
								<th>Distance</th>
							{/if}
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
									{#if club.homeBaseAirportIdent}
										<a
											href={resolve(`/airports/${club.homeBaseAirportId}`)}
											target="_blank"
											rel="noopener noreferrer"
											class="flex items-center gap-1 anchor font-mono text-sm text-primary-500 hover:text-primary-600"
										>
											<Plane class="h-4 w-4" />
											<span>{club.homeBaseAirportIdent}</span>
											<ExternalLink class="h-3 w-3" />
										</a>
									{:else}
										<span class="text-surface-500">—</span>
									{/if}
								</td>
								{#if isLocationSearch}
									<td>
										<span class="text-surface-600-300-token text-sm">
											{formatDistance(club.distanceMeters)}
										</span>
									</td>
								{/if}
								<td>
									<a
										href={resolve(`/clubs/${club.id}`)}
										class="preset-tonal-surface-500 btn flex items-center gap-1 btn-sm"
									>
										<ExternalLink class="h-3 w-3" />
										View
									</a>
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

						{#if club.homeBaseAirportIdent}
							<div class="flex items-center gap-2">
								<Plane class="h-4 w-4 flex-shrink-0 text-surface-500" />
								<span class="text-surface-600-300-token">
									Airport:
									<a
										href={resolve(`/airports/${club.homeBaseAirportId}`)}
										target="_blank"
										rel="noopener noreferrer"
										class="inline-flex items-center gap-1 anchor font-mono text-primary-500 hover:text-primary-600"
									>
										{club.homeBaseAirportIdent}
										<ExternalLink class="h-3 w-3" />
									</a>
								</span>
							</div>
						{/if}

						{#if isLocationSearch && club.distanceMeters != null}
							<div class="flex items-center gap-2">
								<MapPinHouse class="h-4 w-4 flex-shrink-0 text-surface-500" />
								<span class="text-surface-600-300-token">
									{formatDistance(club.distanceMeters)} away
								</span>
							</div>
						{/if}
					</div>
				</article>
			{/each}
		</div>
	{:else if !loading && !error && clubs.length === 0 && (searchQuery || (selectedLatitude !== null && selectedLongitude !== null))}
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

<style>
	/* Dark mode support for Google Maps autocomplete */
	/* The gmp-place-autocomplete component automatically respects color-scheme */
	gmp-place-autocomplete {
		width: 100%;
		color-scheme: light;
	}

	/* Dark mode - component will automatically adapt to dark color scheme */
	:global(.dark) gmp-place-autocomplete {
		color-scheme: dark;
	}
</style>
