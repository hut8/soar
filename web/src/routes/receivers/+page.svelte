<script lang="ts">
	import { Search, MapPin, Radio } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { serverCall } from '$lib/api/server';
	import { Loader } from '@googlemaps/js-api-loader';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';

	dayjs.extend(relativeTime);

	const GOOGLE_MAPS_API_KEY = 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

	interface Receiver {
		id: number;
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
	let locationInput = $state<HTMLInputElement>();
	let selectedLatitude = $state<number | null>(null);
	let selectedLongitude = $state<number | null>(null);
	let radiusMiles = $state(100);
	let autocomplete = $state<google.maps.places.Autocomplete | null>(null);

	// Initialize Google Places Autocomplete
	async function initAutocomplete() {
		// Wait for Google Maps to load
		if (typeof google === 'undefined' || !google.maps || !google.maps.places) {
			console.warn('Google Maps not loaded yet, retrying...');
			setTimeout(initAutocomplete, 100);
			return;
		}

		if (locationInput && !autocomplete) {
			autocomplete = new google.maps.places.Autocomplete(locationInput, {
				types: ['(cities)'],
				fields: ['geometry', 'name', 'formatted_address']
			});

			autocomplete.addListener('place_changed', () => {
				const place = autocomplete?.getPlace();
				if (place?.geometry?.location) {
					selectedLatitude = place.geometry.location.lat();
					selectedLongitude = place.geometry.location.lng();
				}
			});
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

	function formatCoordinates(lat: number | null, lng: number | null): string {
		if (lat === null || lng === null) return '—';
		return `${lat.toFixed(4)}, ${lng.toFixed(4)}`;
	}

	function getLastHeard(updatedAt: string): string {
		return dayjs(updatedAt).fromNow();
	}

	async function loadGoogleMapsScript(): Promise<void> {
		const loader = new Loader({
			apiKey: GOOGLE_MAPS_API_KEY,
			version: 'weekly'
		});

		// Import the places library for autocomplete
		await loader.importLibrary('places');
	}

	onMount(() => {
		// Load Google Maps script when component mounts
		loadGoogleMapsScript();
	});

	$effect(() => {
		if (searchMode === 'location' && locationInput) {
			initAutocomplete();
		}
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
				<input
					bind:this={locationInput}
					class="input"
					placeholder="Enter a city or location"
					oninput={() => (error = '')}
				/>

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
				Results <span class="text-surface-500-400-token">({receivers.length})</span>
			</h2>

			<div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each receivers as receiver (receiver.id)}
					<a
						href={resolve(`/receivers/${receiver.id}`)}
						class="hover:preset-filled-surface-200 card p-4"
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

								{#if receiver.latitude !== null && receiver.longitude !== null}
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
		</section>
	{:else if !loading && receivers.length === 0 && (searchQuery || selectedLatitude !== null)}
		<div class="text-surface-500-400-token card p-6 text-center">
			No receivers found matching your search criteria.
		</div>
	{/if}
</div>
