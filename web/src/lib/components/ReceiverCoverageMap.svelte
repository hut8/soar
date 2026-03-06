<script lang="ts">
	import { onMount } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { MapboxOverlay } from '@deck.gl/mapbox';
	import { H3HexagonLayer } from '@deck.gl/geo-layers';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'ReceiverCoverageMap']);
	import { Loader, Calendar, Layers } from '@lucide/svelte';
	import type { CoverageHexProperties, CoverageGeoJsonResponse } from '$lib/types';
	import HexSamplesModal from '$lib/components/HexSamplesModal.svelte';

	let {
		receiverId,
		receiverLatitude,
		receiverLongitude,
		height = '500px'
	} = $props<{
		receiverId: string;
		receiverLatitude: number;
		receiverLongitude: number;
		height?: string;
	}>();

	let mapContainer: HTMLDivElement;
	let map: maplibregl.Map | null = null;
	let deckOverlay: MapboxOverlay | null = null;
	let loading = $state(false);
	let error = $state('');
	let resolution = $state(7);
	let hexCount = $state(0);
	let currentPopup: maplibregl.Popup | null = null;
	let autoResolution = $state(true);
	let moveDebounceTimer: number | null = null;
	let showHexModal = $state(false);
	let selectedHexProperties = $state<CoverageHexProperties | null>(null);

	// Default to last 30 days
	function getDefaultDates() {
		const now = Date.now();
		const thirtyDaysMs = 30 * 24 * 60 * 60 * 1000;
		const todayStr = new Date(now).toISOString().split('T')[0];
		const thirtyDaysAgoStr = new Date(now - thirtyDaysMs).toISOString().split('T')[0];
		return { start: thirtyDaysAgoStr, end: todayStr };
	}

	const defaultDates = getDefaultDates();
	let startDate = $state(defaultDates.start);
	let endDate = $state(defaultDates.end);

	// Calculate smart resolution based on zoom level
	function getSmartResolution(zoom: number): number {
		if (zoom >= 11) return 8; // ~0.7 km² - very close zoom
		if (zoom >= 9) return 7; // ~5 km² - close zoom
		if (zoom >= 7) return 6; // ~36 km² - medium zoom
		if (zoom >= 5) return 5; // ~252 km² - wider zoom
		if (zoom >= 3) return 4; // ~1,770 km² - regional zoom
		return 3; // ~12,400 km² - continental zoom
	}

	/** Interpolate color based on normalized value (0-1) */
	function getHexColor(normalized: number): [number, number, number, number] {
		// Gradient: light blue → light green → light yellow → light orange
		const stops: [number, [number, number, number]][] = [
			[0, [147, 197, 253]], // #93c5fd
			[0.33, [134, 239, 172]], // #86efac
			[0.66, [253, 224, 71]], // #fde047
			[1.0, [253, 186, 116]] // #fdba74
		];

		// Find the two stops to interpolate between
		let lower = stops[0];
		let upper = stops[stops.length - 1];
		for (let i = 0; i < stops.length - 1; i++) {
			if (normalized >= stops[i][0] && normalized <= stops[i + 1][0]) {
				lower = stops[i];
				upper = stops[i + 1];
				break;
			}
		}

		const range = upper[0] - lower[0];
		const t = range === 0 ? 0 : (normalized - lower[0]) / range;

		return [
			Math.round(lower[1][0] + (upper[1][0] - lower[1][0]) * t),
			Math.round(lower[1][1] + (upper[1][1] - lower[1][1]) * t),
			Math.round(lower[1][2] + (upper[1][2] - lower[1][2]) * t),
			180 // alpha
		];
	}

	async function loadCoverage() {
		if (!map || !deckOverlay) return;

		loading = true;
		error = '';

		try {
			const bounds = map.getBounds();
			const west = bounds.getWest();
			const east = bounds.getEast();
			const south = bounds.getSouth();
			const north = bounds.getNorth();

			// Use smart resolution based on zoom level if auto mode is enabled
			let selectedResolution = resolution;
			if (autoResolution) {
				selectedResolution = getSmartResolution(map.getZoom());
			}

			// Try to load coverage, reducing resolution if we hit the limit
			let currentResolution = selectedResolution;
			let response: CoverageGeoJsonResponse | null = null;
			let attempts = 0;
			const maxAttempts = 6;

			while (attempts < maxAttempts) {
				// Build query parameters
				const params = new URLSearchParams({
					resolution: currentResolution.toString(),
					west: west.toString(),
					east: east.toString(),
					south: south.toString(),
					north: north.toString(),
					start_date: startDate,
					end_date: endDate,
					limit: '5000',
					receiver_id: receiverId
				});

				response = await serverCall<CoverageGeoJsonResponse>(
					`/coverage/hexes?${params.toString()}`
				);

				const count = response.features?.length || 0;

				if (count >= 5000 && currentResolution > 3) {
					logger.debug(
						'Hit limit with {count} hexagons at resolution {resolution}, trying lower resolution',
						{ count, resolution: currentResolution }
					);
					currentResolution--;
					attempts++;
				} else {
					if (currentResolution !== selectedResolution && autoResolution) {
						logger.debug('Auto-adjusted from resolution {from} to {to} to stay under limit', {
							from: selectedResolution,
							to: currentResolution
						});
					}
					resolution = currentResolution;
					break;
				}
			}

			if (!response) {
				throw new Error('Failed to load coverage after multiple attempts');
			}

			hexCount = response.features?.length || 0;
			logger.debug('Loaded {count} coverage hexagons at resolution {resolution}', {
				count: hexCount,
				resolution
			});

			// Clear deck.gl layers if no data
			if (hexCount === 0) {
				deckOverlay.setProps({ layers: [] });

				// Clear any stale hover popup and cursor state
				if (currentPopup) {
					currentPopup.remove();
					currentPopup = null;
				}
				if (map) {
					map.getCanvas().style.cursor = '';
				}
				return;
			}

			// Extract hex data from GeoJSON features for deck.gl
			const hexData = response.features.map((f) => f.properties);

			// Calculate max fix count for color/elevation scaling (log scale)
			const maxFixCount = Math.max(...hexData.map((d) => d.fixCount), 1);
			const logMax = Math.log1p(maxFixCount);

			// Create deck.gl H3HexagonLayer
			const hexLayer = new H3HexagonLayer<CoverageHexProperties>({
				id: 'coverage-hexes',
				data: hexData,
				pickable: true,
				extruded: true,
				filled: true,
				getHexagon: (d: CoverageHexProperties) => d.h3Index,
				getFillColor: (d: CoverageHexProperties) => getHexColor(Math.log1p(d.fixCount) / logMax),
				getElevation: (d: CoverageHexProperties) => (Math.log1p(d.fixCount) / logMax) * 5000,
				elevationScale: 1,
				opacity: 0.8,
				onHover: (info: { object?: CoverageHexProperties; coordinate?: number[] }) => {
					if (!map) return;
					if (info.object && info.coordinate) {
						map.getCanvas().style.cursor = 'pointer';
						const props = info.object;

						if (currentPopup) {
							currentPopup.remove();
						}

						currentPopup = new maplibregl.Popup({
							closeButton: false,
							closeOnClick: false
						})
							.setLngLat([info.coordinate[0], info.coordinate[1]])
							.setHTML(
								`
								<div class="bg-surface-800 text-surface-50 p-2 rounded">
									<p class="font-semibold mb-1">Coverage Hex</p>
									<p class="text-sm mb-0.5">Fixes: ${props.fixCount.toLocaleString()}</p>
									<p class="text-sm mb-0.5">Coverage: ${props.coverageHours.toFixed(1)} hours</p>
									${props.avgAltitudeMslFeet ? `<p class="text-sm mb-0.5">Avg Altitude: ${props.avgAltitudeMslFeet.toLocaleString()} ft</p>` : ''}
									${props.minAltitudeMslFeet !== null && props.maxAltitudeMslFeet !== null ? `<p class="text-sm mb-0.5">Altitude Range: ${props.minAltitudeMslFeet.toLocaleString()}-${props.maxAltitudeMslFeet.toLocaleString()} ft</p>` : ''}
									<p class="text-sm text-surface-400">Resolution: ${props.resolution}</p>
								</div>
							`
							)
							.addTo(map);
					} else {
						if (currentPopup) {
							currentPopup.remove();
							currentPopup = null;
						}
						if (map) {
							map.getCanvas().style.cursor = '';
						}
					}
				},
				onClick: (info: { object?: CoverageHexProperties }) => {
					if (info.object) {
						if (currentPopup) {
							currentPopup.remove();
							currentPopup = null;
						}
						selectedHexProperties = info.object;
						showHexModal = true;
					}
				}
			});

			deckOverlay.setProps({ layers: [hexLayer] });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load coverage: ${errorMessage}`;
			logger.error('Coverage load error: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	function handleResolutionChange() {
		autoResolution = false;
		loadCoverage();
	}

	function handleAutoResolutionToggle() {
		if (autoResolution && map) {
			resolution = getSmartResolution(map.getZoom());
			loadCoverage();
		}
	}

	function handleDateChange() {
		loadCoverage();
	}

	onMount(() => {
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
			center: [receiverLongitude, receiverLatitude],
			zoom: 9
		});

		map.addControl(new maplibregl.NavigationControl(), 'top-right');

		deckOverlay = new MapboxOverlay({
			interleaved: false,
			layers: []
		});
		map.addControl(deckOverlay as unknown as maplibregl.IControl);

		map.on('load', () => {
			loadCoverage();
		});

		map.on('moveend', () => {
			if (moveDebounceTimer !== null) {
				clearTimeout(moveDebounceTimer);
			}

			moveDebounceTimer = window.setTimeout(() => {
				loadCoverage();
				moveDebounceTimer = null;
			}, 1000);
		});

		return () => {
			if (moveDebounceTimer !== null) {
				clearTimeout(moveDebounceTimer);
				moveDebounceTimer = null;
			}
			if (currentPopup) {
				currentPopup.remove();
				currentPopup = null;
			}
			if (deckOverlay) {
				deckOverlay.finalize();
				deckOverlay = null;
			}
			if (map) {
				map.remove();
				map = null;
			}
		};
	});
</script>

<div style="height: {height}" class="flex flex-col">
	<!-- Compact control bar -->
	<div class="flex flex-wrap items-center gap-3 rounded-t-lg bg-surface-700 px-3 py-2">
		<!-- Resolution selector -->
		<div class="flex items-center gap-1.5">
			<Layers class="h-4 w-4 text-gray-300" />
			<label for="rcm-resolution" class="text-xs font-medium text-gray-300">Res:</label>
			<select
				id="rcm-resolution"
				bind:value={resolution}
				onchange={handleResolutionChange}
				disabled={autoResolution}
				class="variant-filled-surface select w-28 text-xs"
				class:opacity-50={autoResolution}
			>
				<option value={3}>3 (~12,400 km²)</option>
				<option value={4}>4 (~1,770 km²)</option>
				<option value={5}>5 (~252 km²)</option>
				<option value={6}>6 (~36 km²)</option>
				<option value={7}>7 (~5 km²)</option>
				<option value={8}>8 (~0.7 km²)</option>
			</select>
			<label class="flex items-center gap-1 text-xs text-gray-300">
				<input
					type="checkbox"
					bind:checked={autoResolution}
					onchange={handleAutoResolutionToggle}
					class="checkbox"
				/>
				Auto
			</label>
		</div>

		<!-- Date range -->
		<div class="flex items-center gap-1.5">
			<Calendar class="h-4 w-4 text-gray-300" />
			<input
				type="date"
				bind:value={startDate}
				onchange={handleDateChange}
				class="variant-filled-surface input w-32 text-xs"
			/>
			<span class="text-xs text-gray-400">to</span>
			<input
				type="date"
				bind:value={endDate}
				onchange={handleDateChange}
				class="variant-filled-surface input w-32 text-xs"
			/>
		</div>

		<!-- Stats -->
		<div class="ml-auto flex items-center gap-1.5 text-xs text-gray-300">
			{#if loading}
				<Loader class="h-3.5 w-3.5 animate-spin" />
				<span>Loading...</span>
			{:else}
				<span class="font-semibold">{hexCount.toLocaleString()}</span>
				<span>hexagons</span>
			{/if}
		</div>
	</div>

	<!-- Map fills remaining space -->
	<div bind:this={mapContainer} class="flex-1"></div>

	<!-- Legend -->
	<div class="flex flex-wrap items-center gap-3 rounded-b-lg bg-surface-700 px-3 py-1.5">
		<span class="text-xs font-medium text-gray-300">Density:</span>
		<div class="flex items-center gap-1">
			<div class="h-2.5 w-5 rounded-sm" style="background-color: #93c5fd;"></div>
			<span class="text-xs text-gray-400">Low</span>
		</div>
		<div class="flex items-center gap-1">
			<div class="h-2.5 w-5 rounded-sm" style="background-color: #86efac;"></div>
			<span class="text-xs text-gray-400">Medium</span>
		</div>
		<div class="flex items-center gap-1">
			<div class="h-2.5 w-5 rounded-sm" style="background-color: #fde047;"></div>
			<span class="text-xs text-gray-400">High</span>
		</div>
		<div class="flex items-center gap-1">
			<div class="h-2.5 w-5 rounded-sm" style="background-color: #fdba74;"></div>
			<span class="text-xs text-gray-400">Very High</span>
		</div>
	</div>

	<!-- Error banner -->
	{#if error}
		<div class="mt-1 rounded bg-error-500 p-2 text-sm text-white">
			{error}
		</div>
	{/if}
</div>

<!-- Hex Details Modal -->
<HexSamplesModal
	bind:showModal={showHexModal}
	bind:hexProperties={selectedHexProperties}
	dateRange={{ start: startDate, end: endDate }}
	{receiverId}
/>

<style>
	/* Ensure MapLibre popups render above the deck.gl canvas */
	:global(.maplibregl-popup) {
		z-index: 3 !important;
	}
</style>
