<script lang="ts">
	import { onMount, onDestroy, untrack } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { goto } from '$app/navigation';
	import { ArrowLeft, ChevronDown, ChevronUp, Palette } from '@lucide/svelte';
	import type { PageData } from './$types';
	import type {
		Receiver,
		PathPoint,
		DataListResponse,
		DataResponse,
		Flight,
		Fix
	} from '$lib/types';
	import dayjs from 'dayjs';
	import { serverCall } from '$lib/api/server';
	import FlightProfile from '$lib/components/FlightProfile.svelte';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'FlightMap']);

	let { data }: { data: PageData } = $props();

	let mapContainer = $state<HTMLElement>();
	let map = $state<maplibregl.Map>();
	let altitudePopup = $state<maplibregl.Popup | null>(null);
	let pollingInterval: ReturnType<typeof setInterval> | null = null;
	let hoverMarker = $state<maplibregl.Marker | null>(null);

	// Display options
	let isPanelCollapsed = $state(false);
	let chartRecreateTrigger = $state(0);
	let wasPanelCollapsed = $state(false);

	// Color scheme selection
	type ColorScheme = 'altitude' | 'time';
	let colorScheme = $state<ColorScheme>('altitude');

	// Receiver data
	let showReceivers = $state(false);
	let receivers = $state<Receiver[]>([]);
	let receiverMarkers = $state<maplibregl.Marker[]>([]);
	let isLoadingReceivers = $state(false);

	// Check if fixes have AGL data
	const hasAglData = $derived(data.fixes.some((f) => f.altitudeAglFeet !== null));

	// Calculate maximum altitude from path data (used for trail coloring)
	const maxAltitude = $derived(() => {
		if (data.path.length === 0) return null;
		const maxMsl = Math.max(...data.path.map((p) => p.altitudeFeet || 0));
		return maxMsl > 0 ? maxMsl : null;
	});

	// Calculate minimum altitude from path data
	const minAltitude = $derived(() => {
		if (data.path.length === 0) return null;
		const validAltitudes = data.path
			.map((p) => p.altitudeFeet)
			.filter((alt): alt is number => alt !== null && alt !== undefined);
		if (validAltitudes.length === 0) return null;
		return Math.min(...validAltitudes);
	});

	// Helper function to calculate bearing between two points
	function calculateBearing(lat1: number, lon1: number, lat2: number, lon2: number): number {
		const φ1 = (lat1 * Math.PI) / 180;
		const φ2 = (lat2 * Math.PI) / 180;
		const Δλ = ((lon2 - lon1) * Math.PI) / 180;

		const y = Math.sin(Δλ) * Math.cos(φ2);
		const x = Math.cos(φ1) * Math.sin(φ2) - Math.sin(φ1) * Math.cos(φ2) * Math.cos(Δλ);
		const θ = Math.atan2(y, x);
		const bearing = ((θ * 180) / Math.PI + 360) % 360;
		return bearing;
	}

	// Helper function to map altitude to color (red→blue gradient)
	function altitudeToColor(altitude: number | null | undefined, min: number, max: number): string {
		if (altitude === null || altitude === undefined || max === min) {
			return '#888888'; // Gray for unknown altitude
		}

		// Normalize altitude to 0-1 range
		const normalized = (altitude - min) / (max - min);

		// Interpolate from red (low) to blue (high)
		const r = Math.round(239 - normalized * (239 - 59));
		const g = Math.round(68 + normalized * (130 - 68));
		const b = Math.round(68 + normalized * (246 - 68));

		return `rgb(${r}, ${g}, ${b})`;
	}

	// Helper function to map time to color (purple→orange gradient)
	// Earlier fixes are purple, later fixes are orange
	function timeToColor(fixIndex: number, totalFixes: number): string {
		if (totalFixes <= 1) {
			return '#888888'; // Gray for single fix
		}

		// Normalize index to 0-1 range
		const normalized = fixIndex / (totalFixes - 1);

		// Interpolate from purple (early) to orange (late)
		const r = Math.round(147 + normalized * (251 - 147));
		const g = Math.round(51 + normalized * (146 - 51));
		const b = Math.round(234 - normalized * (234 - 60));

		return `rgb(${r}, ${g}, ${b})`;
	}

	// Get color for a fix based on current color scheme
	function getFixColor(
		fixIndex: number,
		altitude: number | null | undefined,
		minAlt: number,
		maxAlt: number,
		totalFixes: number
	): string {
		if (colorScheme === 'altitude') {
			return altitudeToColor(altitude, minAlt, maxAlt);
		} else {
			return timeToColor(fixIndex, totalFixes);
		}
	}

	// Build GeoJSON for flight track segments (each segment is a 2-point LineString with a color)
	function buildTrackGeoJSON(pathPoints: PathPoint[]): GeoJSON.FeatureCollection {
		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;
		const totalPoints = pathPoints.length;
		const features: GeoJSON.Feature[] = [];

		for (let i = 0; i < pathPoints.length - 1; i++) {
			const p1 = pathPoints[i];
			const p2 = pathPoints[i + 1];
			const color = getFixColor(i, p1.altitudeFeet, minAlt, maxAlt, totalPoints);

			features.push({
				type: 'Feature',
				properties: { color },
				geometry: {
					type: 'LineString',
					coordinates: [
						[p1.longitude, p1.latitude],
						[p2.longitude, p2.latitude]
					]
				}
			});
		}

		return { type: 'FeatureCollection', features };
	}

	// Build GeoJSON for clickable arrow markers from fixes (for info popups)
	function buildFixArrowGeoJSON(fixesInOrder: Fix[]): GeoJSON.FeatureCollection {
		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;
		const totalFixes = fixesInOrder.length;
		const features: GeoJSON.Feature[] = [];

		if (totalFixes === 0) return { type: 'FeatureCollection', features };

		// Place arrows at ~10% intervals
		const arrowInterval = Math.max(1, Math.floor(totalFixes / 10));

		for (let i = 0; i < totalFixes; i++) {
			if (i % arrowInterval !== 0 && i !== 0) continue;

			const fix = fixesInOrder[i];
			let bearing = 0;

			if (i < totalFixes - 1) {
				const nextFix = fixesInOrder[i + 1];
				bearing = calculateBearing(
					fix.latitude,
					fix.longitude,
					nextFix.latitude,
					nextFix.longitude
				);
			} else if (i > 0) {
				const prevFix = fixesInOrder[i - 1];
				bearing = calculateBearing(
					prevFix.latitude,
					prevFix.longitude,
					fix.latitude,
					fix.longitude
				);
			}

			const color = getFixColor(i, fix.altitudeMslFeet, minAlt, maxAlt, totalFixes);

			features.push({
				type: 'Feature',
				properties: {
					bearing,
					color,
					fixIndex: i,
					altitudeMsl: fix.altitudeMslFeet ? Math.round(fix.altitudeMslFeet) : null,
					altitudeAgl: fix.altitudeAglFeet ? Math.round(fix.altitudeAglFeet) : null,
					heading: fix.trackDegrees !== null ? Math.round(fix.trackDegrees) + '°' : null,
					turnRate: fix.turnRateRot !== null ? fix.turnRateRot.toFixed(2) + ' rot/min' : null,
					climbRate: fix.climbFpm !== null ? Math.round(fix.climbFpm) + ' fpm' : null,
					groundSpeed:
						fix.groundSpeedKnots !== null ? Math.round(fix.groundSpeedKnots) + ' kt' : null,
					timestamp: dayjs(fix.receivedAt).format('h:mm:ss A')
				},
				geometry: {
					type: 'Point',
					coordinates: [fix.longitude, fix.latitude]
				}
			});
		}

		return { type: 'FeatureCollection', features };
	}

	// Add flight track layers to the map
	function addFlightLayers() {
		if (!map) return;

		const trackGeoJSON = buildTrackGeoJSON(data.path);
		const fixesInOrder = [...data.fixes].reverse();
		const arrowGeoJSON = buildFixArrowGeoJSON(fixesInOrder);

		// Add track source and layer
		if (map.getSource('flight-track')) {
			(map.getSource('flight-track') as maplibregl.GeoJSONSource).setData(trackGeoJSON);
		} else {
			map.addSource('flight-track', { type: 'geojson', data: trackGeoJSON });
			map.addLayer({
				id: 'flight-track-line',
				type: 'line',
				source: 'flight-track',
				paint: {
					'line-color': ['get', 'color'],
					'line-width': 3
				}
			});
		}

		// Add arrow source and layer
		if (map.getSource('flight-arrows')) {
			(map.getSource('flight-arrows') as maplibregl.GeoJSONSource).setData(arrowGeoJSON);
		} else {
			map.addSource('flight-arrows', { type: 'geojson', data: arrowGeoJSON });
			map.addLayer({
				id: 'flight-arrows-layer',
				type: 'symbol',
				source: 'flight-arrows',
				layout: {
					'icon-image': 'arrow-icon',
					'icon-size': 0.7,
					'icon-rotate': ['get', 'bearing'],
					'icon-allow-overlap': true,
					'icon-rotation-alignment': 'map'
				},
				paint: {
					'icon-color': ['get', 'color'],
					'icon-halo-color': 'rgba(0, 0, 0, 0.6)',
					'icon-halo-width': 1
				}
			});
		}
	}

	// Create arrow icon for the map
	function createArrowIcon(mapInstance: maplibregl.Map) {
		const size = 32;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;
		const ctx = canvas.getContext('2d')!;

		// Draw arrow pointing up (rotation handled by MapLibre)
		ctx.fillStyle = '#ffffff';
		ctx.strokeStyle = 'rgba(0,0,0,0.3)';
		ctx.lineWidth = 1;
		ctx.beginPath();
		ctx.moveTo(size / 2, 2); // top
		ctx.lineTo(size - 4, size - 4); // bottom right
		ctx.lineTo(size / 2, size - 8); // notch
		ctx.lineTo(4, size - 4); // bottom left
		ctx.closePath();
		ctx.fill();
		ctx.stroke();

		const imageData = ctx.getImageData(0, 0, size, size);
		mapInstance.addImage('arrow-icon', imageData, { sdf: true });
	}

	function isFlightInProgress(): boolean {
		return data.flight.state === 'active';
	}

	// Poll for updates to in-progress flights
	async function pollForUpdates() {
		try {
			// Get the timestamp of the most recent fix (fixes are in DESC order, so first element is newest)
			const latestFixTimestamp = data.fixes.length > 0 ? data.fixes[0].receivedAt : null;

			// Build URL with 'after' parameter if we have fixes
			const fixesUrl = latestFixTimestamp
				? `/flights/${data.flight.id}/fixes?after=${encodeURIComponent(latestFixTimestamp)}`
				: `/flights/${data.flight.id}/fixes`;

			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<DataResponse<Flight>>(`/flights/${data.flight.id}`),
				serverCall<DataListResponse<Fix>>(fixesUrl)
			]);

			data.flight = flightResponse.data;

			// Append new fixes to the existing list (new fixes are in DESC order)
			if (fixesResponse.data.length > 0) {
				data.fixes = [...fixesResponse.data, ...data.fixes];
				data.fixesCount = data.fixes.length;
			}

			if (data.flight.state !== 'active') {
				stopPolling();
			}

			// Only update map if we got new fixes (chart will update automatically via reactivity)
			if (fixesResponse.data.length > 0) {
				updateMap();
			}
		} catch (err) {
			logger.error('Failed to poll for flight updates: {error}', { error: err });
		}
	}

	// Start polling for in-progress flights
	function startPolling() {
		if (isFlightInProgress() && !pollingInterval) {
			pollingInterval = setInterval(pollForUpdates, 10000);
		}
	}

	// Stop polling
	function stopPolling() {
		if (pollingInterval) {
			clearInterval(pollingInterval);
			pollingInterval = null;
		}
	}

	// Update map with new data
	function updateMap() {
		if (data.path.length === 0 || !map || !map.isStyleLoaded()) return;

		// Update track and arrow layers with new data
		addFlightLayers();

		// Update takeoff/landing markers
		updateEndpointMarkers();
	}

	// Track takeoff/landing markers for cleanup
	let takeoffMarker: maplibregl.Marker | null = null;
	let landingMarker: maplibregl.Marker | null = null;

	function updateEndpointMarkers() {
		// Remove existing markers
		takeoffMarker?.remove();
		landingMarker?.remove();
		takeoffMarker = null;
		landingMarker = null;

		if (!map || data.path.length === 0) return;

		// Add takeoff marker (green)
		const first = data.path[0];
		const takeoffEl = document.createElement('div');
		takeoffEl.style.cssText =
			'background-color: #10b981; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;';
		takeoffMarker = new maplibregl.Marker({ element: takeoffEl })
			.setLngLat([first.longitude, first.latitude])
			.addTo(map);

		// Add landing marker (red) if flight is complete
		if (data.flight.landingTime && data.path.length > 0) {
			const last = data.path[data.path.length - 1];
			const landingEl = document.createElement('div');
			landingEl.style.cssText =
				'background-color: #ef4444; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;';
			landingMarker = new maplibregl.Marker({ element: landingEl })
				.setLngLat([last.longitude, last.latitude])
				.addTo(map);
		}
	}

	// Initialize map
	onMount(async () => {
		if (data.path.length === 0 || !mapContainer) return;

		try {
			// Calculate bounds from path data
			const bounds = new maplibregl.LngLatBounds();
			data.path.forEach((point) => {
				bounds.extend([point.longitude, point.latitude]);
			});

			// Create map with satellite view (ESRI)
			map = new maplibregl.Map({
				container: mapContainer,
				style: {
					version: 8,
					sources: {
						esri: {
							type: 'raster',
							tiles: [
								'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'
							],
							tileSize: 256,
							attribution:
								'Tiles &copy; Esri &mdash; Source: Esri, i-cubed, USDA, USGS, AEX, GeoEye, Getmapping, Aerogrid, IGN, IGP, UPR-EGP, and the GIS User Community',
							maxzoom: 19
						}
					},
					layers: [
						{
							id: 'esri-satellite',
							type: 'raster',
							source: 'esri',
							minzoom: 0,
							maxzoom: 19
						}
					]
				},
				bounds: bounds,
				fitBoundsOptions: { padding: 50 }
			});

			map.addControl(new maplibregl.NavigationControl(), 'top-right');

			// Wait for map to load before adding layers
			map.on('load', () => {
				if (!map) return;

				// Create arrow icon
				createArrowIcon(map);

				// Add flight track and arrow layers
				addFlightLayers();

				// Add takeoff/landing markers
				updateEndpointMarkers();

				// Setup click handler for arrow markers
				map.on('click', 'flight-arrows-layer', (e) => {
					if (!map || !e.features || e.features.length === 0) return;

					const feature = e.features[0];
					const props = feature.properties;
					const coordinates = (feature.geometry as GeoJSON.Point).coordinates.slice() as [
						number,
						number
					];

					const content = `
						<div style="padding: 12px; min-width: 200px; background: white; color: #1f2937; border-radius: 8px; font-family: system-ui, -apple-system, sans-serif;">
							<div style="font-weight: 600; margin-bottom: 8px; font-size: 14px; color: #111827; border-bottom: 1px solid #e5e7eb; padding-bottom: 6px;">${props.timestamp}</div>
							<div style="display: flex; flex-direction: column; gap: 6px; font-size: 13px;">
								<div style="display: flex; justify-content: space-between;">
									<span style="color: #6b7280;">MSL:</span>
									<span style="font-weight: 600; color: #3b82f6;">${props.altitudeMsl ?? 'N/A'} ft</span>
								</div>
								<div style="display: flex; justify-content: space-between;">
									<span style="color: #6b7280;">AGL:</span>
									<span style="font-weight: 600; color: #10b981;">${props.altitudeAgl ?? 'N/A'} ft</span>
								</div>
								<div style="display: flex; justify-content: space-between;">
									<span style="color: #6b7280;">Heading:</span>
									<span style="font-weight: 500; color: #111827;">${props.heading ?? 'N/A'}</span>
								</div>
								<div style="display: flex; justify-content: space-between;">
									<span style="color: #6b7280;">Turn Rate:</span>
									<span style="font-weight: 500; color: #111827;">${props.turnRate ?? 'N/A'}</span>
								</div>
								<div style="display: flex; justify-content: space-between;">
									<span style="color: #6b7280;">Climb:</span>
									<span style="font-weight: 500; color: #111827;">${props.climbRate ?? 'N/A'}</span>
								</div>
								<div style="display: flex; justify-content: space-between;">
									<span style="color: #6b7280;">Speed:</span>
									<span style="font-weight: 500; color: #111827;">${props.groundSpeed ?? 'N/A'}</span>
								</div>
							</div>
						</div>
					`;

					// Close existing popup
					altitudePopup?.remove();
					altitudePopup = new maplibregl.Popup({ closeOnClick: true, maxWidth: '300px' })
						.setLngLat(coordinates)
						.setHTML(content)
						.addTo(map!);
				});

				// Change cursor on hover over arrows
				map.on('mouseenter', 'flight-arrows-layer', () => {
					if (map) map.getCanvas().style.cursor = 'pointer';
				});
				map.on('mouseleave', 'flight-arrows-layer', () => {
					if (map) map.getCanvas().style.cursor = '';
				});
			});
		} catch (error) {
			logger.error('Failed to initialize map: {error}', { error });
		}

		// Start polling if flight is in progress
		startPolling();
	});

	// Update map when color scheme changes
	$effect(() => {
		// Track colorScheme changes
		void colorScheme;

		// Untrack the rest to avoid infinite loop when updateMap modifies state
		untrack(() => {
			if (map && map.isStyleLoaded()) {
				updateMap();
			}
		});
	});

	// Recreate chart when panel transitions from collapsed to expanded
	$effect(() => {
		// Detect transition: was collapsed, now expanded
		if (wasPanelCollapsed && !isPanelCollapsed) {
			// Trigger chart recreation when panel expands
			chartRecreateTrigger++;
		}
		// Update previous state for next check
		wasPanelCollapsed = isPanelCollapsed;
	});

	// Callbacks for chart hover interaction with map
	function handleChartHover(fix: (typeof data.fixes)[0]) {
		if (!map) return;

		if (!hoverMarker) {
			const el = document.createElement('div');
			el.style.cssText =
				'background-color: #f97316; width: 16px; height: 16px; border-radius: 50%; border: 3px solid white; box-shadow: 0 2px 6px rgba(0,0,0,0.3);';
			hoverMarker = new maplibregl.Marker({ element: el })
				.setLngLat([fix.longitude, fix.latitude])
				.addTo(map);
		} else {
			hoverMarker.setLngLat([fix.longitude, fix.latitude]).addTo(map);
		}
	}

	function handleChartUnhover() {
		hoverMarker?.remove();
	}

	function handleChartClick(fix: (typeof data.fixes)[0]) {
		if (!map) return;

		if (!hoverMarker) {
			const el = document.createElement('div');
			el.style.cssText =
				'background-color: #f97316; width: 16px; height: 16px; border-radius: 50%; border: 3px solid white; box-shadow: 0 2px 6px rgba(0,0,0,0.3);';
			hoverMarker = new maplibregl.Marker({ element: el })
				.setLngLat([fix.longitude, fix.latitude])
				.addTo(map);
		} else {
			hoverMarker.setLngLat([fix.longitude, fix.latitude]).addTo(map);
		}
	}

	// Cleanup
	onDestroy(() => {
		stopPolling();
		map?.remove();
	});

	function goBack() {
		goto(`/flights/${data.flight.id}`);
	}

	function togglePanel() {
		isPanelCollapsed = !isPanelCollapsed;
	}

	// Handle receivers toggle
	function handleReceiversToggle() {
		if (showReceivers) {
			fetchReceivers();
		} else {
			// Clear receivers from map
			receiverMarkers.forEach((marker) => marker.remove());
			receiverMarkers = [];
			receivers = [];
		}
	}

	// Fetch receivers in viewport
	async function fetchReceivers() {
		if (!map) return;

		isLoadingReceivers = true;
		try {
			const bounds = map.getBounds();
			const ne = bounds.getNorthEast();
			const sw = bounds.getSouthWest();

			const params = new URLSearchParams({
				north: ne.lat.toString(),
				south: sw.lat.toString(),
				east: ne.lng.toString(),
				west: sw.lng.toString()
			});

			const response = await serverCall<DataListResponse<Receiver>>(`/receivers?${params}`);
			receivers = response.data.filter((receiver: Receiver) => {
				if (!receiver) return false;
				if (typeof receiver.latitude !== 'number' || typeof receiver.longitude !== 'number')
					return false;
				return true;
			});

			// Display receivers on map
			if (map) {
				// Clear existing receiver markers
				receiverMarkers.forEach((marker) => marker.remove());
				receiverMarkers = [];

				receivers.forEach((receiver) => {
					if (!receiver.latitude || !receiver.longitude) return;

					// Create marker content with Radio icon and link
					const markerLink = document.createElement('a');
					markerLink.href = `/receivers/${receiver.id}`;
					markerLink.target = '_blank';
					markerLink.rel = 'noopener noreferrer';
					markerLink.className = 'receiver-marker';

					const iconDiv = document.createElement('div');
					iconDiv.className = 'receiver-icon';
					iconDiv.innerHTML = `
						<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/>
							<path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/>
							<circle cx="12" cy="12" r="2"/>
							<path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/>
							<path d="M19.1 4.9C23 8.8 23 15.1 19.1 19"/>
						</svg>
					`;

					const labelDiv = document.createElement('div');
					labelDiv.className = 'receiver-label';
					labelDiv.textContent = receiver.callsign;

					markerLink.appendChild(iconDiv);
					markerLink.appendChild(labelDiv);

					const marker = new maplibregl.Marker({ element: markerLink })
						.setLngLat([receiver.longitude, receiver.latitude])
						.addTo(map!);

					receiverMarkers.push(marker);
				});
			}
		} catch (err) {
			logger.error('Failed to fetch receivers: {error}', { error: err });
		} finally {
			isLoadingReceivers = false;
		}
	}
</script>

<!-- Container that fills viewport - using fixed positioning to break out of main container -->
<div class="fixed inset-x-0 top-[42px] bottom-0 w-full overflow-hidden">
	<!-- Map container -->
	<div
		bind:this={mapContainer}
		class="absolute top-0 right-0 left-0"
		class:bottom-[48px]={isPanelCollapsed}
		class:bottom-[300px]={!isPanelCollapsed}
	></div>

	<!-- Back button (top-left) -->
	<button onclick={goBack} class="location-btn absolute top-4 left-4 z-[80]" title="Back to Flight">
		<ArrowLeft size={20} />
	</button>

	<!-- Bottom panel with altitude chart -->
	<div
		class="bg-surface-50-900-token absolute right-0 bottom-0 left-0 z-[80] shadow-lg transition-all duration-300"
		style={isPanelCollapsed ? 'height: 48px;' : 'height: 300px;'}
	>
		<!-- Panel header -->
		<div class="flex items-center justify-between gap-3 px-4 py-2">
			<div class="flex items-center gap-3">
				<button onclick={togglePanel} class="toggle-btn">
					{#if isPanelCollapsed}
						<ChevronUp class="h-4 w-4" />
					{:else}
						<ChevronDown class="h-4 w-4" />
					{/if}
				</button>
				<h3 class="font-semibold">Flight Profile</h3>
			</div>
			<div class="flex items-center gap-4">
				<label class="flex cursor-pointer items-center gap-2">
					<input
						type="checkbox"
						class="checkbox"
						bind:checked={showReceivers}
						onchange={handleReceiversToggle}
					/>
					<span class="text-sm">Show Receivers</span>
					{#if isLoadingReceivers}
						<span class="text-surface-600-300-token text-xs">(Loading...)</span>
					{/if}
				</label>
				<Palette class="text-surface-600-300-token h-4 w-4" />
				<span class="text-surface-600-300-token text-sm">Color:</span>
				<div class="inline-flex rounded-md shadow-sm" role="group">
					<button
						type="button"
						onclick={() => (colorScheme = 'altitude')}
						class="btn btn-sm {colorScheme === 'altitude'
							? 'preset-filled-primary-500'
							: 'preset-tonal'} rounded-r-none"
					>
						Altitude
					</button>
					<button
						type="button"
						onclick={() => (colorScheme = 'time')}
						class="btn btn-sm {colorScheme === 'time'
							? 'preset-filled-primary-500'
							: 'preset-tonal'} rounded-l-none"
					>
						Time
					</button>
				</div>
				<span class="text-surface-600-300-token text-xs">
					{#if colorScheme === 'altitude'}
						(Red→Blue)
					{:else}
						(Purple→Orange)
					{/if}
				</span>
			</div>
		</div>

		<!-- Panel content -->
		{#if !isPanelCollapsed}
			<div class="h-[252px] w-full">
				<FlightProfile
					fixes={data.fixes}
					{hasAglData}
					onHover={handleChartHover}
					onUnhover={handleChartUnhover}
					onClick={handleChartClick}
					bind:recreateTrigger={chartRecreateTrigger}
				/>
			</div>
		{/if}
	</div>
</div>

<style>
	/* Button styling to match operations page buttons */
	.location-btn,
	.toggle-btn {
		background: white;
		color: #374151; /* Gray-700 for good contrast against white */
		border: none;
		border-radius: 0.375rem;
		padding: 0.75rem;
		cursor: pointer;
		transition: all 200ms;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.location-btn:hover,
	.toggle-btn:hover {
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
		transform: translateY(-1px);
	}

	.location-btn:focus,
	.toggle-btn:focus {
		outline: none;
		box-shadow:
			0 0 0 2px rgba(59, 130, 246, 0.5),
			0 2px 8px rgba(0, 0, 0, 0.15);
	}

	/* Receiver marker styling */
	:global(.receiver-marker) {
		display: flex;
		flex-direction: column;
		align-items: center;
		pointer-events: auto;
		cursor: pointer;
		text-decoration: none;
		transition: transform 0.2s ease-in-out;
	}

	:global(.receiver-marker:hover) {
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
		color: #fb923c;
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
		transition: all 0.2s ease-in-out;
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
		font-weight: 600;
		color: #374151;
		margin-top: 2px;
		white-space: nowrap;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
		text-rendering: optimizeLegibility;
		-webkit-font-smoothing: antialiased;
		-moz-osx-font-smoothing: grayscale;
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 0.2s ease-in-out,
			visibility 0.2s ease-in-out;
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
