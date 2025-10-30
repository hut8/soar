<script lang="ts">
	import { Search, MapPin, Radio, Navigation } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { serverCall } from '$lib/api/server';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';

	dayjs.extend(relativeTime);

	interface Receiver {
		id: string;
		callsign: string;
		description: string | null;
		contact: string | null;
		email: string | null;
		country: string | null;
		latitude: number | null;
		longitude: number | null;
		created_at: string;
		updated_at: string;
	}

	interface ReceiverSearchResponse {
		receivers: Receiver[];
	}

	let receivers = $state<Receiver[]>([]);
	let loading = $state(false);
	let error = $state('');
	let searchMode = $state<'query' | 'location'>('query');

	// Query search
	let searchQuery = $state('');

	// Location search
	let autocompleteElement = $state<google.maps.places.PlaceAutocompleteElement | null>(null);
	let selectedLatitude = $state<number | null>(null);
	let selectedLongitude = $state<number | null>(null);
	let radiusMiles = $state(100);
	let gettingLocation = $state(false);

	// Handle place selection from autocomplete
	function handlePlaceSelect(event: Event) {
		const placeEvent = event as CustomEvent;
		const place = placeEvent.detail.place;

		if (place?.location) {
			selectedLatitude = place.location.lat();
			selectedLongitude = place.location.lng();
		}
	}

	async function searchReceivers() {
		loading = true;
		error = '';

		try {
			let endpoint = '/receivers';
			const queryParams: string[] = [];

			if (searchMode === 'query') {
				if (!searchQuery.trim()) {
					error = 'Please enter a search query';
					loading = false;
					return;
				}
				queryParams.push(`query=${encodeURIComponent(searchQuery.trim())}`);
			} else {
				// location search
				if (selectedLatitude === null || selectedLongitude === null) {
					error = 'Please select a location from the autocomplete';
					loading = false;
					return;
				}
				queryParams.push(`latitude=${selectedLatitude}`);
				queryParams.push(`longitude=${selectedLongitude}`);
				queryParams.push(`radius_miles=${radiusMiles}`);
			}

			if (queryParams.length > 0) {
				endpoint += `?${queryParams.join('&')}`;
			}
			const response = await serverCall<ReceiverSearchResponse>(endpoint);
			receivers = response.receivers || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search receivers: ${errorMessage}`;
			console.error('Error searching receivers:', err);
			receivers = [];
		} finally {
			loading = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			searchReceivers();
		}
	}

	async function useMyLocation() {
		if (!navigator.geolocation) {
			error = 'Geolocation is not supported by your browser';
			return;
		}

		gettingLocation = true;
		error = '';

		try {
			const position = await new Promise<GeolocationPosition>((resolve, reject) => {
				navigator.geolocation.getCurrentPosition(resolve, reject, {
					enableHighAccuracy: true,
					timeout: 10000,
					maximumAge: 0
				});
			});

			selectedLatitude = position.coords.latitude;
			selectedLongitude = position.coords.longitude;

			// Automatically search with the user's location
			await searchReceivers();
		} catch (err) {
			if (err instanceof GeolocationPositionError) {
				switch (err.code) {
					case err.PERMISSION_DENIED:
						error = 'Location permission denied. Please enable location access in your browser.';
						break;
					case err.POSITION_UNAVAILABLE:
						error = 'Location information is unavailable.';
						break;
					case err.TIMEOUT:
						error = 'Location request timed out.';
						break;
					default:
						error = 'An error occurred while getting your location.';
				}
			} else {
				error = 'Failed to get your location';
			}
			console.error('Geolocation error:', err);
		} finally {
			gettingLocation = false;
		}
	}

	function formatCoordinates(
		lat: number | null | undefined,
		lng: number | null | undefined
	): string {
		if (lat == null || lng == null) return '—';
		return `${lat.toFixed(4)}, ${lng.toFixed(4)}`;
	}

	function getLastHeard(updatedAt: string): string {
		return dayjs(updatedAt).fromNow();
	}

	async function loadGoogleMapsScript(): Promise<void> {
		setOptions({
			key: GOOGLE_MAPS_API_KEY,
			v: 'weekly'
		});

		// Import the places library for autocomplete
		await importLibrary('places');
	}

	async function loadRecentReceivers() {
		loading = true;
		error = '';

		try {
			// Call API without any query parameters to get recently updated receivers
			const response = await serverCall<ReceiverSearchResponse>('/receivers');
			receivers = response.receivers || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load receivers: ${errorMessage}`;
			console.error('Error loading recent receivers:', err);
			receivers = [];
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		// Load Google Maps script when component mounts
		loadGoogleMapsScript();
		// Load recently updated receivers
		loadRecentReceivers();
	});
</script>

<svelte:head>
	<title>Receivers - Receiver Search</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Radio class="h-8 w-8" />
			Receiver Search
		</h1>
	</header>

	<!-- Search Section -->
	<section class="space-y-4 card p-6">
		<h3 class="mb-3 flex items-center gap-2 text-lg font-semibold">
			<Search class="h-5 w-5" />
			Search Receivers
		</h3>

		<!-- Search Mode Toggle -->
		<div class="mb-4 flex gap-4">
			<button
				class="btn {searchMode === 'query' ? 'preset-filled-primary-500' : 'preset-outlined'}"
				onclick={() => {
					searchMode = 'query';
					error = '';
				}}
			>
				<Search class="h-4 w-4" />
				Text Search
			</button>
			<button
				class="btn {searchMode === 'location' ? 'preset-filled-primary-500' : 'preset-outlined'}"
				onclick={() => {
					searchMode = 'location';
					error = '';
				}}
			>
				<MapPin class="h-4 w-4" />
				Location Search
			</button>
		</div>

		{#if searchMode === 'query'}
			<!-- Query Search -->
			<div class="space-y-3 rounded-lg border p-3">
				<input
					class="input"
					placeholder="Search by callsign, description, country, contact, or email"
					bind:value={searchQuery}
					onkeydown={handleKeydown}
					oninput={() => (error = '')}
				/>

				<button
					class="btn w-full preset-filled-primary-500"
					onclick={searchReceivers}
					disabled={loading}
				>
					{#if loading}
						Searching...
					{:else}
						Search
					{/if}
				</button>
			</div>
		{:else}
			<!-- Location Search -->
			<div class="space-y-3 rounded-lg border p-3">
				<div class="flex gap-2">
					<div class="min-w-0 flex-1">
						<gmp-place-autocomplete
							bind:this={autocompleteElement}
							placeholder="Enter a city or location"
							ongmpplaceselect={handlePlaceSelect}
						></gmp-place-autocomplete>
					</div>
					<button
						class="preset-tonal-primary-500 btn flex-shrink-0"
						onclick={useMyLocation}
						disabled={gettingLocation || loading}
						title="Use my current location"
					>
						{#if gettingLocation}
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
							></div>
						{:else}
							<Navigation class="h-4 w-4" />
						{/if}
					</button>
				</div>

				<div class="flex items-center gap-3">
					<label class="flex-1">
						<span class="text-sm font-medium">Radius (miles)</span>
						<input type="number" class="input" min="1" max="1000" bind:value={radiusMiles} />
					</label>
				</div>

				<button
					class="btn w-full preset-filled-primary-500"
					onclick={searchReceivers}
					disabled={loading}
				>
					{#if loading}
						Searching...
					{:else}
						Search
					{/if}
				</button>
			</div>
		{/if}

		{#if error}
			<div class="alert preset-filled-error-500">{error}</div>
		{/if}
	</section>

	<!-- Results Section -->
	{#if receivers.length > 0}
		<section class="space-y-4">
			<h2 class="h3">
				{#if !searchQuery && selectedLatitude === null}
					Recently Updated Receivers
				{:else}
					Results
				{/if}
				<span class="text-surface-500-400-token">({receivers.length})</span>
			</h2>

			<!-- Mobile: Card Layout -->
			<div class="grid gap-4 md:hidden">
				{#each receivers as receiver (receiver.id)}
					<a
						href={resolve(`/receivers/${receiver.id}`)}
						class="card p-6 transition-all duration-200 hover:scale-[1.02] hover:shadow-lg"
					>
						<div class="space-y-2">
							<h3 class="h4 font-bold">{receiver.callsign}</h3>

							{#if receiver.description}
								<p class="text-surface-600-300-token text-sm">{receiver.description}</p>
							{/if}

							<div class="space-y-1 text-sm">
								{#if receiver.country}
									<div class="flex items-center gap-2">
										<MapPin class="h-4 w-4" />
										<span>{receiver.country}</span>
									</div>
								{/if}

								{#if receiver.latitude != null && receiver.longitude != null}
									<div class="text-surface-500-400-token text-xs">
										{formatCoordinates(receiver.latitude, receiver.longitude)}
									</div>
								{/if}

								<div class="text-surface-500-400-token text-xs">
									Last heard: {getLastHeard(receiver.updated_at)}
								</div>
							</div>
						</div>
					</a>
				{/each}
			</div>

			<!-- Desktop: Table Layout -->
			<div class="hidden card md:block">
				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Callsign</th>
								<th>Description</th>
								<th>Country</th>
								<th>Coordinates</th>
								<th>Last Heard</th>
							</tr>
						</thead>
						<tbody>
							{#each receivers as receiver (receiver.id)}
								<tr
									class="cursor-pointer"
									onclick={() => (window.location.href = resolve(`/receivers/${receiver.id}`))}
								>
									<td class="font-semibold">{receiver.callsign}</td>
									<td class="text-surface-600-300-token">
										{receiver.description || '—'}
									</td>
									<td>{receiver.country || '—'}</td>
									<td class="text-surface-500-400-token text-sm">
										{formatCoordinates(receiver.latitude, receiver.longitude)}
									</td>
									<td class="text-surface-500-400-token text-sm">
										{getLastHeard(receiver.updated_at)}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		</section>
	{:else if !loading && receivers.length === 0 && (searchQuery || selectedLatitude !== null)}
		<div class="text-surface-500-400-token card p-6 text-center">
			No receivers found matching your search criteria.
		</div>
	{/if}
</div>
