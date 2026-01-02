<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount, onDestroy, untrack } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import { goto } from '$app/navigation';
	import { ArrowLeft, ChevronDown, ChevronUp, Palette } from '@lucide/svelte';
	import type { PageData } from './$types';
	import type { Receiver, DataListResponse } from '$lib/types';
	import dayjs from 'dayjs';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { serverCall } from '$lib/api/server';
	import FlightProfile from '$lib/components/FlightProfile.svelte';

	let { data }: { data: PageData } = $props();

	let mapContainer = $state<HTMLElement>();
	let map = $state<google.maps.Map>();
	let flightPathSegments = $state<google.maps.Polyline[]>([]);
	let altitudeInfoWindow = $state<google.maps.InfoWindow | null>(null);
	let fixMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let pollingInterval: ReturnType<typeof setInterval> | null = null;
	let hoverMarker = $state<google.maps.marker.AdvancedMarkerElement | null>(null);

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
	let receiverMarkers = $state<google.maps.marker.AdvancedMarkerElement[]>([]);
	let isLoadingReceivers = $state(false);

	// Check if fixes have AGL data
	const hasAglData = $derived(data.fixes.some((f) => f.altitudeAglFeet !== null));

	// Calculate maximum altitude from fixes
	const maxAltitude = $derived(() => {
		if (data.fixes.length === 0) return null;
		const maxMsl = Math.max(...data.fixes.map((f) => f.altitudeMslFeet || 0));
		return maxMsl > 0 ? maxMsl : null;
	});

	// Calculate minimum altitude from fixes
	const minAltitude = $derived(() => {
		if (data.fixes.length === 0) return null;
		const validAltitudes = data.fixes
			.map((f) => f.altitudeMslFeet)
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

	// Helper function to determine which fixes should display arrow markers
	// Returns a Set of indices for fixes that should show arrows
	function getArrowFixIndices(fixesInOrder: typeof data.fixes): SvelteSet<number> {
		const indices = new SvelteSet<number>();

		if (fixesInOrder.length === 0) return indices;

		// Always include the first fix
		indices.add(0);

		if (fixesInOrder.length === 1) return indices;

		// Calculate 10% intervals
		const totalFixes = fixesInOrder.length;
		const interval = Math.floor(totalFixes / 10); // 10% of total fixes

		if (interval === 0) return indices; // Too few fixes for intervals

		let lastArrowTimestamp = new Date(fixesInOrder[0].timestamp).getTime();
		const oneMinuteMs = 60 * 1000; // 1 minute in milliseconds

		// Check each 10% marker
		for (let i = 1; i <= 10; i++) {
			const candidateIndex = Math.min(i * interval, totalFixes - 1);
			const candidateTimestamp = new Date(fixesInOrder[candidateIndex].timestamp).getTime();

			// Only add if at least 1 minute has passed since last arrow
			if (candidateTimestamp - lastArrowTimestamp >= oneMinuteMs) {
				indices.add(candidateIndex);
				lastArrowTimestamp = candidateTimestamp;
			}
		}

		return indices;
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
		// Purple: rgb(147, 51, 234) - #9333ea
		// Orange: rgb(251, 146, 60) - #fb923c
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

	// Calculate arrow scale based on zoom level (inversely proportional to zoom)
	// Zoom 8 (far out) -> scale 1, Zoom 16 (close in) -> scale 4
	function getArrowScale(zoom: number): number {
		// Scale increases linearly with zoom: at zoom 8 = 1, at zoom 16 = 4
		return Math.max(1, Math.min(5, (zoom - 8) * 0.5 + 1));
	}

	// Update arrow icons on all polyline segments based on current zoom
	function updateArrowScales() {
		if (!map) return;
		const zoom = map.getZoom() ?? 12;
		const scale = getArrowScale(zoom);
		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;

		// Track last arrow timestamp to display one every 10 minutes
		let lastArrowTime: Date | null = null;
		const TEN_MINUTES_MS = 10 * 60 * 1000;

		const fixes = [...data.fixes].reverse();
		const totalFixes = fixes.length;

		flightPathSegments.forEach((segment, index) => {
			const path = segment.getPath();
			if (path.getLength() < 2) return;

			// Get color based on segment index
			if (index >= fixes.length) return;
			const fix = fixes[index];
			const color = getFixColor(index, fix.altitudeMslFeet, minAlt, maxAlt, totalFixes);

			// Check if we should display an arrow for this segment
			const fixTime = new Date(fix.timestamp);
			const shouldShowArrow =
				lastArrowTime === null || fixTime.getTime() - lastArrowTime.getTime() >= TEN_MINUTES_MS;

			// Only add arrow icon if this segment should have one
			const icons = [];
			if (shouldShowArrow) {
				const arrowSymbol = {
					path: google.maps.SymbolPath.FORWARD_CLOSED_ARROW,
					fillColor: color,
					fillOpacity: 1,
					strokeColor: color,
					strokeOpacity: 0.8,
					strokeWeight: 1,
					scale: scale
				};
				icons.push({
					icon: arrowSymbol,
					offset: '50%'
				});
				lastArrowTime = fixTime;
			}

			segment.setOptions({
				icons: icons
			});
		});
	}

	// Helper function to create gradient polyline segments
	function createGradientPolylines(
		fixesInOrder: typeof data.fixes,
		targetMap: google.maps.Map
	): google.maps.Polyline[] {
		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;
		const segments: google.maps.Polyline[] = [];
		const zoom = targetMap.getZoom() ?? 12;
		const scale = getArrowScale(zoom);
		const totalFixes = fixesInOrder.length;

		// Track last arrow timestamp to display one every 10 minutes
		let lastArrowTime: Date | null = null;
		const TEN_MINUTES_MS = 10 * 60 * 1000;

		for (let i = 0; i < fixesInOrder.length - 1; i++) {
			const fix1 = fixesInOrder[i];
			const fix2 = fixesInOrder[i + 1];

			const color = getFixColor(i, fix1.altitudeMslFeet, minAlt, maxAlt, totalFixes);

			// Check if we should display an arrow for this segment
			const fix1Time = new Date(fix1.timestamp);
			const shouldShowArrow =
				lastArrowTime === null || fix1Time.getTime() - lastArrowTime.getTime() >= TEN_MINUTES_MS;

			// Define arrow symbol for this segment if needed
			const icons = [];
			if (shouldShowArrow) {
				const arrowSymbol = {
					path: google.maps.SymbolPath.FORWARD_CLOSED_ARROW,
					fillColor: color,
					fillOpacity: 1,
					strokeColor: color,
					strokeOpacity: 0.8,
					strokeWeight: 1,
					scale: scale
				};
				icons.push({
					icon: arrowSymbol,
					offset: '50%'
				});
				lastArrowTime = fix1Time;
			}

			const segment = new google.maps.Polyline({
				path: [
					{ lat: fix1.latitude, lng: fix1.longitude },
					{ lat: fix2.latitude, lng: fix2.longitude }
				],
				geodesic: true,
				strokeColor: color,
				strokeOpacity: 1.0,
				strokeWeight: 3,
				icons: icons
			});

			segment.setMap(targetMap);
			segments.push(segment);
		}

		return segments;
	}

	function isFlightInProgress(): boolean {
		return data.flight.state === 'active';
	}

	// Poll for updates to in-progress flights
	async function pollForUpdates() {
		try {
			// Get the timestamp of the most recent fix (fixes are in DESC order, so first element is newest)
			const latestFixTimestamp = data.fixes.length > 0 ? data.fixes[0].timestamp : null;

			// Build URL with 'after' parameter if we have fixes
			const fixesUrl = latestFixTimestamp
				? `/flights/${data.flight.id}/fixes?after=${encodeURIComponent(latestFixTimestamp)}`
				: `/flights/${data.flight.id}/fixes`;

			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<{
					flight: typeof data.flight;
				}>(`/flights/${data.flight.id}`),
				serverCall<{
					fixes: typeof data.fixes;
					count: number;
				}>(fixesUrl)
			]);

			data.flight = flightResponse.flight;
			// Device doesn't change during a flight, so we don't re-fetch it

			// Append new fixes to the existing list (new fixes are in DESC order)
			if (fixesResponse.fixes.length > 0) {
				data.fixes = [...fixesResponse.fixes, ...data.fixes];
				data.fixesCount = data.fixes.length;
			}

			if (data.flight.state !== 'active') {
				stopPolling();
			}

			// Only update map if we got new fixes (chart will update automatically via reactivity)
			if (fixesResponse.fixes.length > 0) {
				updateMap();
			}
		} catch (err) {
			console.error('Failed to poll for flight updates:', err);
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
		if (data.fixes.length === 0 || !map || flightPathSegments.length === 0) return;

		const fixesInOrder = [...data.fixes].reverse();

		// Clear existing flight path segments
		flightPathSegments.forEach((segment) => {
			segment.setMap(null);
		});
		flightPathSegments = [];

		// Create new gradient polyline segments
		flightPathSegments = createGradientPolylines(fixesInOrder, map);

		// Clear existing fix markers
		fixMarkers.forEach((marker) => {
			marker.map = null;
		});
		fixMarkers = [];

		// Wait for map to be ready before adding markers
		google.maps.event.addListenerOnce(map, 'idle', () => {
			addFixMarkers(fixesInOrder);
		});
	}

	function addFixMarkers(fixesInOrder: typeof data.fixes) {
		if (!map) return;

		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;
		const arrowIndices = getArrowFixIndices(fixesInOrder);
		const totalFixes = fixesInOrder.length;

		fixesInOrder.forEach((fix, index) => {
			// Only create arrow markers for selected indices
			if (!arrowIndices.has(index)) return;

			// Calculate bearing to next fix
			let bearing = 0;
			if (index < fixesInOrder.length - 1) {
				const nextFix = fixesInOrder[index + 1];
				bearing = calculateBearing(
					fix.latitude,
					fix.longitude,
					nextFix.latitude,
					nextFix.longitude
				);
			} else if (index > 0) {
				const prevFix = fixesInOrder[index - 1];
				bearing = calculateBearing(
					prevFix.latitude,
					prevFix.longitude,
					fix.latitude,
					fix.longitude
				);
			}

			const color = getFixColor(index, fix.altitudeMslFeet, minAlt, maxAlt, totalFixes);

			// Create SVG arrow element (12x12 pixels, twice the original size)
			const arrowSvg = document.createElement('div');
			arrowSvg.innerHTML = `
				<svg width="12" height="12" viewBox="0 0 16 16" style="transform: rotate(${bearing}deg); filter: drop-shadow(0 0 1px rgba(0,0,0,0.5)); cursor: pointer;">
					<path d="M8 2 L14 14 L8 11 L2 14 Z" fill="${color}" stroke="rgba(0,0,0,0.3)" stroke-width="0.4"/>
				</svg>
			`;

			const marker = new google.maps.marker.AdvancedMarkerElement({
				map,
				position: { lat: fix.latitude, lng: fix.longitude },
				content: arrowSvg
			});

			marker.addListener('click', () => {
				const mslAlt = fix.altitudeMslFeet ? Math.round(fix.altitudeMslFeet) : 'N/A';
				const aglAlt = fix.altitudeAglFeet ? Math.round(fix.altitudeAglFeet) : 'N/A';
				const heading = fix.trackDegrees !== null ? Math.round(fix.trackDegrees) + '°' : 'N/A';
				const turnRate = fix.turnRateRot !== null ? fix.turnRateRot.toFixed(2) + ' rot/min' : 'N/A';
				const climbRate = fix.climbFpm !== null ? Math.round(fix.climbFpm) + ' fpm' : 'N/A';
				const groundSpeed =
					fix.groundSpeedKnots !== null ? Math.round(fix.groundSpeedKnots) + ' kt' : 'N/A';
				const timestamp = dayjs(fix.timestamp).format('h:mm:ss A');

				const content = `
					<div style="padding: 12px; min-width: 200px; background: white; color: #1f2937; border-radius: 8px; font-family: system-ui, -apple-system, sans-serif;">
						<div style="font-weight: 600; margin-bottom: 8px; font-size: 14px; color: #111827; border-bottom: 1px solid #e5e7eb; padding-bottom: 6px;">${timestamp}</div>
						<div style="display: flex; flex-direction: column; gap: 6px; font-size: 13px;">
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">MSL:</span>
								<span style="font-weight: 600; color: #3b82f6;">${mslAlt} ft</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">AGL:</span>
								<span style="font-weight: 600; color: #10b981;">${aglAlt} ft</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Heading:</span>
								<span style="font-weight: 500; color: #111827;">${heading}</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Turn Rate:</span>
								<span style="font-weight: 500; color: #111827;">${turnRate}</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Climb:</span>
								<span style="font-weight: 500; color: #111827;">${climbRate}</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Speed:</span>
								<span style="font-weight: 500; color: #111827;">${groundSpeed}</span>
							</div>
						</div>
					</div>
				`;

				altitudeInfoWindow?.setContent(content);
				altitudeInfoWindow?.setPosition({ lat: fix.latitude, lng: fix.longitude });
				altitudeInfoWindow?.open(map);
			});

			fixMarkers.push(marker);
		});

		// Add landing marker if flight is complete
		if (data.flight.landingTime && fixesInOrder.length > 0) {
			const last = fixesInOrder[fixesInOrder.length - 1];
			const landingPin = document.createElement('div');
			landingPin.innerHTML = `
				<div style="background-color: #ef4444; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;"></div>
			`;

			new google.maps.marker.AdvancedMarkerElement({
				map,
				position: { lat: last.latitude, lng: last.longitude },
				content: landingPin,
				title: 'Landing'
			});
		}
	}

	// Initialize map
	onMount(async () => {
		if (data.fixes.length === 0 || !mapContainer) return;

		try {
			setOptions({
				key: GOOGLE_MAPS_API_KEY,
				v: 'weekly'
			});

			await importLibrary('maps');
			await importLibrary('marker');

			const fixesInOrder = [...data.fixes].reverse();

			// Calculate center and bounds
			const bounds = new google.maps.LatLngBounds();
			fixesInOrder.forEach((fix) => {
				bounds.extend({ lat: fix.latitude, lng: fix.longitude });
			});

			const center = bounds.getCenter();

			// Create map with satellite view
			map = new google.maps.Map(mapContainer, {
				center: { lat: center.lat(), lng: center.lng() },
				zoom: 12,
				mapId: 'FLIGHT_MAP',
				mapTypeId: google.maps.MapTypeId.SATELLITE,
				mapTypeControl: true,
				mapTypeControlOptions: {
					position: google.maps.ControlPosition.RIGHT_TOP,
					style: google.maps.MapTypeControlStyle.DEFAULT
				},
				streetViewControl: false,
				fullscreenControl: false,
				zoomControl: false
			});

			// Fit bounds
			map.fitBounds(bounds);

			// Create gradient polyline segments
			flightPathSegments = createGradientPolylines(fixesInOrder, map);

			// Add zoom listener to update arrow scales
			map.addListener('zoom_changed', () => {
				updateArrowScales();
			});

			// Create info window
			altitudeInfoWindow = new google.maps.InfoWindow();

			// Wait for map to be ready before adding markers
			google.maps.event.addListenerOnce(map, 'idle', () => {
				// Add takeoff marker (green)
				if (fixesInOrder.length > 0) {
					const first = fixesInOrder[0];
					const takeoffPin = document.createElement('div');
					takeoffPin.innerHTML = `
						<div style="background-color: #10b981; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;"></div>
					`;

					new google.maps.marker.AdvancedMarkerElement({
						map,
						position: { lat: first.latitude, lng: first.longitude },
						content: takeoffPin,
						title: 'Takeoff'
					});
				}

				// Add landing marker if flight is complete
				if (data.flight.landingTime && fixesInOrder.length > 0) {
					const last = fixesInOrder[fixesInOrder.length - 1];
					const landingPin = document.createElement('div');
					landingPin.innerHTML = `
						<div style="background-color: #ef4444; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;"></div>
					`;

					new google.maps.marker.AdvancedMarkerElement({
						map,
						position: { lat: last.latitude, lng: last.longitude },
						content: landingPin,
						title: 'Landing'
					});
				}

				// Add directional arrow markers at selected intervals
				addFixMarkers(fixesInOrder);
			});
		} catch (error) {
			console.error('Failed to load Google Maps:', error);
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
			if (map && flightPathSegments.length > 0) {
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

		// Create or update hover marker
		if (!hoverMarker) {
			const markerContent = document.createElement('div');
			markerContent.innerHTML = `
				<div style="background-color: #f97316; width: 16px; height: 16px; border-radius: 50%; border: 3px solid white; box-shadow: 0 2px 6px rgba(0,0,0,0.3);"></div>
			`;

			hoverMarker = new google.maps.marker.AdvancedMarkerElement({
				map,
				position: { lat: fix.latitude, lng: fix.longitude },
				content: markerContent,
				zIndex: 1000
			});
		} else {
			// Update position of existing marker
			hoverMarker.position = { lat: fix.latitude, lng: fix.longitude };
			hoverMarker.map = map;
		}
	}

	function handleChartUnhover() {
		// Hide hover marker
		if (hoverMarker) {
			hoverMarker.map = null;
		}
	}

	function handleChartClick(fix: (typeof data.fixes)[0]) {
		if (!map) return;

		// Create or update hover marker (reuse the same marker for clicks)
		if (!hoverMarker) {
			const markerContent = document.createElement('div');
			markerContent.innerHTML = `
				<div style="background-color: #f97316; width: 16px; height: 16px; border-radius: 50%; border: 3px solid white; box-shadow: 0 2px 6px rgba(0,0,0,0.3);"></div>
			`;

			hoverMarker = new google.maps.marker.AdvancedMarkerElement({
				map,
				position: { lat: fix.latitude, lng: fix.longitude },
				content: markerContent,
				zIndex: 1000
			});
		} else {
			// Update position of existing marker
			hoverMarker.position = { lat: fix.latitude, lng: fix.longitude };
			hoverMarker.map = map;
		}
	}

	// Cleanup
	onDestroy(() => {
		stopPolling();
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
			receiverMarkers.forEach((marker) => {
				marker.map = null;
			});
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
			if (!bounds) return;

			const ne = bounds.getNorthEast();
			const sw = bounds.getSouthWest();

			const params = new URLSearchParams({
				north: ne.lat().toString(),
				south: sw.lat().toString(),
				east: ne.lng().toString(),
				west: sw.lng().toString()
			});

			const response = await serverCall<DataListResponse<Receiver>>(`/receivers?${params}`);
			receivers = response.data.filter((receiver: Receiver) => {
				// Basic validation
				if (!receiver) {
					console.error('Invalid receiver: null or undefined', receiver);
					return false;
				}

				// Validate latitude and longitude are numbers
				if (typeof receiver.latitude !== 'number' || typeof receiver.longitude !== 'number') {
					console.error('Invalid receiver: latitude or longitude is not a number', receiver);
					return false;
				}

				return true;
			});

			// Display receivers on map
			if (map) {
				// Clear existing receiver markers
				receiverMarkers.forEach((marker) => {
					marker.map = null;
				});
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

					const marker = new google.maps.marker.AdvancedMarkerElement({
						position: { lat: receiver.latitude, lng: receiver.longitude },
						map: map,
						title: `${receiver.callsign}${receiver.description ? ` - ${receiver.description}` : ''}`,
						content: markerLink,
						zIndex: 150
					});

					receiverMarkers.push(marker);
				});
			}
		} catch (err) {
			console.error('Failed to fetch receivers:', err);
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
