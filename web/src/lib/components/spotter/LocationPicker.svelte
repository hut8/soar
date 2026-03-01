<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { X, MapPin, Search, LocateFixed } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import type { Airport, DataListResponse } from '$lib/types';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';

	let {
		onSelect,
		onClose,
		initialLat = null,
		initialLon = null
	} = $props<{
		onSelect: (lat: number, lon: number, label?: string) => void;
		onClose: () => void;
		initialLat?: number | null;
		initialLon?: number | null;
	}>();

	let mapContainer: HTMLDivElement | undefined = $state();
	let map: maplibregl.Map | null = $state(null);
	let marker: maplibregl.Marker | null = $state(null);

	let searchQuery = $state('');
	let searchResults = $state<Airport[]>([]);
	let searching = $state(false);
	let searchError = $state<string | null>(null);
	let locating = $state(false);

	let selectedLat = $state<number | null>(initialLat);
	let selectedLon = $state<number | null>(initialLon);
	let selectedLabel = $state<string | undefined>(undefined);

	const hasSelection = $derived(selectedLat !== null && selectedLon !== null);

	function placeMarker(lat: number, lon: number) {
		if (!map) return;

		selectedLat = lat;
		selectedLon = lon;

		if (marker) {
			marker.setLngLat([lon, lat]);
		} else {
			marker = new maplibregl.Marker({ color: '#ef4444' }).setLngLat([lon, lat]).addTo(map);
		}

		map.flyTo({ center: [lon, lat], zoom: Math.max(map.getZoom(), 10) });
	}

	async function handleSearch() {
		const q = searchQuery.trim();
		if (!q) return;

		searching = true;
		searchError = null;
		try {
			const response = await serverCall<DataListResponse<Airport>>(
				`/airports?q=${encodeURIComponent(q)}`
			);
			searchResults = (response.data || []).slice(0, 8);
		} catch (err) {
			searchError = err instanceof Error ? err.message : 'Search failed';
			searchResults = [];
		} finally {
			searching = false;
		}
	}

	function selectAirport(airport: Airport) {
		if (airport.latitudeDeg != null && airport.longitudeDeg != null) {
			selectedLabel = `${airport.ident} - ${airport.name}`;
			placeMarker(airport.latitudeDeg, airport.longitudeDeg);
			searchResults = [];
			searchQuery = '';
		}
	}

	function handleGeolocate() {
		if (!navigator.geolocation) return;
		locating = true;
		navigator.geolocation.getCurrentPosition(
			(position) => {
				selectedLabel = 'My Location';
				placeMarker(position.coords.latitude, position.coords.longitude);
				locating = false;
			},
			() => {
				locating = false;
			},
			{ enableHighAccuracy: true, timeout: 10000 }
		);
	}

	function handleConfirm() {
		if (selectedLat !== null && selectedLon !== null) {
			onSelect(selectedLat, selectedLon, selectedLabel);
		}
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			handleSearch();
		}
	}

	onMount(() => {
		if (!mapContainer) return;

		const center: [number, number] =
			initialLon != null && initialLat != null ? [initialLon, initialLat] : [-98.58, 39.83]; // CONUS center

		const zoom = initialLat != null ? 10 : 4;

		map = new maplibregl.Map({
			container: mapContainer,
			style: {
				version: 8,
				sources: {
					osm: {
						type: 'raster',
						tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
						tileSize: 256,
						maxzoom: 19,
						attribution: '&copy; OpenStreetMap contributors'
					}
				},
				layers: [
					{
						id: 'osm',
						type: 'raster',
						source: 'osm'
					}
				]
			},
			center,
			zoom
		});

		map.addControl(new maplibregl.NavigationControl(), 'top-right');

		map.on('click', (e: maplibregl.MapMouseEvent) => {
			selectedLabel = undefined;
			placeMarker(e.lngLat.lat, e.lngLat.lng);
		});

		if (initialLat != null && initialLon != null) {
			placeMarker(initialLat, initialLon);
		}
	});

	onDestroy(() => {
		if (marker) marker.remove();
		if (map) map.remove();
	});
</script>

<div class="modal-backdrop" role="presentation">
	<div
		class="modal-content"
		onclick={(e) => e.stopPropagation()}
		role="dialog"
		aria-labelledby="location-picker-title"
	>
		<div class="modal-header">
			<h2 id="location-picker-title">Choose Location</h2>
			<button class="btn-icon" onclick={onClose}>
				<X size={20} />
			</button>
		</div>

		<!-- Search bar -->
		<div class="search-bar">
			<div class="search-input-wrap">
				<Search size={16} class="search-icon" />
				<input
					type="text"
					placeholder="Search airport (ICAO, name...)"
					bind:value={searchQuery}
					onkeydown={handleSearchKeydown}
					class="search-input"
				/>
				{#if searchQuery}
					<button
						class="btn-clear"
						onclick={() => {
							searchQuery = '';
							searchResults = [];
						}}
					>
						<X size={14} />
					</button>
				{/if}
			</div>
			<button class="btn-search" onclick={handleSearch} disabled={searching || !searchQuery.trim()}>
				{searching ? 'Searching...' : 'Search'}
			</button>
		</div>

		{#if searchError}
			<div class="search-error">{searchError}</div>
		{/if}

		<!-- Search results -->
		{#if searchResults.length > 0}
			<div class="search-results">
				{#each searchResults as airport (airport.id)}
					<button class="result-item" onclick={() => selectAirport(airport)}>
						<MapPin size={14} />
						<span class="result-ident">{airport.ident}</span>
						<span class="result-name">{airport.name}</span>
					</button>
				{/each}
			</div>
		{/if}

		<!-- Map -->
		<div class="map-container" bind:this={mapContainer}></div>

		<!-- Actions -->
		<div class="modal-actions">
			<button class="btn-geo" onclick={handleGeolocate} disabled={locating}>
				<LocateFixed size={16} />
				{locating ? 'Locating...' : 'Use My Location'}
			</button>
			<button class="btn-confirm" onclick={handleConfirm} disabled={!hasSelection}>
				{#if hasSelection}
					Go to Location
				{:else}
					Click map or search
				{/if}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(4px);
		z-index: 300;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.modal-content {
		background: rgba(20, 20, 20, 0.97);
		backdrop-filter: blur(12px);
		border-radius: 1rem;
		width: 100%;
		max-width: 600px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	.modal-header h2 {
		color: white;
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	.btn-icon {
		background: rgba(255, 255, 255, 0.1);
		border: none;
		border-radius: 0.5rem;
		padding: 0.5rem;
		color: white;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.search-bar {
		display: flex;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
	}

	.search-input-wrap {
		flex: 1;
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-input-wrap :global(.search-icon) {
		position: absolute;
		left: 0.75rem;
		color: rgba(255, 255, 255, 0.4);
		pointer-events: none;
	}

	.search-input {
		width: 100%;
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 0.5rem;
		padding: 0.5rem 2rem 0.5rem 2.25rem;
		color: white;
		font-size: 0.875rem;
		outline: none;
	}

	.search-input:focus {
		border-color: rgb(var(--color-primary-500));
	}

	.search-input::placeholder {
		color: rgba(255, 255, 255, 0.4);
	}

	.btn-clear {
		position: absolute;
		right: 0.5rem;
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.5);
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
	}

	.btn-search {
		background: rgb(var(--color-primary-500));
		border: none;
		border-radius: 0.5rem;
		padding: 0.5rem 1rem;
		color: white;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		white-space: nowrap;
	}

	.btn-search:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.search-error {
		padding: 0.5rem 1rem;
		color: #ef4444;
		font-size: 0.8125rem;
	}

	.search-results {
		max-height: 12rem;
		overflow-y: auto;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	.result-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.625rem 1rem;
		background: transparent;
		border: none;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		color: white;
		cursor: pointer;
		text-align: left;
		font-size: 0.8125rem;
	}

	.result-item:hover {
		background: rgba(255, 255, 255, 0.05);
	}

	.result-ident {
		font-weight: 700;
		font-family: monospace;
		min-width: 3.5rem;
	}

	.result-name {
		opacity: 0.7;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.map-container {
		flex: 1;
		min-height: 300px;
	}

	.modal-actions {
		display: flex;
		gap: 0.75rem;
		padding: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.1);
	}

	.btn-geo {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 0.5rem;
		padding: 0.625rem 1rem;
		color: white;
		font-size: 0.875rem;
		cursor: pointer;
		white-space: nowrap;
	}

	.btn-geo:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-confirm {
		flex: 1;
		background: rgb(var(--color-primary-500));
		border: none;
		border-radius: 0.5rem;
		padding: 0.625rem 1rem;
		color: white;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-confirm:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
