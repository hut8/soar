<script lang="ts">
	import { onMount } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { serverCall } from '$lib/api/server';
	import { Loader, Calendar, Layers, Radio, Filter, ChevronDown, ChevronUp } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import type { CoverageHexProperties, CoverageGeoJsonResponse } from '$lib/types';

	let mapContainer: HTMLDivElement;
	let map: maplibregl.Map | null = null;
	let loading = false;
	let error = '';
	let resolution = 7;
	let hexCount = 0;
	let receivers: { id: string; callsign: string }[] = [];
	let selectedReceiverId = '';
	let minAltitude = 0;
	let maxAltitude = 50000;
	let showAdvancedFilters = false;

	// Default to last 30 days
	function getDefaultDates() {
		const now = Date.now();
		const thirtyDaysMs = 30 * 24 * 60 * 60 * 1000;
		const todayStr = new Date(now).toISOString().split('T')[0];
		const thirtyDaysAgoStr = new Date(now - thirtyDaysMs).toISOString().split('T')[0];
		return { start: thirtyDaysAgoStr, end: todayStr };
	}

	const defaultDates = getDefaultDates();
	let startDate = defaultDates.start;
	let endDate = defaultDates.end;

	async function loadReceivers() {
		try {
			const response = await serverCall<{ receivers: { id: string; callsign: string }[] }>(
				'/receivers'
			);
			receivers = response.receivers || [];
		} catch (err) {
			console.error('Failed to load receivers:', err);
		}
	}

	onMount(() => {
		// Load receivers list
		loadReceivers();

		// Initialize MapLibre map centered on US
		map = new maplibregl.Map({
			container: mapContainer,
			style: {
				version: 8,
				sources: {
					osm: {
						type: 'raster',
						tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
						tileSize: 256,
						attribution:
							'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
					}
				},
				layers: [
					{
						id: 'osm',
						type: 'raster',
						source: 'osm',
						minzoom: 0,
						maxzoom: 19
					}
				]
			},
			center: [-98.5, 39.8], // Center of US
			zoom: 4
		});

		map.addControl(new maplibregl.NavigationControl(), 'top-right');

		// Load coverage data when map is ready
		map.on('load', () => {
			loadCoverage();
		});

		// Reload coverage when user stops moving the map
		map.on('moveend', () => {
			loadCoverage();
		});

		return () => {
			map?.remove();
		};
	});

	async function loadCoverage() {
		if (!map) return;

		loading = true;
		error = '';

		try {
			const bounds = map.getBounds();
			const west = bounds.getWest();
			const east = bounds.getEast();
			const south = bounds.getSouth();
			const north = bounds.getNorth();

			// Build query parameters
			// eslint-disable-next-line svelte/prefer-svelte-reactivity -- URLSearchParams created fresh on each call, no persistent state
			const params = new URLSearchParams({
				resolution: resolution.toString(),
				west: west.toString(),
				east: east.toString(),
				south: south.toString(),
				north: north.toString(),
				start_date: startDate,
				end_date: endDate,
				limit: '5000'
			});

			// Add optional filters if they're set
			if (selectedReceiverId) {
				params.append('receiver_id', selectedReceiverId);
			}
			if (minAltitude > 0) {
				params.append('min_altitude', minAltitude.toString());
			}
			if (maxAltitude < 50000) {
				params.append('max_altitude', maxAltitude.toString());
			}

			const response = await serverCall<CoverageGeoJsonResponse>(
				`/coverage/hexes?${params.toString()}`
			);

			hexCount = response.features?.length || 0;

			// Remove existing coverage layer if it exists
			if (map.getLayer('coverage-hexes')) {
				map.removeLayer('coverage-hexes');
			}
			if (map.getSource('coverage')) {
				map.removeSource('coverage');
			}

			// Add coverage source and layer
			map.addSource('coverage', {
				type: 'geojson',
				data: response
			});

			// Calculate max fix count for color scaling
			const maxFixCount = Math.max(...response.features.map((f) => f.properties.fixCount), 1);

			map.addLayer({
				id: 'coverage-hexes',
				type: 'fill',
				source: 'coverage',
				paint: {
					'fill-color': [
						'interpolate',
						['linear'],
						['get', 'fixCount'],
						0,
						'#440154', // Dark purple (no coverage)
						maxFixCount * 0.25,
						'#31688e', // Blue
						maxFixCount * 0.5,
						'#35b779', // Green
						maxFixCount * 0.75,
						'#fde724', // Yellow
						maxFixCount,
						'#ff0000' // Red (high coverage)
					],
					'fill-opacity': 0.6,
					'fill-outline-color': '#ffffff'
				}
			});

			// Add popup on hover
			map.on('mousemove', 'coverage-hexes', (e: maplibregl.MapLayerMouseEvent) => {
				if (!e.features || !e.features[0]) return;

				map!.getCanvas().style.cursor = 'pointer';

				const feature = e.features[0] as maplibregl.MapGeoJSONFeature & {
					properties: CoverageHexProperties;
				};
				const props = feature.properties;

				const popup = new maplibregl.Popup({
					closeButton: false,
					closeOnClick: false
				})
					.setLngLat(e.lngLat)
					.setHTML(
						`
						<div class="p-2">
							<p class="font-semibold">Coverage Hex</p>
							<p class="text-sm">Fixes: ${props.fixCount.toLocaleString()}</p>
							<p class="text-sm">Coverage: ${props.coverageHours.toFixed(1)} hours</p>
							${props.avgAltitudeMslFeet ? `<p class="text-sm">Avg Altitude: ${props.avgAltitudeMslFeet.toLocaleString()} ft</p>` : ''}
							${props.minAltitudeMslFeet !== null && props.maxAltitudeMslFeet !== null ? `<p class="text-sm">Altitude Range: ${props.minAltitudeMslFeet.toLocaleString()}-${props.maxAltitudeMslFeet.toLocaleString()} ft</p>` : ''}
							<p class="text-sm text-gray-500">Resolution: ${props.resolution}</p>
						</div>
					`
					)
					.addTo(map!);

				// Remove popup when mouse leaves
				map!.once('mouseleave', 'coverage-hexes', () => {
					popup.remove();
					map!.getCanvas().style.cursor = '';
				});
			});
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load coverage: ${errorMessage}`;
			console.error('Coverage load error:', err);
		} finally {
			loading = false;
		}
	}

	function handleResolutionChange() {
		loadCoverage();
	}

	function handleDateChange() {
		loadCoverage();
	}

	function handleReceiverChange() {
		loadCoverage();
	}

	function handleAltitudeChange() {
		loadCoverage();
	}

	function toggleAdvancedFilters() {
		showAdvancedFilters = !showAdvancedFilters;
	}
</script>

<div class="flex h-screen flex-col">
	<!-- Header -->
	<div class="bg-surface-800 p-4 shadow-md">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-2xl font-bold text-white">Receiver Coverage Map</h1>
				<p class="text-sm text-gray-300">
					Visualizing aircraft reception coverage using H3 hexagons
				</p>
			</div>
			<a href={resolve('/receivers')} class="btn gap-2 preset-outlined">
				<Radio class="h-4 w-4" />
				View Receivers
			</a>
		</div>
	</div>

	<!-- Controls -->
	<div class="bg-surface-700 p-4 shadow-sm">
		<div class="flex flex-wrap gap-4">
			<!-- Resolution selector -->
			<div class="flex items-center gap-2">
				<Layers class="h-5 w-5 text-gray-300" />
				<label for="resolution" class="text-sm font-medium text-gray-300">Resolution:</label>
				<select
					id="resolution"
					bind:value={resolution}
					onchange={handleResolutionChange}
					class="variant-filled-surface select w-24"
				>
					<option value={6}>6 (~36 km²)</option>
					<option value={7}>7 (~5 km²)</option>
					<option value={8}>8 (~0.7 km²)</option>
				</select>
			</div>

			<!-- Date range -->
			<div class="flex items-center gap-2">
				<Calendar class="h-5 w-5 text-gray-300" />
				<label for="start-date" class="text-sm font-medium text-gray-300">From:</label>
				<input
					id="start-date"
					type="date"
					bind:value={startDate}
					onchange={handleDateChange}
					class="variant-filled-surface input w-40"
				/>
				<label for="end-date" class="text-sm font-medium text-gray-300">To:</label>
				<input
					id="end-date"
					type="date"
					bind:value={endDate}
					onchange={handleDateChange}
					class="variant-filled-surface input w-40"
				/>
			</div>

			<!-- Advanced Filters Toggle -->
			<button
				onclick={toggleAdvancedFilters}
				class="btn gap-2 preset-outlined"
				title="Toggle advanced filters"
			>
				<Filter class="h-4 w-4" />
				Advanced
				{#if showAdvancedFilters}
					<ChevronUp class="h-4 w-4" />
				{:else}
					<ChevronDown class="h-4 w-4" />
				{/if}
			</button>

			<!-- Stats -->
			<div class="ml-auto flex items-center gap-2 text-sm text-gray-300">
				{#if loading}
					<Loader class="h-4 w-4 animate-spin" />
					<span>Loading...</span>
				{:else}
					<span class="font-semibold">{hexCount.toLocaleString()}</span>
					<span>hexagons</span>
				{/if}
			</div>
		</div>

		<!-- Advanced Filters Panel -->
		{#if showAdvancedFilters}
			<div class="mt-4 space-y-4 rounded border border-surface-500 p-4">
				<h3 class="flex items-center gap-2 text-sm font-semibold text-gray-300">
					<Filter class="h-4 w-4" />
					Advanced Filters
				</h3>

				<div class="grid gap-4 md:grid-cols-2">
					<!-- Receiver Filter -->
					<div class="space-y-2">
						<label for="receiver" class="text-sm font-medium text-gray-300">Receiver:</label>
						<select
							id="receiver"
							bind:value={selectedReceiverId}
							onchange={handleReceiverChange}
							class="variant-filled-surface select w-full"
						>
							<option value="">All Receivers</option>
							{#each receivers as receiver (receiver.id)}
								<option value={receiver.id}>{receiver.callsign}</option>
							{/each}
						</select>
					</div>

					<!-- Altitude Filter -->
					<div class="space-y-2">
						<label class="text-sm font-medium text-gray-300">
							Altitude: {minAltitude.toLocaleString()} - {maxAltitude.toLocaleString()} ft
						</label>
						<div class="flex gap-2">
							<div class="flex-1 space-y-1">
								<label for="min-altitude" class="text-xs text-gray-400">Min:</label>
								<input
									id="min-altitude"
									type="range"
									min="0"
									max="50000"
									step="1000"
									bind:value={minAltitude}
									onchange={handleAltitudeChange}
									class="w-full"
								/>
							</div>
							<div class="flex-1 space-y-1">
								<label for="max-altitude" class="text-xs text-gray-400">Max:</label>
								<input
									id="max-altitude"
									type="range"
									min="0"
									max="50000"
									step="1000"
									bind:value={maxAltitude}
									onchange={handleAltitudeChange}
									class="w-full"
								/>
							</div>
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Error message -->
		{#if error}
			<div class="mt-2 rounded bg-error-500 p-2 text-sm text-white">
				{error}
			</div>
		{/if}
	</div>

	<!-- Map container -->
	<div bind:this={mapContainer} class="flex-1"></div>

	<!-- Legend -->
	<div class="bg-surface-700 p-4">
		<div class="flex items-center gap-4">
			<span class="text-sm font-medium text-gray-300">Fix Count:</span>
			<div class="flex items-center gap-2">
				<div class="h-4 w-8 rounded" style="background-color: #440154;"></div>
				<span class="text-xs text-gray-400">Low</span>
			</div>
			<div class="flex items-center gap-2">
				<div class="h-4 w-8 rounded" style="background-color: #31688e;"></div>
				<span class="text-xs text-gray-400">Medium</span>
			</div>
			<div class="flex items-center gap-2">
				<div class="h-4 w-8 rounded" style="background-color: #35b779;"></div>
				<span class="text-xs text-gray-400">High</span>
			</div>
			<div class="flex items-center gap-2">
				<div class="h-4 w-8 rounded" style="background-color: #fde724;"></div>
				<span class="text-xs text-gray-400">Very High</span>
			</div>
			<div class="flex items-center gap-2">
				<div class="h-4 w-8 rounded" style="background-color: #ff0000;"></div>
				<span class="text-xs text-gray-400">Maximum</span>
			</div>
		</div>
	</div>
</div>
