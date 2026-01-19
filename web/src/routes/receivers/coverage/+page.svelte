<script lang="ts">
	import { onMount } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'ReceiverCoverage']);
	import { Loader, Calendar, Layers, Radio, Filter, ChevronDown, ChevronUp } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import type { CoverageHexProperties, CoverageGeoJsonResponse, Receiver } from '$lib/types';
	import HexSamplesModal from '$lib/components/HexSamplesModal.svelte';

	let mapContainer: HTMLDivElement;
	let map: maplibregl.Map | null = null;
	let loading = false;
	let error = '';
	let resolution = 7;
	let hexCount = $state(0);
	let receivers: Receiver[] = [];
	let receiverMarkers: maplibregl.Marker[] = [];
	let selectedReceiverId = $state('');
	let minAltitude = $state(0);
	let maxAltitude = $state(50000);
	let showAdvancedFilters = $state(false);
	let currentPopup: maplibregl.Popup | null = null;
	let autoResolution = $state(true); // Auto-select resolution based on zoom level
	let moveDebounceTimer: number | null = null;
	let receiverSearchQuery = $state('');
	let receiverSearchResults = $state<Receiver[]>([]);
	let searchDebounceTimer: number | null = null;
	let showReceiverDropdown = $state(false);
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

	async function loadReceivers() {
		if (!map) return;

		try {
			// Get current map bounds
			// Note: Do not normalize longitude - allow west > east for date line crossing
			const bounds = map.getBounds();
			const west = bounds.getWest();
			const east = bounds.getEast();
			const south = bounds.getSouth();
			const north = bounds.getNorth();

			// Build query parameters for bounding box search

			const params = new URLSearchParams({
				south: south.toString(),
				north: north.toString(),
				west: west.toString(),
				east: east.toString()
			});

			const response = await serverCall<{ data: Receiver[] }>(`/receivers?${params.toString()}`);
			receivers = response.data || [];
			logger.debug('Loaded {count} receivers in current view', { count: receivers.length });
		} catch (err) {
			logger.error('Failed to load receivers: {error}', { error: err });
		}
	}

	// Calculate smart resolution based on zoom level
	function getSmartResolution(zoom: number): number {
		if (zoom >= 11) return 8; // ~0.7 km² - very close zoom
		if (zoom >= 9) return 7; // ~5 km² - close zoom
		if (zoom >= 7) return 6; // ~36 km² - medium zoom
		if (zoom >= 5) return 5; // ~252 km² - wider zoom
		if (zoom >= 3) return 4; // ~1,770 km² - regional zoom
		return 3; // ~12,400 km² - continental zoom
	}

	function displayReceiversOnMap() {
		if (!map) return;

		// Store map reference to avoid null check issues
		const currentMap = map;

		// Clear existing receiver markers
		receiverMarkers.forEach((marker) => marker.remove());
		receiverMarkers = [];

		receivers.forEach((receiver) => {
			if (!receiver.latitude || !receiver.longitude) return;

			// Create marker content with Radio icon
			const markerDiv = document.createElement('div');
			markerDiv.className = 'receiver-marker';

			// Create icon container
			const iconDiv = document.createElement('div');
			iconDiv.className = 'receiver-icon';
			iconDiv.innerHTML = `
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/>
					<path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/>
					<circle cx="12" cy="12" r="2"/>
					<path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/>
					<path d="M19.1 4.9C23 8.8 23 15.2 19.1 19.1"/>
				</svg>
			`;

			// Create label
			const labelDiv = document.createElement('div');
			labelDiv.className = 'receiver-label';
			labelDiv.textContent = receiver.callsign;

			markerDiv.appendChild(iconDiv);
			markerDiv.appendChild(labelDiv);

			// Add click handler to navigate to receiver page
			markerDiv.onclick = () => {
				window.location.href = resolve(`/receivers/${receiver.id}`);
			};

			// Create MapLibre marker
			const marker = new maplibregl.Marker({ element: markerDiv })
				.setLngLat([receiver.longitude, receiver.latitude])
				.addTo(currentMap);

			receiverMarkers.push(marker);
		});
	}

	onMount(() => {
		// Add click-outside handler for receiver dropdown
		document.addEventListener('click', handleClickOutside);

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

		// Load coverage data and receivers when map is ready
		map.on('load', async () => {
			loadCoverage();
			await loadReceivers();
			displayReceiversOnMap();
		});

		// Reload coverage and receivers when user stops moving the map (with 1s debounce)
		map.on('moveend', () => {
			// Clear any existing timer
			if (moveDebounceTimer !== null) {
				clearTimeout(moveDebounceTimer);
			}

			// Set a new timer to reload after 1 second
			moveDebounceTimer = window.setTimeout(async () => {
				loadCoverage();
				await loadReceivers();
				displayReceiversOnMap();
				moveDebounceTimer = null;
			}, 1000);
		});

		return () => {
			document.removeEventListener('click', handleClickOutside);
			map?.remove();
		};
	});

	async function loadCoverage() {
		if (!map) return;

		loading = true;
		error = '';

		try {
			const bounds = map.getBounds();
			// Note: Do not normalize longitude - allow west > east for date line crossing
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
			const maxAttempts = 6; // We have 6 resolutions (3-8)

			while (attempts < maxAttempts) {
				// Build query parameters
				// eslint-disable-next-line svelte/prefer-svelte-reactivity -- URLSearchParams created fresh on each call, no persistent state
				const params = new URLSearchParams({
					resolution: currentResolution.toString(),
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

				response = await serverCall<CoverageGeoJsonResponse>(
					`/coverage/hexes?${params.toString()}`
				);

				const count = response.features?.length || 0;

				// If we hit the limit and we're not at the lowest resolution, try a lower resolution
				if (count >= 5000 && currentResolution > 3) {
					logger.debug(
						'Hit limit with {count} hexagons at resolution {resolution}, trying lower resolution',
						{ count, resolution: currentResolution }
					);
					currentResolution--;
					attempts++;
				} else {
					// Success! Update the resolution display if it changed
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

			// Remove existing coverage layer if it exists
			if (map.getLayer('coverage-hexes')) {
				map.removeLayer('coverage-hexes');
			}
			if (map.getSource('coverage')) {
				map.removeSource('coverage');
			}

			// Only add layer if we have features
			if (hexCount === 0) {
				return; // No data to display
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
						'#3b82f6', // Blue (low coverage)
						maxFixCount * 0.33,
						'#22c55e', // Green
						maxFixCount * 0.66,
						'#eab308', // Yellow
						maxFixCount,
						'#f97316' // Orange (high coverage)
					],
					'fill-opacity': 0.35,
					'fill-outline-color': 'rgba(0, 0, 0, 0.3)'
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

				// Remove previous popup if it exists
				if (currentPopup) {
					currentPopup.remove();
				}

				currentPopup = new maplibregl.Popup({
					closeButton: false,
					closeOnClick: false
				})
					.setLngLat(e.lngLat)
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
					.addTo(map!);
			});

			// Remove popup when mouse leaves the hexagon layer
			map.on('mouseleave', 'coverage-hexes', () => {
				if (currentPopup) {
					currentPopup.remove();
					currentPopup = null;
				}
				map!.getCanvas().style.cursor = '';
			});

			// Add click handler for hexagons to show detailed modal
			map.on('click', 'coverage-hexes', (e: maplibregl.MapLayerMouseEvent) => {
				if (!e.features || !e.features[0]) return;

				const feature = e.features[0] as maplibregl.MapGeoJSONFeature & {
					properties: CoverageHexProperties;
				};

				// Close hover popup when clicking
				if (currentPopup) {
					currentPopup.remove();
					currentPopup = null;
				}

				// Open modal with hex properties
				selectedHexProperties = feature.properties;
				showHexModal = true;
			});
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load coverage: ${errorMessage}`;
			logger.error('Coverage load error: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	function handleResolutionChange() {
		// Disable auto mode when manually selecting a resolution
		autoResolution = false;
		loadCoverage();
	}

	function handleAutoResolutionToggle() {
		if (autoResolution && map) {
			// Immediately apply smart resolution
			resolution = getSmartResolution(map.getZoom());
			loadCoverage();
		}
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

	async function searchReceivers(query: string) {
		if (query.trim().length === 0) {
			receiverSearchResults = [];
			return;
		}

		try {
			const params = new URLSearchParams({ query: query.trim() });
			const response = await serverCall<{ data: Receiver[] }>(`/receivers?${params.toString()}`);
			receiverSearchResults = response.data || [];
		} catch (err) {
			logger.error('Failed to search receivers: {error}', { error: err });
			receiverSearchResults = [];
		}
	}

	function handleReceiverSearchInput() {
		// Clear previous timer
		if (searchDebounceTimer !== null) {
			clearTimeout(searchDebounceTimer);
		}

		// Set a new timer to search after 300ms
		searchDebounceTimer = window.setTimeout(() => {
			searchReceivers(receiverSearchQuery);
			searchDebounceTimer = null;
		}, 300);
	}

	function selectReceiver(receiverId: string, callsign: string) {
		selectedReceiverId = receiverId;
		receiverSearchQuery = callsign;
		showReceiverDropdown = false;
		receiverSearchResults = [];
		handleReceiverChange();
	}

	function clearReceiverSelection() {
		selectedReceiverId = '';
		receiverSearchQuery = '';
		receiverSearchResults = [];
		showReceiverDropdown = false;
		handleReceiverChange();
	}

	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('#receiver-search') && !target.closest('.receiver-dropdown')) {
			showReceiverDropdown = false;
		}
	}
</script>

<div class="fixed inset-x-0 top-[42px] bottom-0 flex w-full flex-col">
	<!-- Header -->
	<div class="bg-surface-800 px-4 py-3 shadow-md">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-xl font-bold text-white">Receiver Coverage Map</h1>
				<p class="text-xs text-gray-300">
					Visualizing aircraft reception coverage using H3 hexagons
				</p>
			</div>
			<a href={resolve('/receivers')} class="btn gap-2 preset-outlined btn-sm">
				<Radio class="h-4 w-4" />
				View Receivers
			</a>
		</div>
	</div>

	<!-- Controls -->
	<div class="bg-surface-700 px-4 py-2 shadow-sm">
		<div class="flex flex-wrap gap-4">
			<!-- Resolution selector -->
			<div class="flex items-center gap-2">
				<Layers class="h-5 w-5 text-gray-300" />
				<label for="resolution" class="text-sm font-medium text-gray-300">Resolution:</label>
				<select
					id="resolution"
					bind:value={resolution}
					onchange={handleResolutionChange}
					disabled={autoResolution}
					class="variant-filled-surface select w-32"
					class:opacity-50={autoResolution}
				>
					<option value={3}>3 (~12,400 km²)</option>
					<option value={4}>4 (~1,770 km²)</option>
					<option value={5}>5 (~252 km²)</option>
					<option value={6}>6 (~36 km²)</option>
					<option value={7}>7 (~5 km²)</option>
					<option value={8}>8 (~0.7 km²)</option>
				</select>
				<label class="flex items-center gap-1.5 text-sm text-gray-300">
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
					<div class="relative space-y-2">
						<label for="receiver-search" class="text-sm font-medium text-gray-300">Receiver:</label>
						<div class="relative">
							<input
								id="receiver-search"
								type="text"
								placeholder={selectedReceiverId
									? receiverSearchQuery
									: 'All Receivers (type to search)'}
								bind:value={receiverSearchQuery}
								oninput={handleReceiverSearchInput}
								onfocus={() => {
									showReceiverDropdown = true;
									if (receiverSearchQuery.trim().length > 0) {
										searchReceivers(receiverSearchQuery);
									}
								}}
								class="variant-filled-surface input w-full pr-8"
							/>
							{#if selectedReceiverId}
								<button
									onclick={clearReceiverSelection}
									class="absolute top-1/2 right-2 -translate-y-1/2 text-gray-400 hover:text-white"
									title="Clear selection"
								>
									×
								</button>
							{/if}
						</div>

						{#if showReceiverDropdown && receiverSearchResults.length > 0}
							<div
								class="receiver-dropdown variant-filled-surface absolute z-50 mt-1 max-h-60 w-full overflow-auto rounded border border-surface-400 shadow-lg"
							>
								{#each receiverSearchResults as receiver (receiver.id)}
									<button
										onclick={() => selectReceiver(receiver.id, receiver.callsign)}
										class="w-full px-3 py-2 text-left text-sm hover:bg-surface-600"
									>
										<div class="font-medium">{receiver.callsign}</div>
										{#if receiver.description}
											<div class="text-xs text-gray-400">{receiver.description}</div>
										{/if}
									</button>
								{/each}
							</div>
						{/if}
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
	<div class="bg-surface-700 px-4 py-2">
		<div class="flex flex-wrap items-center gap-4">
			<span class="text-sm font-medium text-gray-300">Density:</span>
			<div class="flex items-center gap-1">
				<div class="h-3 w-6 rounded-sm" style="background-color: #3b82f6;"></div>
				<span class="text-xs text-gray-400">Low</span>
			</div>
			<div class="flex items-center gap-1">
				<div class="h-3 w-6 rounded-sm" style="background-color: #22c55e;"></div>
				<span class="text-xs text-gray-400">Medium</span>
			</div>
			<div class="flex items-center gap-1">
				<div class="h-3 w-6 rounded-sm" style="background-color: #eab308;"></div>
				<span class="text-xs text-gray-400">High</span>
			</div>
			<div class="flex items-center gap-1">
				<div class="h-3 w-6 rounded-sm" style="background-color: #f97316;"></div>
				<span class="text-xs text-gray-400">Very High</span>
			</div>
		</div>
	</div>
</div>

<!-- Hex Details Modal -->
<HexSamplesModal
	bind:showModal={showHexModal}
	bind:hexProperties={selectedHexProperties}
	dateRange={{ start: startDate, end: endDate }}
	receiverId={selectedReceiverId}
	altitudeFilter={minAltitude > 0 || maxAltitude < 50000
		? { min: minAltitude, max: maxAltitude }
		: undefined}
/>

<style>
	/* Receiver marker styling - matches operations page */
	:global(.receiver-marker) {
		display: flex;
		flex-direction: column;
		align-items: center;
		pointer-events: auto;
		cursor: pointer;
	}

	:global(.receiver-marker:hover .receiver-icon) {
		transform: scale(1.1);
	}

	:global(.receiver-icon) {
		background: transparent;
		border: 2px solid #374151;
		border-radius: 50%;
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: #374151;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
		transition:
			background 0.2s ease-in-out,
			border-color 0.2s ease-in-out,
			box-shadow 0.2s ease-in-out,
			transform 0.2s ease-in-out;
	}

	@media (prefers-color-scheme: dark) {
		:global(.receiver-icon) {
			background: transparent;
			border-color: #6b7280;
		}
	}

	:global(.receiver-marker:hover .receiver-icon) {
		background: white;
		border-color: #fb923c;
		box-shadow: 0 3px 8px rgba(251, 146, 60, 0.4);
	}

	@media (prefers-color-scheme: dark) {
		:global(.receiver-marker:hover .receiver-icon) {
			background: #1f2937;
		}
	}

	:global(.receiver-label) {
		background: rgba(255, 255, 255, 0.95);
		border: 1px solid #d1d5db;
		border-radius: 4px;
		padding: 2px 6px;
		font-size: 11px;
		font-weight: 500;
		color: #374151;
		margin-top: 4px;
		white-space: nowrap;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		opacity: 0;
		visibility: hidden;
		transition: all 0.2s ease-in-out;
		pointer-events: none;
	}

	@media (prefers-color-scheme: dark) {
		:global(.receiver-label) {
			background: rgba(31, 41, 55, 0.95);
			border-color: #4b5563;
			color: #e5e7eb;
		}
	}

	:global(.receiver-marker:hover .receiver-label) {
		opacity: 1;
		visibility: visible;
	}
</style>
