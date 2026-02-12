<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import {
		Download,
		Plane,
		PlaneTakeoff,
		PlaneLanding,
		Gauge,
		TrendingUp,
		Route,
		MoveUpRight,
		MapPinMinus,
		ChevronsLeft,
		ChevronLeft,
		ChevronRight,
		ChevronsRight,
		ChevronDown,
		ChevronUp,
		Info,
		ExternalLink,
		MountainSnow,
		Clock,
		Expand,
		LocateFixed,
		Palette,
		Globe
	} from '@lucide/svelte';
	import type { PageData } from './$types';
	import type { Flight, Receiver, DataResponse, DataListResponse } from '$lib/types';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import durationPlugin from 'dayjs/plugin/duration';
	import {
		getAircraftCategoryDescription,
		formatAircraftAddress,
		getAircraftCategoryColor,
		getFlagPath,
		getCountryName
	} from '$lib/formatters';
	import { serverCall } from '$lib/api/server';
	import FlightStateBadge from '$lib/components/FlightStateBadge.svelte';
	import RadarLoader from '$lib/components/RadarLoader.svelte';
	import FixesList from '$lib/components/FixesList.svelte';
	import FlightsList from '$lib/components/FlightsList.svelte';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';
	import FlightProfile from '$lib/components/FlightProfile.svelte';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'FlightDetail']);

	dayjs.extend(relativeTime);
	dayjs.extend(durationPlugin);

	let { data }: { data: PageData } = $props();

	// Progressive loading state - await promises from page load
	import type { Aircraft, Fix } from '$lib/types';
	let aircraft = $state<Aircraft | undefined>(undefined);
	let fixes = $state<Fix[]>([]);
	let isLoadingAircraft = $state(true);
	let isLoadingFixes = $state(true);

	// Aircraft images state
	interface AircraftImage {
		source: 'airport_data' | 'planespotters';
		pageUrl: string;
		thumbnailUrl: string;
		imageUrl?: string;
		photographer?: string;
	}
	interface AircraftImageCollection {
		images: AircraftImage[];
		lastFetched: Record<string, string>;
	}
	let aircraftImages = $state<AircraftImage[]>([]);
	let isLoadingImages = $state(false);

	// Load aircraft and fixes progressively
	$effect(() => {
		isLoadingAircraft = true;
		data.aircraftPromise.then((result) => {
			aircraft = result;
			isLoadingAircraft = false;
			// Load images after aircraft is loaded
			if (result?.id) {
				loadAircraftImages(result.id);
			}
		});

		isLoadingFixes = true;
		data.fixesPromise.then((result) => {
			fixes = result;
			isLoadingFixes = false;
		});
	});

	interface FlightGap {
		gapStart: string;
		gapEnd: string;
		durationSeconds: number;
		distanceMeters: number;
		callsignBefore: string | null;
		callsignAfter: string | null;
		squawkBefore: string | null;
		squawkAfter: string | null;
		climbRateBefore: number | null;
		climbRateAfter: number | null;
		avgClimbRate10Before: number | null;
		avgClimbRate10After: number | null;
	}

	let mapContainer = $state<HTMLElement>();
	let map = $state<maplibregl.Map>();
	let altitudePopup = $state<maplibregl.Popup | null>(null);
	let pollingInterval: ReturnType<typeof setInterval> | null = null;
	let hoverMarker = $state<maplibregl.Marker | null>(null);

	// Track takeoff/landing markers
	let takeoffMarker: maplibregl.Marker | null = null;
	let landingMarker: maplibregl.Marker | null = null;

	// Track user interaction with map
	let hasUserInteracted = $state(false);
	let isAutomatedZoom = false;

	// Pagination state
	let currentPage = $state(1);
	let pageSize = 25;

	// Display options
	let includeNearbyFlights = $state(false);
	let showReceivers = $state(false);

	// Color scheme selection
	type ColorScheme = 'altitude' | 'time';
	let colorScheme = $state<ColorScheme>('altitude');

	// Nearby flights data - shared between map and section
	let nearbyFlights = $state<Flight[]>([]);
	let nearbyFlightsFixes = $state<Map<string, Fix[]>>(new Map());
	let isLoadingNearbyFlights = $state(false);
	let showNearbyFlightsSection = $state(false);
	// Counter for unique nearby flight layer IDs
	let nearbyFlightLayerIds = $state<{ layerId: string; sourceId: string }[]>([]);

	// Receiver data
	let receivers = $state<Receiver[]>([]);
	let receiverMarkers = $state<maplibregl.Marker[]>([]);
	let isLoadingReceivers = $state(false);

	// Fix gaps data
	let flightGaps = $state<FlightGap[]>([]);
	let isLoadingGaps = $state(false);
	let showGaps = $state(false);

	// Fixes list collapse state (mobile only)
	let isFixesCollapsed = $state(true);

	// Reverse fixes to show chronologically (earliest first, landing last)
	const reversedFixes = $derived(fixes.length > 0 ? [...fixes].reverse() : []);
	const totalPages = $derived(Math.ceil(reversedFixes.length / pageSize));
	const paginatedFixes = $derived(
		reversedFixes.slice((currentPage - 1) * pageSize, currentPage * pageSize)
	);

	// Calculate flight duration
	const duration = $derived.by(() => {
		if (!data.flight.takeoffTime || !data.flight.landingTime) {
			return null;
		}
		const start = new Date(data.flight.takeoffTime);
		const end = new Date(data.flight.landingTime);
		const diffMs = end.getTime() - start.getTime();
		const hours = Math.floor(diffMs / (1000 * 60 * 60));
		const minutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	});

	// Calculate fixes per second rate
	const fixesPerSecond = $derived.by(() => {
		if (!data.flight.takeoffTime || !data.flight.landingTime || fixes.length === 0) {
			return null;
		}
		const start = new Date(data.flight.takeoffTime);
		const end = new Date(data.flight.landingTime);
		const durationSeconds = (end.getTime() - start.getTime()) / 1000;
		if (durationSeconds <= 0) return null;
		return (fixes.length / durationSeconds).toFixed(2);
	});

	// Calculate maximum altitude from fixes
	const maxAltitude = $derived.by(() => {
		if (fixes.length === 0) return null;
		const maxMsl = Math.max(...fixes.map((f) => f.altitudeMslFeet || 0));
		return maxMsl > 0 ? maxMsl : null;
	});

	// Calculate minimum altitude from fixes
	const minAltitude = $derived.by(() => {
		if (fixes.length === 0) return null;
		const validAltitudes = fixes
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

	// Build GeoJSON for flight track segments
	function buildTrackGeoJSON(fixesInOrder: Fix[]): GeoJSON.FeatureCollection {
		const minAlt = minAltitude ?? 0;
		const maxAlt = maxAltitude ?? 1000;
		const totalFixes = fixesInOrder.length;
		const features: GeoJSON.Feature[] = [];

		for (let i = 0; i < fixesInOrder.length - 1; i++) {
			const fix1 = fixesInOrder[i];
			const fix2 = fixesInOrder[i + 1];
			const color = getFixColor(i, fix1.altitudeMslFeet, minAlt, maxAlt, totalFixes);

			features.push({
				type: 'Feature',
				properties: { color },
				geometry: {
					type: 'LineString',
					coordinates: [
						[fix1.longitude, fix1.latitude],
						[fix2.longitude, fix2.latitude]
					]
				}
			});
		}

		return { type: 'FeatureCollection', features };
	}

	// Build GeoJSON for clickable arrow markers from fixes (for info popups)
	function buildFixArrowGeoJSON(fixesInOrder: Fix[]): GeoJSON.FeatureCollection {
		const minAlt = minAltitude ?? 0;
		const maxAlt = maxAltitude ?? 1000;
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
		ctx.moveTo(size / 2, 2);
		ctx.lineTo(size - 4, size - 4);
		ctx.lineTo(size / 2, size - 8);
		ctx.lineTo(4, size - 4);
		ctx.closePath();
		ctx.fill();
		ctx.stroke();

		const imageData = ctx.getImageData(0, 0, size, size);
		mapInstance.addImage('arrow-icon', imageData, { sdf: true });
	}

	// Add flight track layers to the map
	function addFlightLayers() {
		if (!map) return;

		const fixesInOrder = [...fixes].reverse();
		const trackGeoJSON = buildTrackGeoJSON(fixesInOrder);
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

	// Update endpoint markers (takeoff/landing)
	function updateEndpointMarkers() {
		takeoffMarker?.remove();
		landingMarker?.remove();
		takeoffMarker = null;
		landingMarker = null;

		if (!map || fixes.length === 0) return;

		const fixesInOrder = [...fixes].reverse();

		// Add takeoff marker (green)
		if (fixesInOrder.length > 0) {
			const first = fixesInOrder[0];
			const takeoffEl = document.createElement('div');
			takeoffEl.style.cssText =
				'background-color: #10b981; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;';
			takeoffMarker = new maplibregl.Marker({ element: takeoffEl })
				.setLngLat([first.longitude, first.latitude])
				.addTo(map);
		}

		// Add landing marker (red) if flight is complete
		if (data.flight.landingTime && fixesInOrder.length > 0) {
			const last = fixesInOrder[fixesInOrder.length - 1];
			const landingEl = document.createElement('div');
			landingEl.style.cssText =
				'background-color: #ef4444; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;';
			landingMarker = new maplibregl.Marker({ element: landingEl })
				.setLngLat([last.longitude, last.latitude])
				.addTo(map);
		}
	}

	// Calculate maximum AGL altitude from fixes
	const maxAglAltitude = $derived.by(() => {
		if (fixes.length === 0) return null;
		const maxAgl = Math.max(...fixes.map((f) => f.altitudeAglFeet || 0));
		return maxAgl > 0 ? maxAgl : null;
	});

	// Check if this is an outlanding (flight complete with known departure but no arrival airport)
	const isOutlanding = $derived(
		data.flight.landingTime !== null &&
			data.flight.landingTime !== undefined &&
			data.flight.departureAirport &&
			!data.flight.arrivalAirport
	);

	// Check if any fix has AGL data available
	const hasAglData = $derived(
		fixes.some(
			(fix) =>
				fix.altitudeAglFeet !== null && fix.altitudeAglFeet !== undefined && fix.altitudeAglFeet > 0
		)
	);

	// Format date/time with relative time and full datetime
	function formatDateTime(dateString: string | undefined): string {
		if (!dateString) return 'N/A';
		const date = dayjs(dateString);
		const relative = date.fromNow();
		const fullDate = date.format('MMM D, YYYY h:mm A');
		return `${relative} (${fullDate})`;
	}

	// Format date/time - mobile only shows relative
	function formatDateTimeMobile(dateString: string | undefined): string {
		if (!dateString) return 'N/A';
		return dayjs(dateString).fromNow();
	}

	// Format altitude
	function formatAltitude(feet: number | undefined): string {
		if (feet === undefined || feet === null) return 'N/A';
		return `${Math.round(feet)} ft`;
	}

	// Format distance in meters to nautical miles and kilometers
	function formatDistance(meters: number | undefined): string {
		if (meters === undefined || meters === null) return 'N/A';
		const nm = meters / 1852;
		const km = meters / 1000;

		if (nm >= 1) {
			return `${nm.toFixed(2)} nm (${km.toFixed(2)} km)`;
		} else {
			return `${km.toFixed(2)} km`;
		}
	}

	// Check if flight is in progress
	function isFlightInProgress(): boolean {
		return data.flight.state === 'active';
	}

	// Handle nearby flights toggle
	function handleNearbyFlightsToggle() {
		if (includeNearbyFlights) {
			fetchNearbyFlights();
			showNearbyFlightsSection = true;
		} else {
			// Clear nearby flights from map
			clearNearbyFlightLayers();
		}
	}

	// Clear nearby flight layers from map
	function clearNearbyFlightLayers() {
		if (!map) return;
		for (const { layerId, sourceId } of nearbyFlightLayerIds) {
			if (map.getLayer(layerId)) map.removeLayer(layerId);
			if (map.getSource(sourceId)) map.removeSource(sourceId);
		}
		nearbyFlightLayerIds = [];
	}

	// Handle receivers toggle
	function handleReceiversToggle() {
		if (showReceivers) {
			fetchReceivers();
		} else {
			receiverMarkers.forEach((marker) => marker.remove());
			receiverMarkers = [];
			receivers = [];
		}
	}

	// Show nearby flights section and fetch data if needed
	async function showNearbyFlights() {
		showNearbyFlightsSection = true;

		if (nearbyFlights.length > 0) {
			return;
		}

		isLoadingNearbyFlights = true;
		try {
			const response = await serverCall<DataListResponse<Flight>>(
				`/flights/${data.flight.id}/nearby`
			);
			nearbyFlights = response.data;
		} catch (err) {
			logger.error('Failed to fetch nearby flights: {error}', { error: err });
		} finally {
			isLoadingNearbyFlights = false;
		}
	}

	// Update nearby flight paths on map
	function updateNearbyFlightPaths() {
		if (!map || nearbyFlightsFixes.size === 0) return;

		// Clear existing
		clearNearbyFlightLayers();

		const colors = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4'];

		let colorIdx = 0;
		for (const [flightId, flightFixes] of nearbyFlightsFixes) {
			if (flightFixes.length === 0) continue;

			const fixesInOrder = [...flightFixes].reverse();
			const coordinates = fixesInOrder.map((f) => [f.longitude, f.latitude]);
			const sourceId = `nearby-flight-${flightId}`;
			const layerId = `nearby-flight-line-${flightId}`;

			map.addSource(sourceId, {
				type: 'geojson',
				data: {
					type: 'Feature',
					properties: {},
					geometry: {
						type: 'LineString',
						coordinates
					}
				}
			});

			map.addLayer({
				id: layerId,
				type: 'line',
				source: sourceId,
				paint: {
					'line-color': colors[colorIdx % colors.length],
					'line-width': 2,
					'line-opacity': 0.6
				}
			});

			nearbyFlightLayerIds.push({ layerId, sourceId });
			colorIdx++;
		}
	}

	// Fetch nearby flights and their fixes
	async function fetchNearbyFlights() {
		if (nearbyFlights.length > 0 && map) {
			if (nearbyFlightsFixes.size > 0) {
				updateNearbyFlightPaths();
				return;
			}
		}

		isLoadingNearbyFlights = true;
		try {
			if (nearbyFlights.length === 0) {
				const response = await serverCall<DataListResponse<Flight>>(
					`/flights/${data.flight.id}/nearby`
				);
				nearbyFlights = response.data;
			}

			if (map) {
				const fixesPromises = nearbyFlights.map((nearbyFlight) =>
					serverCall<DataListResponse<Fix>>(`/flights/${nearbyFlight.id}/fixes`)
						.then((response) => ({ flightId: nearbyFlight.id, fixes: response.data }))
						.catch((err) => {
							logger.error('Failed to fetch fixes for nearby flight {id}: {error}', {
								id: nearbyFlight.id,
								error: err
							});
							return null;
						})
				);

				const allFixesResponses = await Promise.all(fixesPromises);

				nearbyFlightsFixes.clear();
				allFixesResponses.forEach((response) => {
					if (response && response.fixes.length > 0) {
						nearbyFlightsFixes.set(response.flightId, response.fixes);
					}
				});

				updateNearbyFlightPaths();
			}
		} catch (err) {
			logger.error('Failed to fetch nearby flights: {error}', { error: err });
		} finally {
			isLoadingNearbyFlights = false;
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

			if (map) {
				receiverMarkers.forEach((marker) => marker.remove());
				receiverMarkers = [];

				receivers.forEach((receiver) => {
					if (!receiver.latitude || !receiver.longitude) return;

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

	// Load aircraft images
	async function loadAircraftImages(aircraftId: string) {
		isLoadingImages = true;
		try {
			const response = await serverCall<DataResponse<AircraftImageCollection>>(
				`/aircraft/${aircraftId}/images`
			);
			aircraftImages = response.data.images || [];
		} catch (err) {
			logger.error('Failed to load aircraft images: {error}', { error: err });
			aircraftImages = [];
		} finally {
			isLoadingImages = false;
		}
	}

	// Poll for updates to in-progress flights
	async function pollForUpdates() {
		try {
			const latestFixTimestamp = fixes.length > 0 ? fixes[0].receivedAt : null;

			const fixesUrl = latestFixTimestamp
				? `/flights/${data.flight.id}/fixes?after=${encodeURIComponent(latestFixTimestamp)}`
				: `/flights/${data.flight.id}/fixes`;

			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<DataResponse<Flight>>(`/flights/${data.flight.id}`),
				serverCall<DataListResponse<Fix>>(fixesUrl)
			]);

			data.flight = flightResponse.data;

			if (fixesResponse.data.length > 0) {
				fixes = [...fixesResponse.data, ...fixes];
			}

			if (data.flight.state !== 'active') {
				stopPolling();
			}

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
		if (fixes.length === 0 || !map || !map.isStyleLoaded()) return;

		addFlightLayers();
		updateEndpointMarkers();

		// Update bounds to show all fixes (only if user hasn't manually interacted)
		if (!hasUserInteracted) {
			fitMapToBounds();
		}
	}

	// Initialize map when fixes are loaded
	$effect(() => {
		if (fixes.length === 0 || !mapContainer || isLoadingFixes) return;

		untrack(() => {
			try {
				const fixesInOrder = [...fixes].reverse();

				// Calculate bounds
				const bounds = new maplibregl.LngLatBounds();
				fixesInOrder.forEach((fix) => {
					bounds.extend([fix.longitude, fix.latitude]);
				});

				// Create map with satellite view (ESRI)
				if (!mapContainer) return;
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

				// Track user interaction
				map.on('dragend', () => {
					hasUserInteracted = true;
				});

				map.on('zoomend', () => {
					if (!isAutomatedZoom) {
						hasUserInteracted = true;
					}
					isAutomatedZoom = false;
				});

				// Debounced bounds change for nearby flights
				let boundsChangedTimeout: ReturnType<typeof setTimeout> | null = null;
				map.on('moveend', () => {
					if (boundsChangedTimeout) {
						clearTimeout(boundsChangedTimeout);
					}
					boundsChangedTimeout = setTimeout(() => {
						if (includeNearbyFlights && nearbyFlightsFixes.size > 0) {
							updateNearbyFlightPaths();
						}
					}, 300);
				});

				// Wait for map to load before adding layers
				map.on('load', () => {
					if (!map) return;

					createArrowIcon(map);
					addFlightLayers();
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
		}); // end untrack
	}); // end $effect

	// Callbacks for chart hover interaction with map
	function handleChartHover(fix: (typeof fixes)[0]) {
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

	function handleChartClick(fix: (typeof fixes)[0]) {
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

	// Update map when color scheme changes
	$effect(() => {
		void colorScheme;

		untrack(() => {
			if (map && fixes.length > 0 && map.isStyleLoaded()) {
				updateMap();
			}
		});
	});

	// Cleanup on component unmount
	onDestroy(() => {
		stopPolling();
		map?.remove();
	});

	// Fit bounds with better padding to reduce wasted space
	function fitMapToBounds() {
		if (!map || fixes.length === 0) return;

		const fixesInOrder = [...fixes].reverse();
		const bounds = new maplibregl.LngLatBounds();
		fixesInOrder.forEach((fix) => {
			bounds.extend([fix.longitude, fix.latitude]);
		});

		isAutomatedZoom = true;
		map.fitBounds(bounds, { padding: 50 });
		hasUserInteracted = false;
	}

	// Reset map to auto-zoom/pan mode
	function resetMapView() {
		fitMapToBounds();
	}

	// KML download
	function downloadKML() {
		window.open(`/data/flights/${data.flight.id}/kml`, '_blank');
	}

	// IGC download
	function downloadIGC() {
		window.open(`/data/flights/${data.flight.id}/igc`, '_blank');
	}

	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			currentPage = page;
			document.getElementById('fixes-table')?.scrollIntoView({ behavior: 'smooth' });
		}
	}

	// Fetch flight gaps
	async function fetchFlightGaps() {
		if (flightGaps.length > 0) {
			showGaps = true;
			return;
		}

		isLoadingGaps = true;
		try {
			const response = await serverCall<DataResponse<FlightGap[]>>(
				`/flights/${data.flight.id}/gaps`
			);
			flightGaps = response.data;
			showGaps = true;
		} catch (err) {
			logger.error('Failed to fetch flight gaps: {error}', { error: err });
		} finally {
			isLoadingGaps = false;
		}
	}

	// Format duration from seconds to human-readable format
	function formatDuration(seconds: number): string {
		const d = dayjs.duration(seconds, 'seconds');
		const hours = Math.floor(d.asHours());
		const minutes = d.minutes();
		const secs = d.seconds();

		if (hours > 0) {
			return `${hours}h ${minutes}m ${secs}s`;
		} else if (minutes > 0) {
			return `${minutes}m ${secs}s`;
		} else {
			return `${secs}s`;
		}
	}

	// Format distance in meters to human-readable format
	function formatDistanceMeters(meters: number): string {
		const nm = meters / 1852;
		const km = meters / 1000;

		if (nm >= 1) {
			return `${nm.toFixed(2)} nm (${km.toFixed(2)} km)`;
		} else if (km >= 0.1) {
			return `${km.toFixed(2)} km`;
		} else {
			return `${meters.toFixed(0)} m`;
		}
	}

	// Format climb rate in fpm
	function formatClimbRate(fpm: number | null | undefined): string {
		if (fpm === null || fpm === undefined) return 'N/A';
		return `${fpm.toFixed(0)} fpm`;
	}

	// Check if aircraft was detected while airborne (no takeoff detected)
	const wasDetectedAirborne = $derived(!data.flight.takeoffTime);

	// Format geocoded location
	function formatLocation(city?: string | null, state?: string | null): string | null {
		const parts: string[] = [];
		if (city) parts.push(city);
		if (state) parts.push(state);
		if (!parts.length) return null;
		return parts.join(', ');
	}
</script>

<svelte:head>
	<title>Flight {data.flight.deviceAddress} | SOAR</title>
</svelte:head>

<div class="container mx-auto space-y-4 p-4">
	<!-- Flight Header -->
	<div class="card p-6">
		<div class="mb-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
			<div class="flex flex-col gap-2">
				<div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
					<h1 class="flex items-center gap-2 h1">
						<Plane class="h-8 w-8" />
						Flight
					</h1>
					<div class="flex flex-wrap items-center gap-2">
						<FlightStateBadge state={data.flight.state} />
						{#if isOutlanding}
							<span
								class="chip flex items-center gap-2 preset-filled-warning-500 text-base font-semibold"
							>
								<MapPinMinus class="h-5 w-5" />
								Outlanding
							</span>
						{/if}
					</div>
				</div>
				{#if isLoadingAircraft}
					<!-- Aircraft loading skeleton -->
					<div class="flex flex-wrap items-center gap-2">
						<div class="h-5 placeholder w-32 animate-pulse"></div>
						<span class="text-surface-400-500-token">•</span>
						<div class="h-5 placeholder w-24 animate-pulse"></div>
					</div>
				{:else if aircraft}
					<div class="flex flex-wrap items-center gap-2 text-sm">
						{#if aircraft.registration}
							<span class="font-mono font-semibold">
								{aircraft.registration}
								{#if aircraft.competitionNumber}
									<span class="text-surface-500-400-token ml-1">({aircraft.competitionNumber})</span
									>
								{/if}
							</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if data.flight.callsign}
							<span class="font-mono font-semibold">
								{data.flight.callsign}
							</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if aircraft.aircraftModel}
							<span class="font-semibold">{aircraft.aircraftModel}</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if aircraft.aircraftCategory}
							<span
								class="chip {getAircraftCategoryColor(
									aircraft.aircraftCategory
								)} text-xs font-semibold"
							>
								{getAircraftCategoryDescription(aircraft.aircraftCategory)}
							</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if data.flight.aircraftId && data.flight.deviceAddress && data.flight.deviceAddressType}
							{#if data.flight.aircraftCountryCode}
								<img
									src={getFlagPath(data.flight.aircraftCountryCode)}
									alt={getCountryName(data.flight.aircraftCountryCode) || ''}
									title={getCountryName(data.flight.aircraftCountryCode) || ''}
									class="h-4 rounded-sm"
								/>
							{/if}
							<a
								href="/aircraft/{data.flight.aircraftId}"
								target="_blank"
								rel="noopener noreferrer"
								class="btn flex items-center gap-1 preset-filled-primary-500 btn-sm"
							>
								<span class="font-mono text-xs">
									{formatAircraftAddress(data.flight.deviceAddressType, data.flight.deviceAddress)}
								</span>
								<ExternalLink class="h-3 w-3" />
							</a>
						{/if}
					</div>
				{/if}
			</div>
			<div class="flex items-center gap-2">
				{#if data.flight.previousFlightId || data.flight.nextFlightId}
					<div class="flex items-center gap-1">
						{#if data.flight.previousFlightId}
							<a
								href="/flights/{data.flight.previousFlightId}"
								class="btn preset-tonal btn-sm"
								title="Previous flight for this device"
							>
								<ChevronLeft class="h-4 w-4" />
								Previous
							</a>
						{:else}
							<button class="btn preset-tonal btn-sm" disabled title="No previous flight">
								<ChevronLeft class="h-4 w-4" />
								Previous
							</button>
						{/if}
						{#if data.flight.nextFlightId}
							<a
								href="/flights/{data.flight.nextFlightId}"
								class="btn preset-tonal btn-sm"
								title="Next flight for this device"
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</a>
						{:else}
							<button class="btn preset-tonal btn-sm" disabled title="No next flight">
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						{/if}
					</div>
				{/if}
				<button
					onclick={downloadKML}
					class="btn flex items-center gap-2 preset-filled-primary-500"
					type="button"
				>
					<Download class="h-4 w-4" />
					KML
				</button>
				<button
					onclick={downloadIGC}
					class="btn flex items-center gap-2 preset-filled-primary-500"
					type="button"
				>
					<Download class="h-4 w-4" />
					IGC
				</button>
			</div>
		</div>

		<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
			<!-- Takeoff / Detection -->
			<div class="flex items-start gap-3">
				<PlaneTakeoff class="mt-1 h-5 w-5 text-primary-500" />
				<div class="space-y-2">
					<div>
						<div class="text-surface-600-300-token mb-1 flex items-center gap-2 text-sm">
							{wasDetectedAirborne ? 'Detected' : 'Takeoff'}
							{#if wasDetectedAirborne}
								<span
									class="chip flex items-center gap-1 preset-filled-warning-500 text-xs"
									title="Takeoff was not detected. Aircraft was first observed on this flight in the air."
								>
									<Info class="h-3 w-3" />
									Detected while airborne
								</span>
							{/if}
						</div>
						<div class="font-semibold">
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(data.flight.createdAt)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(data.flight.createdAt)}</span>
						</div>
					</div>
					{#if data.flight.departureAirport}
						<div class="text-surface-600-300-token text-sm">
							{#if data.flight.departureAirportCountry}
								<img
									src={getFlagPath(data.flight.departureAirportCountry)}
									alt=""
									class="mr-1 inline-block h-3.5 rounded-sm"
								/>
							{/if}
							{#if data.flight.departureAirportId}
								<a href="/airports/{data.flight.departureAirportId}" class="anchor">
									{data.flight.departureAirport}
								</a>
							{:else}
								{data.flight.departureAirport}
							{/if}
						</div>
					{/if}
					{#if data.flight.startLocationCity || data.flight.startLocationState}
						<div class="text-surface-600-300-token flex items-center gap-1 text-sm">
							{#if data.flight.startLocationCountry}
								<img
									src={getFlagPath(data.flight.startLocationCountry)}
									alt=""
									class="h-3.5 rounded-sm"
								/>
							{/if}
							<span>
								{formatLocation(data.flight.startLocationCity, data.flight.startLocationState)}
							</span>
						</div>
					{/if}
					{#if data.flight.takeoffRunwayIdent}
						<div class="text-surface-600-300-token flex items-center gap-2 text-sm">
							<span>Runway {data.flight.takeoffRunwayIdent}</span>
							{#if data.flight.runwaysInferred === true}
								<span
									class="preset-tonal-surface-500 chip flex items-center gap-1 text-xs"
									title="This runway was inferred from the aircraft's heading during takeoff, not matched to airport runway data"
								>
									<Info class="h-3 w-3" />
									Inferred
								</span>
							{/if}
						</div>
					{/if}
				</div>
			</div>

			<!-- Landing / Lost Signal (hidden for active flights) -->
			{#if data.flight.state !== 'active'}
				<div class="flex items-start gap-3">
					<PlaneLanding class="mt-1 h-5 w-5 text-primary-500" />
					<div class="space-y-2">
						<div>
							<div class="text-surface-600-300-token mb-1 flex items-center gap-2 text-sm">
								{data.flight.landingTime && data.flight.arrivalAirport ? 'Landed' : 'Lost'}
								{#if !data.flight.landingTime || !data.flight.arrivalAirport}
									<span
										class="chip flex items-center gap-1 preset-filled-warning-500 text-xs"
										title="Landing was not detected. Aircraft lost contact with receiver."
									>
										<Info class="h-3 w-3" />
										Signal lost
									</span>
								{/if}
							</div>
							<div class="font-semibold">
								{#if data.flight.landingTime}
									<!-- Mobile: relative time only -->
									<span class="md:hidden">{formatDateTimeMobile(data.flight.landingTime)}</span>
									<!-- Desktop: relative time with full datetime -->
									<span class="hidden md:inline">{formatDateTime(data.flight.landingTime)}</span>
								{:else if data.flight.timedOutAt}
									<!-- Mobile: relative time only -->
									<span class="md:hidden">{formatDateTimeMobile(data.flight.timedOutAt)}</span>
									<!-- Desktop: relative time with full datetime -->
									<span class="hidden md:inline">{formatDateTime(data.flight.timedOutAt)}</span>
								{/if}
							</div>
						</div>
						{#if data.flight.arrivalAirport}
							<div class="text-surface-600-300-token text-sm">
								{#if data.flight.arrivalAirportCountry}
									<img
										src={getFlagPath(data.flight.arrivalAirportCountry)}
										alt=""
										class="mr-1 inline-block h-3.5 rounded-sm"
									/>
								{/if}
								{#if data.flight.arrivalAirportId}
									<a href="/airports/{data.flight.arrivalAirportId}" class="anchor">
										{data.flight.arrivalAirport}
									</a>
								{:else}
									{data.flight.arrivalAirport}
								{/if}
							</div>
						{/if}
						{#if data.flight.endLocationCity || data.flight.endLocationState}
							<div class="text-surface-600-300-token flex items-center gap-1 text-sm">
								{#if data.flight.endLocationCountry}
									<img
										src={getFlagPath(data.flight.endLocationCountry)}
										alt=""
										class="h-3.5 rounded-sm"
									/>
								{/if}
								<span>
									{formatLocation(data.flight.endLocationCity, data.flight.endLocationState)}
								</span>
							</div>
						{/if}
						{#if data.flight.landingRunwayIdent}
							<div class="text-surface-600-300-token flex items-center gap-2 text-sm">
								<span>Runway {data.flight.landingRunwayIdent}</span>
								{#if data.flight.runwaysInferred === true}
									<span
										class="preset-tonal-surface-500 chip flex items-center gap-1 text-xs"
										title="This runway was inferred from the aircraft's heading during landing, not matched to airport runway data"
									>
										<Info class="h-3 w-3" />
										Inferred
									</span>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			{/if}

			<!-- Duration -->
			{#if duration}
				<div class="flex items-start gap-3">
					<Gauge class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Duration</div>
						<div class="font-semibold">{duration}</div>
					</div>
				</div>
			{/if}

			<!-- Maximum Altitude -->
			{#if maxAltitude || maxAglAltitude}
				<div class="flex items-start gap-3">
					<MountainSnow class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Maximum Altitude</div>
						{#if maxAltitude}
							<div class="font-semibold">{formatAltitude(maxAltitude ?? undefined)} MSL</div>
						{/if}
						{#if maxAglAltitude}
							<div class="text-surface-600-300-token text-sm">
								{formatAltitude(maxAglAltitude ?? undefined)} AGL
							</div>
						{/if}
					</div>
				</div>
			{/if}

			<!-- Total Distance -->
			{#if data.flight.totalDistanceMeters}
				<div class="flex items-start gap-3">
					<Route class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Total Distance</div>
						<div class="font-semibold">{formatDistance(data.flight.totalDistanceMeters)}</div>
					</div>
				</div>
			{/if}

			<!-- Maximum Displacement -->
			{#if data.flight.maximumDisplacementMeters}
				<div class="flex items-start gap-3">
					<MoveUpRight class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Max Displacement</div>
						<div class="font-semibold">
							{formatDistance(data.flight.maximumDisplacementMeters)}
						</div>
						<div class="text-surface-600-300-token text-sm">
							from {data.flight.departureAirport}
						</div>
					</div>
				</div>
			{/if}

			<!-- Tow Aircraft -->
			{#if data.flight.towedByAircraftId}
				<div class="flex items-start gap-3">
					<TrendingUp class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Tow Aircraft</div>
						<div class="font-semibold">
							<TowAircraftLink aircraftId={data.flight.towedByAircraftId} size="md" />
						</div>
					</div>
				</div>
			{/if}

			<!-- Latest fix (for active flights) -->
			{#if data.flight.state === 'active' && fixes.length > 0}
				<div class="flex items-start gap-3">
					<Clock class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Latest fix</div>
						<div class="font-semibold">
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(fixes[0].receivedAt)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(fixes[0].receivedAt)}</span>
						</div>
						<div class="text-surface-600-300-token text-sm">
							Most recent position update
							{#if data.flight.callsign}
								<span class="text-surface-500-400-token ml-1"
									>• Callsign: {data.flight.callsign}</span
								>
							{/if}
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Aircraft Information -->
	{#if !isLoadingAircraft && aircraft && data.flight.aircraftId}
		<div class="card p-6">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="flex items-center gap-2 h2">
					<Plane class="h-6 w-6" />
					Aircraft Information
				</h2>
				<a
					href="/aircraft/{data.flight.aircraftId}"
					class="btn flex items-center gap-2 preset-filled-primary-500"
				>
					<span>View Full Aircraft Details</span>
					<ExternalLink class="h-4 w-4" />
				</a>
			</div>

			<!-- Aircraft Photos -->
			{#if !isLoadingImages && aircraftImages.length > 0}
				<div class="mb-4 flex gap-4 overflow-x-auto pb-2">
					{#each aircraftImages as image (image.thumbnailUrl)}
						<a
							href={image.pageUrl}
							target="_blank"
							rel="noopener noreferrer"
							class="group relative flex-shrink-0"
						>
							<img
								src={image.thumbnailUrl}
								alt="Aircraft photo{image.photographer ? ` by ${image.photographer}` : ''}"
								class="border-surface-300-600-token h-48 rounded-lg border object-cover transition-all group-hover:border-primary-500 group-hover:shadow-lg"
								loading="lazy"
							/>
							{#if image.photographer}
								<p class="text-surface-600-300-token mt-1 text-center text-xs">
									© {image.photographer}
								</p>
							{/if}
							<p class="text-surface-600-300-token mt-0.5 text-center text-xs capitalize">
								{image.source === 'airport_data' ? 'Airport Data' : 'Planespotters'}
							</p>
						</a>
					{/each}
				</div>
			{/if}

			<!-- Aircraft Details -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#if aircraft.registration}
					<div>
						<div class="text-surface-600-300-token text-sm">Registration</div>
						<div class="font-mono font-semibold">
							{aircraft.registration}
							{#if aircraft.competitionNumber}
								<span class="text-surface-500-400-token ml-1">({aircraft.competitionNumber})</span>
							{/if}
						</div>
					</div>
				{/if}
				{#if aircraft.aircraftModel}
					<div>
						<div class="text-surface-600-300-token text-sm">Model</div>
						<div class="font-semibold">{aircraft.aircraftModel}</div>
					</div>
				{/if}
				{#if aircraft.aircraftCategory}
					<div>
						<div class="text-surface-600-300-token text-sm">Aircraft Category</div>
						<span
							class="chip {getAircraftCategoryColor(
								aircraft.aircraftCategory
							)} text-sm font-semibold"
						>
							{getAircraftCategoryDescription(aircraft.aircraftCategory)}
						</span>
					</div>
				{/if}
				{#if aircraft.countryCode}
					{@const countryName = getCountryName(aircraft.countryCode)}
					<div>
						<div class="text-surface-600-300-token text-sm">Country</div>
						<div class="flex items-center gap-2">
							{#if getFlagPath(aircraft.countryCode)}
								<img
									src={getFlagPath(aircraft.countryCode)}
									alt="{aircraft.countryCode} flag"
									class="h-4 w-6 rounded-sm object-cover"
								/>
							{/if}
							<span class="font-semibold"
								>{countryName
									? `${countryName} (${aircraft.countryCode})`
									: aircraft.countryCode}</span
							>
						</div>
					</div>
				{/if}
				{#if aircraft.homeBaseAirportIdent}
					<div>
						<div class="text-surface-600-300-token text-sm">Home Base Airport</div>
						<div class="font-semibold">{aircraft.homeBaseAirportIdent}</div>
					</div>
				{/if}
				{#if aircraft.icaoModelCode}
					<div>
						<div class="text-surface-600-300-token text-sm">ICAO Model Code</div>
						<div class="font-mono font-semibold">{aircraft.icaoModelCode}</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Map -->
	{#if isLoadingFixes}
		<!-- Map loading skeleton -->
		<div class="card p-4">
			<div class="mb-3">
				<div class="h-8 placeholder w-48 animate-pulse"></div>
			</div>
			<div class="h-96 placeholder w-full animate-pulse rounded-lg"></div>
		</div>
	{:else if fixes.length > 0}
		<div class="card p-4">
			<div class="mb-3 flex flex-col gap-3">
				<div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
					<div class="flex items-center gap-3">
						<h2 class="h3">Flight Track</h2>
						<a
							href="/flights/{data.flight.id}/map"
							class="btn flex items-center gap-1 preset-filled-primary-500 btn-sm"
						>
							<Expand class="h-3 w-3" />
							<span>Full Screen</span>
						</a>
						<a
							href="/globe?flight={data.flight.id}"
							class="btn flex items-center gap-1 preset-filled-primary-500 btn-sm"
						>
							<Globe class="h-3 w-3" />
							<span>Globe</span>
						</a>
					</div>
					<div class="flex flex-wrap items-center gap-4">
						<label class="flex cursor-pointer items-center gap-2">
							<input
								type="checkbox"
								class="checkbox"
								bind:checked={includeNearbyFlights}
								onchange={handleNearbyFlightsToggle}
							/>
							<span class="text-sm">Include Nearby Flights</span>
							{#if isLoadingNearbyFlights}
								<span class="text-surface-600-300-token text-xs">(Loading...)</span>
							{/if}
						</label>
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
					</div>
				</div>
				<div class="flex items-center gap-2">
					<Palette class="text-surface-600-300-token h-4 w-4" />
					<span class="text-surface-600-300-token text-sm">Color by:</span>
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
							(Red = lowest, Blue = highest)
						{:else}
							(Purple = start, Orange = end)
						{/if}
					</span>
				</div>
			</div>
			<div class="relative">
				<div bind:this={mapContainer} class="h-96 w-full rounded-lg"></div>
				{#if hasUserInteracted}
					<button
						onclick={resetMapView}
						class="absolute top-3 right-3 z-10 rounded-md bg-white p-2 shadow-lg transition-all hover:scale-105 hover:shadow-xl"
						title="Reset map view to show entire flight"
						style="color: #374151;"
					>
						<LocateFixed class="h-5 w-5" />
					</button>
				{/if}
			</div>
		</div>

		<!-- Altitude Chart -->
		<div class="card p-4">
			<h2 class="mb-3 h3">Flight Profile</h2>
			<div class="h-80 w-full">
				<FlightProfile
					{fixes}
					{hasAglData}
					onHover={handleChartHover}
					onUnhover={handleChartUnhover}
					onClick={handleChartClick}
				/>
			</div>
		</div>
	{/if}

	<!-- Nearby Flights Section -->
	<div class="card p-6">
		<h2 class="mb-4 h2">Nearby Flights</h2>

		{#if !showNearbyFlightsSection}
			<button onclick={showNearbyFlights} class="btn preset-filled-primary-500" type="button">
				Find nearby flights
			</button>
		{:else if isLoadingNearbyFlights && nearbyFlights.length === 0}
			<div class="flex flex-col items-center gap-4 py-8">
				<div class="flex items-center gap-3">
					<RadarLoader />
					<span class="text-lg font-semibold">Searching for nearby flights...</span>
				</div>
				<div class="text-surface-600-300-token max-w-2xl text-center text-sm">
					<p class="mb-2">This may take a minute.</p>
					<p>
						Finding flights that were in the air within <strong>15 minutes</strong> of this flight
						and within <strong>50 miles</strong> of the departure airport.
					</p>
				</div>
			</div>
		{:else if nearbyFlights.length === 0}
			<div class="text-surface-600-300-token py-8 text-center">
				<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<p>No nearby flights found.</p>
			</div>
		{:else}
			<FlightsList flights={nearbyFlights} showEnd={true} showAircraft={true} />
		{/if}
	</div>

	<!-- Fixes Table -->
	<div class="card p-6" id="fixes-table">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="h2">
				Position Fixes {#if !isLoadingFixes}({fixes.length}){/if}
				{#if fixesPerSecond && !isLoadingFixes}
					<span class="text-surface-600-300-token ml-2 text-lg">
						({fixesPerSecond} fixes/sec)
					</span>
				{/if}
			</h2>
			<!-- Mobile collapse button -->
			<button
				class="btn preset-tonal btn-sm md:hidden"
				onclick={() => (isFixesCollapsed = !isFixesCollapsed)}
				type="button"
			>
				{#if isFixesCollapsed}
					<ChevronDown class="h-4 w-4" />
					<span>Show</span>
				{:else}
					<ChevronUp class="h-4 w-4" />
					<span>Hide</span>
				{/if}
			</button>
		</div>

		<!-- Desktop: always show; Mobile: only show when not collapsed -->
		<div class="md:block" class:hidden={isFixesCollapsed}>
			{#if isLoadingFixes}
				<!-- Fixes loading skeleton -->
				<div class="space-y-2">
					<div class="h-12 placeholder w-full animate-pulse"></div>
					<div class="h-12 placeholder w-full animate-pulse"></div>
					<div class="h-12 placeholder w-full animate-pulse"></div>
					<div class="h-12 placeholder w-full animate-pulse"></div>
					<div class="h-12 placeholder w-full animate-pulse"></div>
				</div>
			{:else if fixes.length === 0}
				<div class="text-surface-600-300-token py-8 text-center">
					<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
					<p>No position data available for this flight.</p>
				</div>
			{:else}
				<FixesList
					fixes={paginatedFixes}
					showHideInactive={false}
					showRaw={true}
					useRelativeTimes={true}
					showIntervals={true}
					showClimb={true}
					emptyMessage="No position data available for this flight."
				/>

				<!-- Pagination -->
				{#if totalPages > 1}
					<div class="mt-4 flex flex-col items-center gap-3 sm:flex-row sm:justify-between">
						<div class="text-surface-600-300-token text-sm">
							Page {currentPage} of {totalPages}
						</div>
						<div class="flex flex-wrap justify-center gap-2">
							<button
								onclick={() => goToPage(1)}
								disabled={currentPage === 1}
								class="btn preset-tonal btn-sm"
								type="button"
								title="First page (Takeoff)"
							>
								<ChevronsLeft class="h-4 w-4" />
								Takeoff
							</button>
							<button
								onclick={() => goToPage(currentPage - 1)}
								disabled={currentPage === 1}
								class="btn preset-tonal btn-sm"
								type="button"
								title="Previous page"
							>
								<ChevronLeft class="h-4 w-4" />
							</button>
							<button
								onclick={() => goToPage(currentPage + 1)}
								disabled={currentPage === totalPages}
								class="btn preset-tonal btn-sm"
								type="button"
								title="Next page"
							>
								<ChevronRight class="h-4 w-4" />
							</button>
							<button
								onclick={() => goToPage(totalPages)}
								disabled={currentPage === totalPages}
								class="btn preset-tonal btn-sm"
								type="button"
								title="Last page (Landing)"
							>
								Landing
								<ChevronsRight class="h-4 w-4" />
							</button>
						</div>
					</div>
				{/if}
			{/if}
		</div>
	</div>

	<!-- Fix Gaps Section -->
	<div class="card p-6">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="h2">Fix Gaps</h2>
		</div>

		{#if !showGaps}
			<button onclick={fetchFlightGaps} class="btn preset-filled-primary-500" type="button">
				Show fix gaps (5+ minute intervals)
			</button>
		{:else if isLoadingGaps}
			<div class="flex items-center justify-center py-8">
				<div
					class="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
				></div>
				<span class="ml-2">Loading fix gaps...</span>
			</div>
		{:else if flightGaps.length === 0}
			<div class="text-surface-600-300-token py-8 text-center">
				<Info class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<p>No significant gaps found (all fixes within 5 minutes of each other).</p>
			</div>
		{:else}
			<div class="space-y-4">
				<div class="text-surface-600-300-token text-sm">
					Found {flightGaps.length} gap{flightGaps.length !== 1 ? 's' : ''} of 5+ minutes between fixes
				</div>

				{#each flightGaps as gap, index (gap.gapStart)}
					<div class="preset-tonal-surface-500 card p-4">
						<div class="mb-3 flex items-center gap-2">
							<span class="chip preset-filled-warning-500 text-sm font-semibold">
								Gap #{index + 1}
							</span>
							<span class="text-lg font-semibold">{formatDuration(gap.durationSeconds)}</span>
						</div>

						<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
							<!-- Time Information -->
							<div>
								<div class="text-surface-600-300-token text-sm">Start Time</div>
								<div class="font-semibold">{dayjs(gap.gapStart).format('h:mm:ss A')}</div>
								<div class="text-surface-500-400-token text-xs">
									{dayjs(gap.gapStart).format('MMM D, YYYY')}
								</div>
							</div>

							<div>
								<div class="text-surface-600-300-token text-sm">End Time</div>
								<div class="font-semibold">{dayjs(gap.gapEnd).format('h:mm:ss A')}</div>
								<div class="text-surface-500-400-token text-xs">
									{dayjs(gap.gapEnd).format('MMM D, YYYY')}
								</div>
							</div>

							<div>
								<div class="text-surface-600-300-token text-sm">Distance Covered</div>
								<div class="font-semibold">{formatDistanceMeters(gap.distanceMeters)}</div>
							</div>

							<!-- Callsign and Squawk -->
							{#if gap.callsignBefore || gap.callsignAfter}
								<div>
									<div class="text-surface-600-300-token text-sm">Callsign</div>
									<div class="font-mono text-sm">
										{#if gap.callsignBefore === gap.callsignAfter}
											{gap.callsignBefore || 'N/A'}
										{:else}
											<span>{gap.callsignBefore || 'N/A'}</span>
											<span class="text-surface-500-400-token mx-1">→</span>
											<span>{gap.callsignAfter || 'N/A'}</span>
										{/if}
									</div>
								</div>
							{/if}

							{#if gap.squawkBefore || gap.squawkAfter}
								<div>
									<div class="text-surface-600-300-token text-sm">Squawk Code</div>
									<div class="font-mono text-sm">
										{#if gap.squawkBefore === gap.squawkAfter}
											{gap.squawkBefore || 'N/A'}
										{:else}
											<span>{gap.squawkBefore || 'N/A'}</span>
											<span class="text-surface-500-400-token mx-1">→</span>
											<span>{gap.squawkAfter || 'N/A'}</span>
										{/if}
									</div>
								</div>
							{/if}

							<!-- Climb Rates -->
							<div>
								<div class="text-surface-600-300-token text-sm">
									Climb Rate (immediately before/after)
								</div>
								<div class="text-sm">
									<span>{formatClimbRate(gap.climbRateBefore)}</span>
									<span class="text-surface-500-400-token mx-1">→</span>
									<span>{formatClimbRate(gap.climbRateAfter)}</span>
								</div>
							</div>

							<div>
								<div class="text-surface-600-300-token text-sm">
									Avg Climb Rate (10 fixes before/after)
								</div>
								<div class="text-sm">
									<span>{formatClimbRate(gap.avgClimbRate10Before)}</span>
									<span class="text-surface-500-400-token mx-1">→</span>
									<span>{formatClimbRate(gap.avgClimbRate10After)}</span>
								</div>
							</div>

							<div>
								<div class="text-surface-600-300-token text-sm">Average Speed</div>
								<div class="text-sm">
									{#if gap.durationSeconds > 0}
										{((gap.distanceMeters / gap.durationSeconds) * 1.94384).toFixed(1)} knots
									{:else}
										N/A
									{/if}
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
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
