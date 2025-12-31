<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onDestroy, untrack } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
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
	import type { Flight, Receiver } from '$lib/types';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import durationPlugin from 'dayjs/plugin/duration';
	import {
		getAircraftTypeOgnDescription,
		formatAircraftAddress,
		getAircraftTypeColor,
		getFlagPath,
		getCountryName
	} from '$lib/formatters';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { serverCall } from '$lib/api/server';
	import FlightStateBadge from '$lib/components/FlightStateBadge.svelte';
	import RadarLoader from '$lib/components/RadarLoader.svelte';
	import FixesList from '$lib/components/FixesList.svelte';
	import FlightsList from '$lib/components/FlightsList.svelte';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';
	import FlightProfile from '$lib/components/FlightProfile.svelte';

	dayjs.extend(relativeTime);
	dayjs.extend(durationPlugin);

	let { data }: { data: PageData } = $props();

	// Progressive loading state - await promises from page load
	import type { Aircraft, Fix } from '$lib/types';
	let aircraft = $state<Aircraft | undefined>(undefined);
	let fixes = $state<Fix[]>([]);
	let isLoadingAircraft = $state(true);
	let isLoadingFixes = $state(true);

	// Load aircraft and fixes progressively
	$effect(() => {
		isLoadingAircraft = true;
		data.aircraftPromise.then((result) => {
			aircraft = result;
			isLoadingAircraft = false;
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

	interface FlightGapsResponse {
		gaps: FlightGap[];
		count: number;
	}

	let mapContainer = $state<HTMLElement>();
	let map = $state<google.maps.Map>();
	let flightPathSegments = $state<google.maps.Polyline[]>([]);
	let altitudeInfoWindow = $state<google.maps.InfoWindow | null>(null);
	let fixMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let pollingInterval: ReturnType<typeof setInterval> | null = null;
	let hoverMarker = $state<google.maps.marker.AdvancedMarkerElement | null>(null);

	// Track user interaction with map
	let hasUserInteracted = $state(false);
	let isAutomatedZoom = false;

	// Pagination state
	let currentPage = $state(1);
	let pageSize = 50;

	// Display options
	let includeNearbyFlights = $state(false);
	let showReceivers = $state(false);

	// Color scheme selection
	type ColorScheme = 'altitude' | 'time';
	let colorScheme = $state<ColorScheme>('altitude');

	// Nearby flights data - shared between map and section
	let nearbyFlights = $state<Flight[]>([]);
	let nearbyFlightsFixes = $state<Map<string, typeof fixes>>(new Map());
	let nearbyFlightPaths = $state<google.maps.Polyline[]>([]);
	let isLoadingNearbyFlights = $state(false);
	let showNearbyFlightsSection = $state(false);

	// Receiver data
	let receivers = $state<Receiver[]>([]);
	let receiverMarkers = $state<google.maps.marker.AdvancedMarkerElement[]>([]);
	let isLoadingReceivers = $state(false);

	// Fix gaps data
	let flightGaps = $state<FlightGap[]>([]);
	let isLoadingGaps = $state(false);
	let showGaps = $state(false);

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

	// Helper function to determine which fixes should display arrow markers
	// Returns a Set of indices for fixes that should show arrows
	function getArrowFixIndices(fixesInOrder: Fix[]): SvelteSet<number> {
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
		// Red: rgb(239, 68, 68) - #ef4444
		// Blue: rgb(59, 130, 246) - #3b82f6
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
		const minAlt = minAltitude ?? 0;
		const maxAlt = maxAltitude ?? 1000;

		// Track last arrow timestamp to display one every 10 minutes
		let lastArrowTime: Date | null = null;
		const TEN_MINUTES_MS = 10 * 60 * 1000;

		const reversedFixesForArrows = [...fixes].reverse();
		const totalFixes = reversedFixesForArrows.length;

		flightPathSegments.forEach((segment, index) => {
			const path = segment.getPath();
			if (path.getLength() < 2) return;

			// Get color based on segment index
			if (index >= reversedFixesForArrows.length) return;
			const fix = reversedFixesForArrows[index];
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
		fixesInOrder: Fix[],
		targetMap: google.maps.Map
	): google.maps.Polyline[] {
		const minAlt = minAltitude ?? 0;
		const maxAlt = maxAltitude ?? 1000;
		const segments: google.maps.Polyline[] = [];
		const zoom = targetMap.getZoom() ?? 12;
		const scale = getArrowScale(zoom);
		const totalFixes = fixesInOrder.length;

		// Track last arrow timestamp to display one every 10 minutes
		let lastArrowTime: Date | null = null;
		const TEN_MINUTES_MS = 10 * 60 * 1000;

		// Create a polyline segment for each pair of consecutive fixes
		for (let i = 0; i < fixesInOrder.length - 1; i++) {
			const fix1 = fixesInOrder[i];
			const fix2 = fixesInOrder[i + 1];

			// Use the starting fix's color for the segment
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
		// Convert meters to nautical miles (1 nm = 1852 meters)
		const nm = meters / 1852;
		// Convert meters to kilometers
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
			// Also show the nearby flights section
			showNearbyFlightsSection = true;
		} else {
			// Clear nearby flights from map only (keep the data cached)
			nearbyFlightPaths.forEach((path) => path.setMap(null));
			nearbyFlightPaths = [];
		}
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

	// Show nearby flights section and fetch data if needed
	async function showNearbyFlights() {
		showNearbyFlightsSection = true;

		// If data is already cached, no need to fetch again
		if (nearbyFlights.length > 0) {
			return;
		}

		// Otherwise fetch the data
		isLoadingNearbyFlights = true;
		try {
			const flights = await serverCall<Flight[]>(`/flights/${data.flight.id}/nearby`);
			nearbyFlights = flights;
		} catch (err) {
			console.error('Failed to fetch nearby flights:', err);
		} finally {
			isLoadingNearbyFlights = false;
		}
	}

	// Helper function to filter fixes to only those in viewport (with padding)
	function filterFixesToViewport(fixes: Fix[], bounds: google.maps.LatLngBounds): typeof fixes {
		// Expand bounds by ~20% in each direction to include slightly off-screen fixes
		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();
		const latPadding = (ne.lat() - sw.lat()) * 0.2;
		const lngPadding = (ne.lng() - sw.lng()) * 0.2;

		const paddedBounds = new google.maps.LatLngBounds(
			{ lat: sw.lat() - latPadding, lng: sw.lng() - lngPadding },
			{ lat: ne.lat() + latPadding, lng: ne.lng() + lngPadding }
		);

		return fixes.filter((fix) => {
			const latLng = { lat: fix.latitude, lng: fix.longitude };
			return paddedBounds.contains(latLng);
		});
	}

	// Helper function to simplify polyline by reducing point density
	function simplifyPath(fixes: Fix[], maxPoints: number = 500): { lat: number; lng: number }[] {
		if (fixes.length <= maxPoints) {
			// No simplification needed
			return fixes.map((fix) => ({ lat: fix.latitude, lng: fix.longitude }));
		}

		// Use a simple decimation algorithm - take every Nth point
		// Always include first and last points
		const result: { lat: number; lng: number }[] = [];
		const step = Math.ceil(fixes.length / maxPoints);

		result.push({ lat: fixes[0].latitude, lng: fixes[0].longitude });

		for (let i = step; i < fixes.length - 1; i += step) {
			result.push({ lat: fixes[i].latitude, lng: fixes[i].longitude });
		}

		// Always include the last point
		if (fixes.length > 1) {
			const last = fixes[fixes.length - 1];
			result.push({ lat: last.latitude, lng: last.longitude });
		}

		return result;
	}

	// Update nearby flight paths based on current map viewport
	function updateNearbyFlightPaths() {
		if (!map || nearbyFlightsFixes.size === 0) return;

		const bounds = map.getBounds();
		if (!bounds) return;

		// Clear existing paths
		nearbyFlightPaths.forEach((path) => path.setMap(null));
		nearbyFlightPaths = [];

		// Color palette for nearby flights (excluding red which is used for main flight)
		const colors = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4'];

		// Store map reference for use in closure
		const currentMap = map;

		// Render polylines for each nearby flight
		Array.from(nearbyFlightsFixes.values()).forEach((fixes, i) => {
			if (fixes.length === 0) return;

			const fixesInOrder = [...fixes].reverse();

			// Filter to viewport and simplify the path
			const viewportFixes = filterFixesToViewport(fixesInOrder, bounds);
			if (viewportFixes.length === 0) return;

			const pathCoordinates = simplifyPath(viewportFixes, 500);

			const flightPath = new google.maps.Polyline({
				path: pathCoordinates,
				geodesic: true,
				strokeColor: colors[i % colors.length],
				strokeOpacity: 0.6,
				strokeWeight: 2
			});

			flightPath.setMap(currentMap);
			nearbyFlightPaths.push(flightPath);
		});
	}

	// Fetch nearby flights and their fixes
	async function fetchNearbyFlights() {
		// If data is already cached, just render the map paths
		if (nearbyFlights.length > 0 && map) {
			// Check if we already have fixes cached
			if (nearbyFlightsFixes.size > 0) {
				updateNearbyFlightPaths();
				return;
			}
		}

		isLoadingNearbyFlights = true;
		try {
			// Fetch nearby flights only if not already cached
			if (nearbyFlights.length === 0) {
				const flights = await serverCall<Flight[]>(`/flights/${data.flight.id}/nearby`);
				nearbyFlights = flights;
			}

			if (map) {
				// Fetch all fixes in parallel for better performance
				const fixesPromises = nearbyFlights.map((nearbyFlight) =>
					serverCall<{
						fixes: Fix[];
						count: number;
					}>(`/flights/${nearbyFlight.id}/fixes`)
						.then((response) => ({ flightId: nearbyFlight.id, fixes: response.fixes }))
						.catch((err) => {
							console.error(`Failed to fetch fixes for nearby flight ${nearbyFlight.id}:`, err);
							return null;
						})
				);

				const allFixesResponses = await Promise.all(fixesPromises);

				// Store all fixes in the map
				nearbyFlightsFixes.clear();
				allFixesResponses.forEach((response) => {
					if (response && response.fixes.length > 0) {
						nearbyFlightsFixes.set(response.flightId, response.fixes);
					}
				});

				// Render paths based on current viewport
				updateNearbyFlightPaths();
			}
		} catch (err) {
			console.error('Failed to fetch nearby flights:', err);
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
			if (!bounds) return;

			const ne = bounds.getNorthEast();
			const sw = bounds.getSouthWest();

			const params = new URLSearchParams({
				latitude_min: sw.lat().toString(),
				latitude_max: ne.lat().toString(),
				longitude_min: sw.lng().toString(),
				longitude_max: ne.lng().toString()
			});

			const data = await serverCall(`/receivers?${params}`);
			if (!data || typeof data !== 'object' || !('receivers' in data)) {
				throw new Error('Invalid response format');
			}

			const response = data as { receivers: unknown[] };
			receivers = response.receivers.filter((receiver: unknown): receiver is Receiver => {
				// Validate receiver object
				if (typeof receiver !== 'object' || receiver === null) {
					console.error('Invalid receiver: not an object or is null', receiver);
					return false;
				}

				// Check required fields
				const requiredFields = ['id', 'callsign', 'latitude', 'longitude'] as const;
				for (const field of requiredFields) {
					if (!(field in receiver)) {
						console.error(`Invalid receiver: missing required field "${field}"`, receiver);
						return false;
					}
				}

				// Validate latitude and longitude are numbers (or null)
				const lat = (receiver as Record<string, unknown>).latitude;
				const lng = (receiver as Record<string, unknown>).longitude;

				if (lat !== null && typeof lat !== 'number') {
					console.error('Invalid receiver: latitude is not a number or null', receiver);
					return false;
				}

				if (lng !== null && typeof lng !== 'number') {
					console.error('Invalid receiver: longitude is not a number or null', receiver);
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

	// Poll for updates to in-progress flights
	async function pollForUpdates() {
		try {
			// Get the timestamp of the most recent fix (fixes are in DESC order, so first element is newest)
			const latestFixTimestamp = fixes.length > 0 ? fixes[0].timestamp : null;

			// Build URL with 'after' parameter if we have fixes
			const fixesUrl = latestFixTimestamp
				? `/flights/${data.flight.id}/fixes?after=${encodeURIComponent(latestFixTimestamp)}`
				: `/flights/${data.flight.id}/fixes`;

			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<{
					flight: typeof data.flight;
				}>(`/flights/${data.flight.id}`),
				serverCall<{
					fixes: Fix[];
					count: number;
				}>(fixesUrl)
			]);

			// Update flight data
			data.flight = flightResponse.flight;
			// Device doesn't change during a flight, so we don't re-fetch it

			// Append new fixes to the existing list (new fixes are in DESC order)
			if (fixesResponse.fixes.length > 0) {
				fixes = [...fixesResponse.fixes, ...fixes];
				fixes.length = fixes.length;
			}

			// If flight has landed or timed out, stop polling
			if (data.flight.state !== 'active') {
				stopPolling();
			}

			// Only update map if we got new fixes (chart updates automatically via FlightProfile component)
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
			pollingInterval = setInterval(pollForUpdates, 10000); // Poll every 10 seconds
		}
	}

	// Stop polling
	function stopPolling() {
		if (pollingInterval) {
			clearInterval(pollingInterval);
			pollingInterval = null;
		}
	}

	// Update map with new data (chart updates automatically via FlightProfile component)
	function updateMap() {
		if (fixes.length === 0 || !map || flightPathSegments.length === 0) return;

		const fixesInOrder = [...fixes].reverse();

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
			// Re-add fix markers as directional arrows
			const minAlt = minAltitude ?? 0;
			const maxAlt = maxAltitude ?? 1000;
			const arrowIndices = getArrowFixIndices(fixesInOrder);
			const totalFixes = fixesInOrder.length;

			fixesInOrder.forEach((fix, index) => {
				// Only create arrow markers for selected indices
				if (!arrowIndices.has(index)) return;

				// Calculate bearing to next fix (or use previous bearing for last fix)
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
					// For last fix, use bearing from previous fix
					const prevFix = fixesInOrder[index - 1];
					bearing = calculateBearing(
						prevFix.latitude,
						prevFix.longitude,
						fix.latitude,
						fix.longitude
					);
				}

				// Get color based on current scheme
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
					const heading =
						fix.trackDegrees !== undefined ? Math.round(fix.trackDegrees) + '°' : 'N/A';
					const turnRate =
						fix.turnRateRot !== undefined ? fix.turnRateRot.toFixed(2) + ' rot/min' : 'N/A';
					const climbRate = fix.climbFpm !== undefined ? Math.round(fix.climbFpm) + ' fpm' : 'N/A';
					const groundSpeed =
						fix.groundSpeedKnots !== undefined ? Math.round(fix.groundSpeedKnots) + ' kt' : 'N/A';
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
		});

		// Update bounds to show all fixes (only if user hasn't manually interacted)
		if (!hasUserInteracted) {
			fitMapToBounds();
		}
	}

	// Initialize map
	// Initialize map when fixes are loaded (uses $effect to re-run when fixes change)
	$effect(() => {
		if (fixes.length === 0 || !mapContainer || isLoadingFixes) return;

		// Use untrack to avoid creating reactive dependencies on map state
		untrack(async () => {
			try {
				setOptions({
					key: GOOGLE_MAPS_API_KEY,
					v: 'weekly'
				});

				await importLibrary('maps');
				await importLibrary('marker');

				// Use reversed fixes for chronological order (earliest to latest)
				const fixesInOrder = [...fixes].reverse();

				// Calculate center and bounds
				const bounds = new google.maps.LatLngBounds();
				fixesInOrder.forEach((fix) => {
					bounds.extend({ lat: fix.latitude, lng: fix.longitude });
				});

				const center = bounds.getCenter();

				// Create map with satellite view by default
				if (!mapContainer) return;
				map = new google.maps.Map(mapContainer, {
					center: { lat: center.lat(), lng: center.lng() },
					zoom: 12,
					mapId: 'FLIGHT_MAP',
					mapTypeId: google.maps.MapTypeId.SATELLITE,
					streetViewControl: false,
					fullscreenControl: false
				});

				// Fit bounds with tighter padding
				fitMapToBounds();

				// Create gradient polyline segments
				flightPathSegments = createGradientPolylines(fixesInOrder, map);

				// Track user interaction with map
				let isDragging = false;

				map.addListener('dragstart', () => {
					isDragging = true;
				});

				map.addListener('dragend', () => {
					if (isDragging) {
						hasUserInteracted = true;
						isDragging = false;
					}
				});

				// Add zoom listener to update arrow scales and track user interaction
				map.addListener('zoom_changed', () => {
					updateArrowScales();

					// If zoom wasn't automated, mark as user interaction
					if (!isAutomatedZoom) {
						hasUserInteracted = true;
					}
					isAutomatedZoom = false;
				});

				// Add bounds_changed listener to update nearby flights when panning/zooming
				// Use debouncing to avoid excessive re-renders
				let boundsChangedTimeout: ReturnType<typeof setTimeout> | null = null;
				map.addListener('bounds_changed', () => {
					if (boundsChangedTimeout) {
						clearTimeout(boundsChangedTimeout);
					}
					boundsChangedTimeout = setTimeout(() => {
						// Only update if nearby flights are enabled
						if (includeNearbyFlights && nearbyFlightsFixes.size > 0) {
							updateNearbyFlightPaths();
						}
					}, 300); // 300ms debounce
				});

				// Create info window for altitude display
				altitudeInfoWindow = new google.maps.InfoWindow();

				// Wait for map to be fully initialized before adding advanced markers
				google.maps.event.addListenerOnce(map, 'idle', () => {
					// Add takeoff marker (green) - first fix chronologically
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

					// Add landing marker (red) if flight is complete - last fix chronologically
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
					const minAlt = minAltitude ?? 0;
					const maxAlt = maxAltitude ?? 1000;
					const arrowIndices = getArrowFixIndices(fixesInOrder);
					const totalFixes = fixesInOrder.length;

					fixesInOrder.forEach((fix, index) => {
						// Only create arrow markers for selected indices
						if (!arrowIndices.has(index)) return;

						// Calculate bearing to next fix (or use previous bearing for last fix)
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
							// For last fix, use bearing from previous fix
							const prevFix = fixesInOrder[index - 1];
							bearing = calculateBearing(
								prevFix.latitude,
								prevFix.longitude,
								fix.latitude,
								fix.longitude
							);
						}

						// Get color based on current scheme
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
							const heading =
								fix.trackDegrees !== undefined ? Math.round(fix.trackDegrees) + '°' : 'N/A';
							const turnRate =
								fix.turnRateRot !== undefined ? fix.turnRateRot.toFixed(2) + ' rot/min' : 'N/A';
							const climbRate =
								fix.climbFpm !== undefined ? Math.round(fix.climbFpm) + ' fpm' : 'N/A';
							const groundSpeed =
								fix.groundSpeedKnots !== undefined
									? Math.round(fix.groundSpeedKnots) + ' kt'
									: 'N/A';
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
				});
			} catch (error) {
				console.error('Failed to load Google Maps:', error);
			}

			// Start polling if flight is in progress
			startPolling();
		}); // end untrack
	}); // end $effect

	// Callbacks for chart hover interaction with map
	function handleChartHover(fix: (typeof fixes)[0]) {
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

	function handleChartClick(fix: (typeof fixes)[0]) {
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

	// Update map when color scheme changes
	$effect(() => {
		// Track colorScheme changes
		void colorScheme;

		// Untrack the rest to avoid infinite loop when updateMap modifies state
		untrack(() => {
			if (map && fixes.length > 0 && flightPathSegments.length > 0) {
				updateMap();
			}
		});
	});

	// Cleanup on component unmount
	onDestroy(() => {
		stopPolling();
	});

	// Fit bounds with better padding to reduce wasted space
	function fitMapToBounds() {
		if (!map || fixes.length === 0) return;

		const fixesInOrder = [...fixes].reverse();
		const bounds = new google.maps.LatLngBounds();
		fixesInOrder.forEach((fix) => {
			bounds.extend({ lat: fix.latitude, lng: fix.longitude });
		});

		// Mark this as an automated zoom
		isAutomatedZoom = true;

		// Use tighter padding (50px instead of default ~150px)
		map.fitBounds(bounds, { top: 50, right: 50, bottom: 50, left: 50 });

		// Reset the interaction flag since this is an automated zoom
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

	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			currentPage = page;
			// Scroll to top of fixes table
			document.getElementById('fixes-table')?.scrollIntoView({ behavior: 'smooth' });
		}
	}

	// Fetch flight gaps
	async function fetchFlightGaps() {
		if (flightGaps.length > 0) {
			// Already loaded
			showGaps = true;
			return;
		}

		isLoadingGaps = true;
		try {
			const response = await serverCall<FlightGapsResponse>(`/flights/${data.flight.id}/gaps`);
			flightGaps = response.gaps;
			showGaps = true;
		} catch (err) {
			console.error('Failed to fetch flight gaps:', err);
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
						{#if aircraft.aircraftTypeOgn}
							<span
								class="chip {getAircraftTypeColor(aircraft.aircraftTypeOgn)} text-xs font-semibold"
							>
								{getAircraftTypeOgnDescription(aircraft.aircraftTypeOgn)}
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
			</div>
		</div>

		<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
			<!-- Takeoff -->
			<div class="flex items-start gap-3">
				<PlaneTakeoff class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Takeoff</div>
					<div class="font-semibold">
						{#if data.flight.takeoffTime}
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(data.flight.takeoffTime)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(data.flight.takeoffTime)}</span>
						{:else}
							Unknown
						{/if}
					</div>
					<div class="text-surface-600-300-token text-sm">
						{#if data.flight.departureAirport && data.flight.departureAirportId}
							{#if data.flight.departureAirportCountry}
								<img
									src={getFlagPath(data.flight.departureAirportCountry)}
									alt=""
									class="mr-1 inline-block h-3.5 rounded-sm"
								/>
							{/if}
							<a href="/airports/{data.flight.departureAirportId}" class="anchor">
								{data.flight.departureAirport}
							</a>
						{:else if data.flight.departureAirport}
							{#if data.flight.departureAirportCountry}
								<img
									src={getFlagPath(data.flight.departureAirportCountry)}
									alt=""
									class="mr-1 inline-block h-3.5 rounded-sm"
								/>
							{/if}
							{data.flight.departureAirport}
						{:else}
							Unknown
						{/if}
					</div>
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
					{:else if data.flight.departureAirport}
						<div class="text-surface-600-300-token text-sm">Runway Unknown</div>
					{/if}
				</div>
			</div>

			<!-- Landing / Timeout (hidden for active flights) -->
			{#if data.flight.state === 'timed_out' || data.flight.landingTime}
				<div class="flex items-start gap-3">
					<PlaneLanding class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">
							{data.flight.state === 'timed_out' ? 'Timed Out' : 'Landing'}
						</div>
						<div class="font-semibold">
							{#if data.flight.state === 'timed_out' && data.flight.timedOutAt}
								<!-- Mobile: relative time only -->
								<span class="md:hidden">{formatDateTimeMobile(data.flight.timedOutAt)}</span>
								<!-- Desktop: relative time with full datetime -->
								<span class="hidden md:inline">{formatDateTime(data.flight.timedOutAt)}</span>
							{:else if data.flight.landingTime}
								<!-- Mobile: relative time only -->
								<span class="md:hidden">{formatDateTimeMobile(data.flight.landingTime)}</span>
								<!-- Desktop: relative time with full datetime -->
								<span class="hidden md:inline">{formatDateTime(data.flight.landingTime)}</span>
							{/if}
						</div>
						<div class="text-surface-600-300-token text-sm">
							{#if data.flight.state === 'timed_out'}
								No beacons received for 5+ minutes
							{:else if data.flight.landingTime}
								{#if data.flight.arrivalAirport && data.flight.arrivalAirportId}
									{#if data.flight.arrivalAirportCountry}
										<img
											src={getFlagPath(data.flight.arrivalAirportCountry)}
											alt=""
											class="mr-1 inline-block h-3.5 rounded-sm"
										/>
									{/if}
									<a href="/airports/{data.flight.arrivalAirportId}" class="anchor">
										{data.flight.arrivalAirport}
									</a>
								{:else if data.flight.arrivalAirport}
									{#if data.flight.arrivalAirportCountry}
										<img
											src={getFlagPath(data.flight.arrivalAirportCountry)}
											alt=""
											class="mr-1 inline-block h-3.5 rounded-sm"
										/>
									{/if}
									{data.flight.arrivalAirport}
								{:else}
									Unknown
								{/if}
							{/if}
						</div>
						{#if data.flight.landingTime && data.flight.arrivalAirport}
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
							{:else}
								<div class="text-surface-600-300-token text-sm">Runway Unknown</div>
							{/if}
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

			<!-- Recognized at -->
			<div class="flex items-start gap-3">
				<Clock class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Recognized at</div>
					<div class="font-semibold">
						<!-- Mobile: relative time only -->
						<span class="md:hidden">{formatDateTimeMobile(data.flight.createdAt)}</span>
						<!-- Desktop: relative time with full datetime -->
						<span class="hidden md:inline">{formatDateTime(data.flight.createdAt)}</span>
					</div>
					<div class="text-surface-600-300-token text-sm">When flight was first detected</div>
				</div>
			</div>

			<!-- Latest fix (for active flights) -->
			{#if data.flight.state === 'active' && fixes.length > 0}
				<div class="flex items-start gap-3">
					<Clock class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Latest fix</div>
						<div class="font-semibold">
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(fixes[0].timestamp)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(fixes[0].timestamp)}</span>
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
		</div>

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
									Avg Climb Rate (10 fixes before)
								</div>
								<div class="text-sm">{formatClimbRate(gap.avgClimbRate10Before)}</div>
							</div>

							<div>
								<div class="text-surface-600-300-token text-sm">
									Avg Climb Rate (10 fixes after)
								</div>
								<div class="text-sm">{formatClimbRate(gap.avgClimbRate10After)}</div>
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
