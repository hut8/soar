<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { serverCall } from '$lib/api/server';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import { page } from '$app/stores';
	import { Settings, ListChecks, MapPlus, MapMinus } from '@lucide/svelte';
	import WatchlistModal from '$lib/components/WatchlistModal.svelte';
	import SettingsModal from '$lib/components/SettingsModal.svelte';
	import AircraftStatusModal from '$lib/components/AircraftStatusModal.svelte';
	import AirportModal from '$lib/components/AirportModal.svelte';
	import AirspaceModal from '$lib/components/AirspaceModal.svelte';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { FixFeed } from '$lib/services/FixFeed';
	import type {
		Aircraft,
		Receiver,
		Airspace,
		AirspaceFeatureCollection,
		Fix,
		Airport,
		DataListResponse,
		DataResponse,
		AircraftCluster
	} from '$lib/types';
	import { isAircraftItem, isClusterItem } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { debugStatus } from '$lib/stores/websocket-status';
	import { browser } from '$app/environment';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import type { AircraftRegistryEvent } from '$lib/services/AircraftRegistry';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';

	// Extend dayjs with relative time plugin
	dayjs.extend(relativeTime);

	// Area tracker configuration
	const AREA_TRACKER_LIMIT_ENABLED = false;

	// Aircraft rendering limit to prevent browser crashes
	const MAX_AIRCRAFT_DISPLAY = 50;

	let mapContainer: HTMLElement;
	let map: google.maps.Map;
	let userLocationButton: HTMLButtonElement;
	let isLocating = $state(false);
	let userMarker: google.maps.marker.AdvancedMarkerElement | null = null;

	// Compass rose variables
	let aircraftHeading: number = 0;
	let compassHeading: number = $state(0);
	let previousCompassHeading: number = 0;
	let isCompassActive: boolean = $state(false);
	let displayHeading: number = $state(0);

	// Airport display variables
	let airports: Airport[] = [];
	let airportMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let shouldShowAirports: boolean = false;
	let airportUpdateDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Receiver display variables
	let receivers: Receiver[] = [];
	let receiverMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let shouldShowReceivers: boolean = false;
	let receiverUpdateDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Airspace display variables
	let airspacePolygons: google.maps.Polygon[] = [];
	let shouldShowAirspaces: boolean = false;
	let airspaceUpdateDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Zoom debounce timer
	let zoomDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Aircraft display variables
	let aircraftMarkers = new SvelteMap<string, google.maps.marker.AdvancedMarkerElement>();
	let clusterMarkers = new SvelteMap<string, google.maps.marker.AdvancedMarkerElement>();
	let clusterPolygons = new SvelteMap<string, google.maps.Polygon>();
	let latestFixes = new SvelteMap<string, Fix>();

	// Aircraft trail variables
	interface TrailData {
		polylines: google.maps.Polyline[];
		dots: google.maps.Circle[];
	}
	let aircraftTrails = new SvelteMap<string, TrailData>();

	// Settings modal state
	let showSettingsModal = $state(false);
	let showWatchlistModal = $state(false);

	// Aircraft status modal state
	let showAircraftStatusModal = $state(false);
	let selectedAircraft: Aircraft | null = $state(null);

	// Airport modal state
	let showAirportModal = $state(false);
	let selectedAirport: Airport | null = $state(null);

	// Airspace modal state
	let showAirspaceModal = $state(false);
	let selectedAirspace: Airspace | null = $state(null);

	// Settings state - these will be updated by the SettingsModal
	let currentSettings = $state({
		showCompassRose: true,
		showAirportMarkers: true,
		showReceiverMarkers: true,
		showAirspaceMarkers: true,
		showRunwayOverlays: false,
		positionFixWindow: 8
	});

	// Debounced update for aircraft trails
	let updateTrailsTimeout: ReturnType<typeof setTimeout> | null = null;
	function debouncedUpdateAircraftTrails() {
		if (updateTrailsTimeout) {
			clearTimeout(updateTrailsTimeout);
		}
		updateTrailsTimeout = setTimeout(() => {
			if (map) {
				updateAllAircraftTrails();
			}
		}, 300); // 300ms debounce
	}

	// Handle settings changes from SettingsModal
	function handleSettingsChange(newSettings: {
		showCompassRose: boolean;
		showAirportMarkers: boolean;
		showReceiverMarkers: boolean;
		showAirspaceMarkers: boolean;
		showRunwayOverlays: boolean;
		positionFixWindow: number;
	}) {
		const previousFixWindow = currentSettings.positionFixWindow;

		// Replace entire object to ensure Svelte 5 reactivity triggers
		currentSettings = { ...newSettings };

		// Update aircraft trails when position fix window changes
		if (previousFixWindow !== newSettings.positionFixWindow) {
			debouncedUpdateAircraftTrails();
		}
	}

	// Handle aircraft marker click
	function handleAircraftClick(aircraft: Aircraft) {
		console.log('[AIRCRAFT CLICK] Aircraft clicked:', aircraft.registration || aircraft.address);
		selectedAircraft = aircraft;
		showAircraftStatusModal = true;
	}

	// Center of continental US
	const CONUS_CENTER = {
		lat: 39.8283,
		lng: -98.5795
	};

	// Map persistence keys for localStorage
	const MAP_STATE_KEY = 'operations-map-state';
	const AREA_TRACKER_KEY = 'operations-area-tracker';
	const MAP_TYPE_KEY = 'operations-map-type';

	// Interface for stored map state
	interface MapState {
		center: google.maps.LatLngLiteral;
		zoom: number;
	}

	// Helper function to calculate color based on altitude (red at 500 ft, blue at 18000+ ft)
	function getAltitudeColor(altitudeMslFeet: number | null | undefined): string {
		const altitude = altitudeMslFeet || 0;

		// Clamp altitude to 500-18000 range for color calculation
		const clampedAltitude = Math.max(500, Math.min(18000, altitude));

		// Calculate interpolation factor (0 to 1)
		// 500 ft = 0, 18000 ft = 1
		const factor = (clampedAltitude - 500) / (18000 - 500);

		// Red: rgb(239, 68, 68) or #ef4444
		// Blue: rgb(59, 130, 246) or #3b82f6
		const r = Math.round(239 - (239 - 59) * factor);
		const g = Math.round(68 + (130 - 68) * factor);
		const b = Math.round(68 + (246 - 68) * factor);

		return `rgb(${r}, ${g}, ${b})`;
	}

	// Helper function to get marker color based on active status and altitude
	function getMarkerColor(fix: Fix): string {
		// Use gray for inactive fixes (no current flight)
		if (!fix.active) {
			return 'rgb(156, 163, 175)'; // Gray-400
		}
		// Use altitude-based color for active fixes
		return getAltitudeColor(fix.altitudeMslFeet);
	}

	// Helper function to format altitude with relative time and check if fix is old
	function formatAltitudeWithTime(
		altitudeMslFeet: number | null | undefined,
		timestamp: string
	): {
		altitudeText: string;
		isOld: boolean;
	} {
		const altitudeFt = altitudeMslFeet ? `${altitudeMslFeet}ft` : '---ft';

		// Calculate relative time, handling edge cases
		const fixTime = dayjs(timestamp);
		const now = dayjs();
		const diffSeconds = now.diff(fixTime, 'second');

		// If timestamp is in the future or within 10 seconds, show "just now"
		let relativeTimeText: string;
		if (diffSeconds >= -10 && diffSeconds <= 10) {
			relativeTimeText = 'just now';
		} else {
			relativeTimeText = fixTime.fromNow();
		}

		const altitudeText = `${altitudeFt} ${relativeTimeText}`;

		// Check if fix is more than 5 minutes old
		const diffMinutes = now.diff(fixTime, 'minute');
		const isOld = diffMinutes > 5;

		return { altitudeText, isOld };
	}

	// Save current map state to localStorage
	function saveMapState(): void {
		if (!map || !browser) return;

		const state: MapState = {
			center: {
				lat: map.getCenter()?.lat() || CONUS_CENTER.lat,
				lng: map.getCenter()?.lng() || CONUS_CENTER.lng
			},
			zoom: map.getZoom() || 4
		};

		try {
			localStorage.setItem(MAP_STATE_KEY, JSON.stringify(state));
			console.log('[MAP] Saved map state:', state);
		} catch (e) {
			console.warn('[MAP] Failed to save map state to localStorage:', e);
		}
	}

	// Load map state from URL params, localStorage, or fallback to CONUS center
	function loadMapState(): MapState {
		// First check URL parameters
		if (browser) {
			const params = $page.url.searchParams;
			const lat = params.get('lat');
			const lng = params.get('lng');
			const zoom = params.get('zoom');

			if (lat && lng) {
				const parsedLat = parseFloat(lat);
				const parsedLng = parseFloat(lng);
				const parsedZoom = zoom ? parseInt(zoom, 10) : 13;

				if (!isNaN(parsedLat) && !isNaN(parsedLng) && !isNaN(parsedZoom)) {
					console.log('[MAP] Using URL parameters:', {
						lat: parsedLat,
						lng: parsedLng,
						zoom: parsedZoom
					});
					return { center: { lat: parsedLat, lng: parsedLng }, zoom: parsedZoom };
				}
			}
		}

		// Fall back to localStorage
		if (!browser) {
			return { center: CONUS_CENTER, zoom: 4 };
		}

		try {
			const saved = localStorage.getItem(MAP_STATE_KEY);
			if (saved) {
				const state: MapState = JSON.parse(saved);
				console.log('[MAP] Loaded saved map state:', state);
				return state;
			}
		} catch (e) {
			console.warn('[MAP] Failed to load map state from localStorage:', e);
		}

		console.log('[MAP] Using default CONUS center');
		return { center: CONUS_CENTER, zoom: 4 };
	}

	// Save area tracker state to localStorage
	function saveAreaTrackerState(): void {
		if (!browser) return;

		try {
			localStorage.setItem(AREA_TRACKER_KEY, JSON.stringify(areaTrackerActive));
			console.log('[AREA TRACKER] Saved state:', areaTrackerActive);
		} catch (e) {
			console.warn('[AREA TRACKER] Failed to save state to localStorage:', e);
		}
	}

	// Load area tracker state from localStorage
	function loadAreaTrackerState(): boolean {
		if (!browser) return true;

		// When limit is disabled, area tracker is always on
		if (!AREA_TRACKER_LIMIT_ENABLED) {
			console.log('[AREA TRACKER] Limit disabled, area tracker always on');
			return true;
		}

		try {
			const saved = localStorage.getItem(AREA_TRACKER_KEY);
			if (saved !== null) {
				const state = JSON.parse(saved);
				console.log('[AREA TRACKER] Loaded saved state:', state);
				return state;
			}
		} catch (e) {
			console.warn('[AREA TRACKER] Failed to load state from localStorage:', e);
		}

		console.log('[AREA TRACKER] Using default state: true');
		return true;
	}

	// Save map type to localStorage
	function saveMapType(): void {
		if (!browser) return;

		try {
			localStorage.setItem(MAP_TYPE_KEY, mapType);
			console.log('[MAP TYPE] Saved map type:', mapType);
		} catch (e) {
			console.warn('[MAP TYPE] Failed to save map type to localStorage:', e);
		}
	}

	// Load map type from localStorage
	function loadMapType(): 'satellite' | 'roadmap' {
		if (!browser) return 'satellite';

		try {
			const saved = localStorage.getItem(MAP_TYPE_KEY);
			if (saved === 'satellite' || saved === 'roadmap') {
				console.log('[MAP TYPE] Loaded saved map type:', saved);
				return saved;
			}
		} catch (e) {
			console.warn('[MAP TYPE] Failed to load map type from localStorage:', e);
		}

		console.log('[MAP TYPE] Using default: satellite');
		return 'satellite';
	}

	// Toggle between map types
	function toggleMapType(): void {
		if (!map) return;

		mapType = mapType === 'satellite' ? 'roadmap' : 'satellite';
		map.setMapTypeId(
			mapType === 'satellite' ? google.maps.MapTypeId.SATELLITE : google.maps.MapTypeId.ROADMAP
		);
		saveMapType();
		console.log('[MAP TYPE] Toggled to:', mapType);
	}

	// Reactive effects for settings changes
	$effect(() => {
		if (!currentSettings.showAirportMarkers && shouldShowAirports) {
			clearAirportMarkers();
			airports = [];
			shouldShowAirports = false;
		} else if (currentSettings.showAirportMarkers && map) {
			// Re-check if we should show airports
			checkAndUpdateAirports();
		}
	});

	$effect(() => {
		if (!currentSettings.showReceiverMarkers && shouldShowReceivers) {
			clearReceiverMarkers();
			receivers = [];
			shouldShowReceivers = false;
		} else if (currentSettings.showReceiverMarkers && map) {
			// Re-check if we should show receivers
			checkAndUpdateReceivers();
		}
	});

	$effect(() => {
		if (!currentSettings.showAirspaceMarkers && shouldShowAirspaces) {
			clearAirspacePolygons();
			shouldShowAirspaces = false;
		} else if (currentSettings.showAirspaceMarkers && map) {
			// Re-check if we should show airspaces
			checkAndUpdateAirspaces();
		}
	});

	// Get singleton instances
	const aircraftRegistry = AircraftRegistry.getInstance();
	const fixFeed = FixFeed.getInstance();

	// Subscribe to device registry and update aircraft markers
	let activeDevices: Aircraft[] = $state([]);
	let initialMarkersRendered = false;

	// Area tracker state
	let areaTrackerActive = $state(false);
	let areaTrackerAvailable = $state(true); // Whether area tracker can be enabled (based on map area)
	let currentAreaSubscriptions = new SvelteSet<string>(); // Track subscribed areas

	// Clustering state
	let isClusteredMode = $state(false); // Whether we're currently displaying clusters instead of individual aircraft
	let clusterRefreshTimer: ReturnType<typeof setInterval> | null = null;

	// Map type state
	let mapType = $state<'satellite' | 'roadmap'>('satellite');

	$effect(() => {
		const unsubscribeRegistry = aircraftRegistry.subscribe((event: AircraftRegistryEvent) => {
			// IMPORTANT: Ignore all aircraft registry events when in clustered mode
			// In clustered mode, we only show cluster markers, not individual aircraft
			if (isClusteredMode) {
				console.log('[CLUSTERED MODE] Ignoring aircraft registry event:', event.type);
				return;
			}

			if (event.type === 'aircraft_changed') {
				activeDevices = event.aircraft;
			} else if (event.type === 'aircraft_updated') {
				// When a device is updated, create or update its marker if we have a map
				if (map) {
					updateAircraftMarkerFromAircraft(event.aircraft);
				}
			} else if (event.type === 'fix_added') {
				console.log('Fix added to aircraft:', event.aircraft.id, event.fix);
				// Update the aircraft marker immediately when a new fix is added
				if (map && event.fix) {
					updateAircraftMarkerFromDevice(event.aircraft, event.fix);
				}
			}
		});

		// Initialize active aircraft
		activeDevices = aircraftRegistry.getAllAircraft();

		return () => {
			unsubscribeRegistry();
		};
	});

	// Effect to initialize aircraft markers when map becomes available (runs once)
	$effect(() => {
		if (!map || initialMarkersRendered) return;

		// When map first becomes available, render markers for all active aircraft
		console.log(
			'[EFFECT] Map available, initializing markers for',
			activeDevices.length,
			'aircraft'
		);
		activeDevices.forEach((aircraft) => {
			updateAircraftMarkerFromAircraft(aircraft);
		});

		initialMarkersRendered = true;
	});

	onMount(() => {
		(async () => {
			await loadGoogleMapsScript();
			initializeMap();
			initializeCompass();
			// Start live fixes feed for operations page
			fixFeed.startLiveFixesFeed();

			// Load area tracker state and apply it after map is initialized
			const savedAreaTrackerState = loadAreaTrackerState();
			if (savedAreaTrackerState && areaTrackerAvailable) {
				// Defer activation until map is fully initialized
				setTimeout(() => {
					if (areaTrackerAvailable) {
						areaTrackerActive = savedAreaTrackerState;
						if (areaTrackerActive) {
							// Activate area tracker with saved state
							// Fetch initial aircraft with latest positions only
							(async () => {
								await fetchAndDisplayDevicesInViewport();
								updateAreaSubscriptions();
							})();
						}
					}
				}, 1500);
			}
		})();

		// Cleanup function
		return () => {
			fixFeed.stopLiveFixesFeed();
			clearAircraftMarkers();
			stopClusterRefreshTimer();
		};
	});

	async function loadGoogleMapsScript(): Promise<void> {
		setOptions({
			key: GOOGLE_MAPS_API_KEY,
			v: 'weekly'
		});

		// Import the required libraries
		await importLibrary('maps');
		await importLibrary('geometry');
		await importLibrary('marker');
	}

	function initializeMap(): void {
		console.log('[MAP] Initializing Google Maps:', {
			hasContainer: !!mapContainer,
			hasGoogle: !!window.google
		});

		if (!mapContainer || !window.google) {
			console.error('[MAP] Missing requirements for map initialization');
			return;
		}

		// Load saved map state or use continental US as fallback
		const mapState = loadMapState();

		// Load saved map type preference
		mapType = loadMapType();

		// Initialize map with saved or default state
		map = new google.maps.Map(mapContainer, {
			mapId: 'SOAR_MAP', // Required for AdvancedMarkerElement
			center: mapState.center,
			zoom: mapState.zoom,
			mapTypeId:
				mapType === 'satellite'
					? window.google.maps.MapTypeId.SATELLITE
					: window.google.maps.MapTypeId.ROADMAP,
			mapTypeControl: false, // We'll use a custom toggle button
			zoomControl: false,
			scaleControl: true,
			streetViewControl: false,
			fullscreenControl: false,
			gestureHandling: 'greedy' // Allow one-finger gestures on mobile
		});

		// Add event listeners for viewport changes
		map.addListener('zoom_changed', () => {
			setTimeout(checkAndUpdateAirports, 100); // Small delay to ensure bounds are updated
			setTimeout(checkAndUpdateReceivers, 100); // Check receivers as well
			setTimeout(checkAndUpdateAirspaces, 100); // Check airspaces as well
			// Update aircraft marker scaling on zoom change
			updateAllAircraftMarkersScale();
			// Update area tracker availability
			updateAreaTrackerAvailability();

			// Clear any existing debounce timer
			if (zoomDebounceTimer) {
				clearTimeout(zoomDebounceTimer);
			}

			// Debounce aircraft fetching by 1 second after zoom stops
			zoomDebounceTimer = setTimeout(async () => {
				// Always fetch aircraft in viewport to check for clustering
				// This ensures clustering activates even when area tracker is off
				await fetchAndDisplayDevicesInViewport();

				// Update WebSocket subscriptions only if area tracker is active
				if (areaTrackerActive) {
					updateAreaSubscriptions();
				}

				zoomDebounceTimer = null;
			}, 1000); // Wait 1 second after zoom stops

			// Save map state after zoom changes
			saveMapState();
		});

		// Initial aircraft fetch after map is ready (even if area tracker is off)
		// This ensures clustering works on page load
		setTimeout(async () => {
			await fetchAndDisplayDevicesInViewport();
			if (areaTrackerActive) {
				updateAreaSubscriptions();
			}
		}, 500);

		map.addListener('dragend', async () => {
			checkAndUpdateAirports();
			checkAndUpdateReceivers();
			checkAndUpdateAirspaces();

			// Always fetch aircraft in viewport after panning
			await fetchAndDisplayDevicesInViewport();

			// Update WebSocket subscriptions only if area tracker is active
			if (areaTrackerActive) {
				updateAreaSubscriptions();
			}

			// Save map state after panning
			saveMapState();
		});

		// Initial check for airports, receivers, and airspaces
		setTimeout(checkAndUpdateAirports, 1000); // Give map time to fully initialize
		setTimeout(checkAndUpdateReceivers, 1000);
		setTimeout(checkAndUpdateAirspaces, 1000);

		// Initial area tracker availability check
		setTimeout(updateAreaTrackerAvailability, 1000);

		console.log('[MAP] Google Maps initialized for operations view. Map ready for markers.');
	}

	async function locateUser(): Promise<void> {
		if (!navigator.geolocation) {
			alert('Geolocation is not supported by this browser.');
			return;
		}

		isLocating = true;

		try {
			const position = await getCurrentPosition();
			const userLocation = {
				lat: position.coords.latitude,
				lng: position.coords.longitude
			};

			console.log(`User location found: ${userLocation.lat}, ${userLocation.lng}`);

			// Step 1: First place the marker at user location
			// Create a custom content element for the marker
			const markerContent = document.createElement('div');
			markerContent.innerHTML = `
				<div style="
					width: 24px;
					height: 24px;
					background: #4285F4;
					border: 2px solid #FFFFFF;
					border-radius: 50%;
					position: relative;
					box-shadow: 0 2px 6px rgba(0,0,0,0.3);
				">
					<div style="
						width: 6px;
						height: 6px;
						background: #FFFFFF;
						border-radius: 50%;
						position: absolute;
						top: 50%;
						left: 50%;
						transform: translate(-50%, -50%);
					"></div>
				</div>
			`;

			// Remove existing marker if present
			if (userMarker) {
				userMarker.map = null;
			}

			userMarker = new google.maps.marker.AdvancedMarkerElement({
				position: userLocation,
				map: map,
				title: 'Your Location',
				content: markerContent
			});

			// Step 2: Animate pan to user location
			if (map) {
				map.panTo(userLocation);

				// Step 3: Wait for pan animation to complete, then zoom in smoothly
				setTimeout(() => {
					// Smooth zoom animation to show approximately 10-mile radius
					const targetZoom = 13;
					const currentZoom = map.getZoom() || 4;

					// Animate zoom gradually for smoother transition
					animateZoom(currentZoom, targetZoom);
				}, 1000); // Wait for pan animation to complete
			}

			console.log(`User located and animated to: ${userLocation.lat}, ${userLocation.lng}`);
		} catch (error) {
			console.error('Error getting user location:', error);
			alert('Unable to get your location. Please make sure location services are enabled.');
		} finally {
			isLocating = false;
		}
	}

	function animateZoom(currentZoom: number, targetZoom: number): void {
		if (!map || currentZoom >= targetZoom) return;

		const zoomStep = Math.min(1, targetZoom - currentZoom);
		const nextZoom = currentZoom + zoomStep;

		map.setZoom(nextZoom);

		if (nextZoom < targetZoom) {
			setTimeout(() => animateZoom(nextZoom, targetZoom), 200);
		}
	}

	function getCurrentPosition(): Promise<GeolocationPosition> {
		return new Promise((resolve, reject) => {
			navigator.geolocation.getCurrentPosition(resolve, reject, {
				enableHighAccuracy: true,
				timeout: 10000,
				maximumAge: 300000 // 5 minutes
			});
		});
	}

	async function initializeCompass(): Promise<void> {
		// Check if we need to request permission (for iOS 13+)
		if (
			'requestPermission' in DeviceOrientationEvent &&
			typeof DeviceOrientationEvent.requestPermission === 'function'
		) {
			try {
				const permission = await DeviceOrientationEvent.requestPermission();
				if (permission !== 'granted') {
					console.log('Device orientation permission denied');
					return;
				}
			} catch (error) {
				console.log('Error requesting device orientation permission:', error);
				return;
			}
		}

		// Try to lock screen orientation
		if ('screen' in window && 'orientation' in window.screen) {
			try {
				// Type assertion for screen orientation API
				const orientation = window.screen.orientation as ScreenOrientation & {
					lock?: (orientation: string) => Promise<void>;
				};
				if (orientation && typeof orientation.lock === 'function') {
					await orientation.lock('portrait-primary');
					console.log('Screen orientation locked to portrait');
				}
			} catch (error) {
				console.log('Could not lock screen orientation:', error);
			}
		}

		window.addEventListener('deviceorientation', handleOrientationChange);
	}

	function calculateViewportArea(): number {
		if (!map) return 0;

		const bounds = map.getBounds();
		if (!bounds) return 0;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		// Calculate area using spherical geometry
		// This gives us area in square meters, convert to square miles
		const areaSquareMeters = google.maps.geometry.spherical.computeArea([
			new google.maps.LatLng(sw.lat(), sw.lng()),
			new google.maps.LatLng(ne.lat(), sw.lng()),
			new google.maps.LatLng(ne.lat(), ne.lng()),
			new google.maps.LatLng(sw.lat(), ne.lng())
		]);

		// Convert square meters to square miles (1 square mile = 2,589,988 square meters)
		const areaSquareMiles = areaSquareMeters / 2589988;
		console.log(`Viewport area: ${areaSquareMiles.toFixed(2)} square miles`);
		return areaSquareMiles;
	}

	async function fetchAirportsInViewport(): Promise<void> {
		if (!map) return;

		const bounds = map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		// Validate bounding box coordinates
		const nwLat = ne.lat();
		const nwLng = sw.lng();
		const seLat = sw.lat();
		const seLng = ne.lng();

		// Ensure northwest latitude is greater than southeast latitude
		if (nwLat <= seLat) {
			console.warn(
				'Invalid bounding box: northwest latitude must be greater than southeast latitude'
			);
			return;
		}

		// Validate latitude bounds
		if (nwLat > 90 || nwLat < -90 || seLat > 90 || seLat < -90) {
			console.warn('Invalid latitude values in bounding box');
			return;
		}

		// Validate longitude bounds (allow wrapping around international date line)
		if (nwLng < -180 || nwLng > 180 || seLng < -180 || seLng > 180) {
			console.warn('Invalid longitude values in bounding box');
			return;
		}

		try {
			const params = new URLSearchParams({
				north: nwLat.toString(),
				west: nwLng.toString(),
				south: seLat.toString(),
				east: seLng.toString(),
				limit: '100' // Limit to avoid too many markers
			});

			const response = await serverCall<DataListResponse<Airport>>(`/airports?${params}`);
			airports = response.data || [];

			displayAirportsOnMap();
		} catch (error) {
			console.error('Error fetching airports:', error);
		}
	}

	function displayAirportsOnMap(): void {
		// Clear existing airport markers
		clearAirportMarkers();

		airports.forEach((airport) => {
			if (!airport.latitudeDeg || !airport.longitudeDeg) return;

			// Convert BigDecimal strings to numbers with validation
			const lat = parseFloat(airport.latitudeDeg);
			const lng = parseFloat(airport.longitudeDeg);

			// Validate coordinates are valid numbers and within expected ranges
			if (isNaN(lat) || isNaN(lng) || lat < -90 || lat > 90 || lng < -180 || lng > 180) {
				console.warn(`Invalid coordinates for airport ${airport.ident}: ${lat}, ${lng}`);
				return;
			}

			// Create marker content with proper escaping
			const markerContent = document.createElement('div');
			markerContent.className = 'airport-marker';

			const iconDiv = document.createElement('div');
			iconDiv.className = 'airport-icon';
			iconDiv.textContent = '✈';

			const labelDiv = document.createElement('div');
			labelDiv.className = 'airport-label';
			labelDiv.textContent = airport.ident;

			markerContent.appendChild(iconDiv);
			markerContent.appendChild(labelDiv);

			const marker = new google.maps.marker.AdvancedMarkerElement({
				position: { lat, lng },
				map: map,
				title: `${airport.name} (${airport.ident})`,
				content: markerContent,
				zIndex: 100 // Lower z-index for airports so aircraft appear on top
			});

			// Add click listener to open airport modal
			marker.addListener('click', () => {
				selectedAirport = airport;
				showAirportModal = true;
			});

			airportMarkers.push(marker);
		});
	}

	function clearAirportMarkers(): void {
		airportMarkers.forEach((marker) => {
			marker.map = null;
		});
		airportMarkers = [];
	}

	async function fetchReceiversInViewport(): Promise<void> {
		if (!map) return;

		const bounds = map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		// Validate bounding box coordinates
		const nwLat = ne.lat();
		const nwLng = sw.lng();
		const seLat = sw.lat();
		const seLng = ne.lng();

		// Ensure northwest latitude is greater than southeast latitude
		if (nwLat <= seLat) {
			console.warn(
				'Invalid bounding box: northwest latitude must be greater than southeast latitude'
			);
			return;
		}

		// Validate latitude bounds
		if (nwLat > 90 || nwLat < -90 || seLat > 90 || seLat < -90) {
			console.warn('Invalid latitude values in bounding box');
			return;
		}

		// Validate longitude bounds
		if (nwLng < -180 || nwLng > 180 || seLng < -180 || seLng > 180) {
			console.warn('Invalid longitude values in bounding box');
			return;
		}

		try {
			const params = new URLSearchParams({
				latitude_min: seLat.toString(),
				latitude_max: nwLat.toString(),
				longitude_min: nwLng.toString(),
				longitude_max: seLng.toString()
			});

			const response = await serverCall<DataListResponse<Receiver>>(`/receivers?${params}`);
			receivers = response.data || [];

			displayReceiversOnMap();
		} catch (error) {
			console.error('Error fetching receivers:', error);
		}
	}

	function displayReceiversOnMap(): void {
		// Clear existing receiver markers
		clearReceiverMarkers();

		receivers.forEach((receiver) => {
			if (!receiver.latitude || !receiver.longitude) return;

			// Validate coordinates are valid numbers and within expected ranges
			if (
				isNaN(receiver.latitude) ||
				isNaN(receiver.longitude) ||
				receiver.latitude < -90 ||
				receiver.latitude > 90 ||
				receiver.longitude < -180 ||
				receiver.longitude > 180
			) {
				console.warn(
					`Invalid coordinates for receiver ${receiver.callsign}: ${receiver.latitude}, ${receiver.longitude}`
				);
				return;
			}

			// Create marker content with Radio icon and link
			const markerLink = document.createElement('a');
			markerLink.href = `/receivers/${receiver.id}`;
			markerLink.target = '_blank';
			markerLink.rel = 'noopener noreferrer';
			markerLink.className = 'receiver-marker';

			const iconDiv = document.createElement('div');
			iconDiv.className = 'receiver-icon';
			// Create SVG for Radio icon (antenna symbol)
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
				zIndex: 150 // Between airports (100) and aircraft (1000)
			});

			receiverMarkers.push(marker);
		});
	}

	function clearReceiverMarkers(): void {
		receiverMarkers.forEach((marker) => {
			marker.map = null;
		});
		receiverMarkers = [];
	}

	function checkAndUpdateAirports(): void {
		// Clear any existing debounce timer
		if (airportUpdateDebounceTimer !== null) {
			clearTimeout(airportUpdateDebounceTimer);
		}

		// Debounce airport updates by 100ms to prevent excessive API calls
		airportUpdateDebounceTimer = setTimeout(() => {
			const area = calculateViewportArea();
			const shouldShow = area < 10000 && currentSettings.showAirportMarkers;

			if (shouldShow !== shouldShowAirports) {
				shouldShowAirports = shouldShow;

				if (shouldShowAirports) {
					fetchAirportsInViewport();
				} else {
					clearAirportMarkers();
					airports = [];
				}
			} else if (shouldShowAirports) {
				// Still showing airports, update them for the new viewport
				fetchAirportsInViewport();
			}

			airportUpdateDebounceTimer = null;
		}, 100);
	}

	function checkAndUpdateReceivers(): void {
		// Clear any existing debounce timer
		if (receiverUpdateDebounceTimer !== null) {
			clearTimeout(receiverUpdateDebounceTimer);
		}

		// Debounce receiver updates by 100ms to prevent excessive API calls
		receiverUpdateDebounceTimer = setTimeout(() => {
			const area = calculateViewportArea();
			const shouldShow = area < 10000 && currentSettings.showReceiverMarkers;

			if (shouldShow !== shouldShowReceivers) {
				shouldShowReceivers = shouldShow;

				if (shouldShowReceivers) {
					fetchReceiversInViewport();
				} else {
					clearReceiverMarkers();
					receivers = [];
				}
			} else if (shouldShowReceivers) {
				// Still showing receivers, update them for the new viewport
				fetchReceiversInViewport();
			}

			receiverUpdateDebounceTimer = null;
		}, 100);
	}

	// Airspace functions
	function getAirspaceColor(airspaceClass: string | null): string {
		switch (airspaceClass) {
			case 'A':
			case 'B':
			case 'C':
			case 'D':
				return '#DC2626'; // Red - Controlled airspace
			case 'E':
				return '#F59E0B'; // Amber - Class E
			case 'F':
			case 'G':
				return '#10B981'; // Green - Uncontrolled
			default:
				return '#6B7280'; // Gray - Other/SUA
		}
	}

	async function fetchAirspacesInViewport(): Promise<void> {
		if (!map) return;

		const bounds = map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		try {
			const params = new URLSearchParams({
				west: sw.lng().toString(),
				south: sw.lat().toString(),
				east: ne.lng().toString(),
				north: ne.lat().toString(),
				limit: '500'
			});

			const response = await serverCall<DataResponse<AirspaceFeatureCollection>>(
				`/airspaces?${params}`
			);
			const data = response.data;

			if (data && data.type === 'FeatureCollection' && Array.isArray(data.features)) {
				displayAirspacesOnMap(data.features);
			}
		} catch (error) {
			console.error('Error fetching airspaces:', error);
		}
	}

	function displayAirspacesOnMap(airspaces: Airspace[]): void {
		// Clear existing airspace polygons
		clearAirspacePolygons();

		airspaces.forEach((airspace) => {
			const color = getAirspaceColor(airspace.properties.airspaceClass);

			// Convert GeoJSON coordinates to Google Maps LatLng format
			const paths: google.maps.LatLngLiteral[][] = [];

			if (airspace.geometry.type === 'Polygon') {
				// Single polygon: coordinates is number[][][]
				const coords = airspace.geometry.coordinates as number[][][];
				coords.forEach((ring) => {
					const path = ring.map((coord) => ({ lat: coord[1], lng: coord[0] }));
					paths.push(path);
				});
			} else if (airspace.geometry.type === 'MultiPolygon') {
				// MultiPolygon: coordinates is number[][][][]
				const coords = airspace.geometry.coordinates as number[][][][];
				coords.forEach((polygon) => {
					polygon.forEach((ring) => {
						const path = ring.map((coord) => ({ lat: coord[1], lng: coord[0] }));
						paths.push(path);
					});
				});
			}

			// Create polygon for each path
			paths.forEach((path) => {
				const polygon = new google.maps.Polygon({
					paths: path,
					strokeColor: color,
					strokeOpacity: 0.8,
					strokeWeight: 2,
					fillColor: color,
					fillOpacity: 0.15,
					map: map,
					zIndex: 50 // Below airports (100) and receivers (150)
				});

				// Add click listener to show airspace modal
				polygon.addListener('click', () => {
					selectedAirspace = airspace;
					showAirspaceModal = true;
				});

				airspacePolygons.push(polygon);
			});
		});

		console.log(
			`[AIRSPACES] Displayed ${airspaces.length} airspaces (${airspacePolygons.length} polygons)`
		);
	}

	function clearAirspacePolygons(): void {
		airspacePolygons.forEach((polygon) => {
			polygon.setMap(null);
		});
		airspacePolygons = [];
	}

	function checkAndUpdateAirspaces(): void {
		// Clear any existing debounce timer
		if (airspaceUpdateDebounceTimer !== null) {
			clearTimeout(airspaceUpdateDebounceTimer);
		}

		// Debounce airspace updates by 500ms to prevent excessive API calls
		airspaceUpdateDebounceTimer = setTimeout(() => {
			const area = calculateViewportArea();
			const shouldShow = area < 100000 && currentSettings.showAirspaceMarkers;

			if (shouldShow !== shouldShowAirspaces) {
				shouldShowAirspaces = shouldShow;

				if (shouldShowAirspaces) {
					fetchAirspacesInViewport();
				} else {
					clearAirspacePolygons();
				}
			} else if (shouldShowAirspaces) {
				// Still showing airspaces, update them for the new viewport
				fetchAirspacesInViewport();
			}

			airspaceUpdateDebounceTimer = null;
		}, 500);
	}

	function handleOrientationChange(event: DeviceOrientationEvent): void {
		if (event.alpha !== null) {
			isCompassActive = true;
			// Store the raw device heading
			aircraftHeading = event.alpha;
			displayHeading = Math.round(aircraftHeading);

			// Use aircraftHeading directly (not inverted) to keep north arrow pointing north
			// When phone rotates clockwise (alpha increases), we rotate compass counter-clockwise
			// by using the heading value directly in the CSS transform
			let newHeading = aircraftHeading;

			// Normalize to 0-360 range
			newHeading = ((newHeading % 360) + 360) % 360;

			// Calculate the shortest rotation path to avoid spinning around unnecessarily
			// If the difference is greater than 180°, we should wrap around
			let delta = newHeading - previousCompassHeading;

			// Adjust for boundary crossing to take the shortest path
			if (delta > 180) {
				// Crossed from high to low (e.g., 350° to 10°)
				// Add a full rotation to previousCompassHeading conceptually
				compassHeading = newHeading - 360;
			} else if (delta < -180) {
				// Crossed from low to high (e.g., 10° to 350°)
				// Add a full rotation to newHeading
				compassHeading = newHeading + 360;
			} else {
				// Normal case, no boundary crossing
				compassHeading = newHeading;
			}

			// Update previous heading to track actual compassHeading (not normalized)
			// This maintains continuity across boundary crossings
			previousCompassHeading = compassHeading;
		}
	}

	function updateMarkerScale(markerContent: HTMLElement, zoom: number): void {
		if (!markerContent) return;

		// Calculate scale based on zoom level
		// Zoom levels typically range from 1 (world) to 20+ (street level)
		// Keep markers small even when zoomed in to avoid clutter
		let scale: number;

		if (zoom <= 4) {
			// Very zoomed out (world/continental view) - minimum size
			scale = 0.3;
		} else if (zoom <= 8) {
			// Country/state level - small size
			scale = 0.4 + (zoom - 4) * 0.1; // 0.4 to 0.8
		} else if (zoom <= 12) {
			// Regional level - keep compact
			scale = 0.8 + (zoom - 8) * 0.025; // 0.8 to 0.9
		} else {
			// City/street level - maximum but still compact
			scale = 0.9 + Math.min(zoom - 12, 6) * 0.0167; // 0.9 to 1.0 max
		}

		// Apply transform to the entire marker content
		markerContent.style.transform = `scale(${scale})`;
		markerContent.style.transformOrigin = 'center bottom'; // Anchor at bottom center
	}

	function updateAllAircraftMarkersScale(): void {
		if (!map) return;

		const currentZoom = map.getZoom() || 4;
		aircraftMarkers.forEach((marker) => {
			const markerContent = marker.content as HTMLElement;
			if (markerContent) {
				updateMarkerScale(markerContent, currentZoom);
			}
		});
	}

	// Update aircraft marker using latest position from aircraft object or fixes
	function updateAircraftMarkerFromAircraft(aircraft: Aircraft): void {
		if (!map) return;

		// Use currentFix if available (it's a full Fix object stored as JSONB)
		if (aircraft.currentFix) {
			const currentFix = aircraft.currentFix as unknown as Fix;
			updateAircraftMarkerFromDevice(aircraft, currentFix);
		} else {
			// Fallback to using fixes array if present
			const fixes = aircraft.fixes || [];
			const latestFix = fixes.length > 0 ? fixes[0] : null;
			if (latestFix) {
				updateAircraftMarkerFromDevice(aircraft, latestFix);
			} else {
				console.log('[MARKER] No position data available for aircraft:', aircraft.id);
			}
		}
	}

	function updateAircraftMarkerFromDevice(aircraft: Aircraft, latestFix: Fix): void {
		console.log('[MARKER] updateAircraftMarkerFromDevice called:', {
			deviceId: aircraft.id,
			registration: aircraft.registration,
			latestFix: {
				lat: latestFix.latitude,
				lng: latestFix.longitude,
				alt: latestFix.altitudeMslFeet,
				timestamp: latestFix.timestamp
			},
			mapExists: !!map
		});

		if (!map) {
			console.warn('[MARKER] No map available for marker update');
			return;
		}

		const aircraftKey = aircraft.id;
		if (!aircraftKey) {
			console.warn('[MARKER] No device ID available');
			return;
		}

		// Update latest fix for this device
		latestFixes.set(aircraftKey, latestFix);
		console.log('[MARKER] Updated latest fix for aircraft:', aircraftKey);

		// Get or create marker for this aircraft
		let marker = aircraftMarkers.get(aircraftKey);

		if (!marker) {
			console.log('[MARKER] Creating new marker for aircraft:', aircraftKey);
			// Create new aircraft marker with device info
			marker = createAircraftMarkerFromDevice(aircraft, latestFix);
			aircraftMarkers.set(aircraftKey, marker);
			console.log('[MARKER] New marker created and stored. Total markers:', aircraftMarkers.size);
		} else {
			console.log('[MARKER] Updating existing marker for aircraft:', aircraftKey);
			// Update existing marker position and info
			updateAircraftMarkerPositionFromDevice(marker, aircraft, latestFix);
		}

		// Update trail for this aircraft
		updateAircraftTrail(aircraft);
	}

	function createAircraftMarkerFromDevice(
		aircraft: Aircraft,
		fix: Fix
	): google.maps.marker.AdvancedMarkerElement {
		console.log('[MARKER] Creating marker for aircraft:', {
			deviceId: aircraft.id,
			registration: aircraft.registration,
			address: aircraft.address,
			position: { lat: fix.latitude, lng: fix.longitude },
			track: fix.trackDegrees
		});

		// Create aircraft icon with rotation based on track
		const markerContent = document.createElement('div');
		markerContent.className = 'aircraft-marker';

		// Aircraft icon (rotated based on track) - using a more visible SVG plane
		const aircraftIcon = document.createElement('div');
		aircraftIcon.className = 'aircraft-icon';

		// Calculate color based on active status and altitude
		const markerColor = getMarkerColor(fix);
		aircraftIcon.style.background = markerColor;

		// Create SVG airplane icon that's more visible and oriented correctly
		aircraftIcon.innerHTML = `
			<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
				<path d="M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z"/>
			</svg>
		`;

		// Rotate icon based on track degrees (default to 0 if not available)
		const track = fix.trackDegrees || 0;
		aircraftIcon.style.transform = `rotate(${track}deg)`;
		console.log('[MARKER] Set icon rotation to:', track, 'degrees');

		// Info label below the icon - show proper aircraft information
		const infoLabel = document.createElement('div');
		infoLabel.className = 'aircraft-label';
		infoLabel.style.background = markerColor.replace('rgb', 'rgba').replace(')', ', 0.75)'); // 75% opacity
		infoLabel.style.borderColor = markerColor;

		// Use proper device registration, fallback to address
		const tailNumber = aircraft.registration || aircraft.address || 'Unknown';
		const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
		// Use aircraftModel string from device, or detailed model name if available
		const aircraftModel = aircraft.aircraftModel || null;

		console.log('[MARKER] Aircraft info:', {
			tailNumber,
			altitude: altitudeText,
			model: aircraftModel,
			isOld
		});

		// Create label with tail number + model (if available) on top, altitude on bottom
		const tailDiv = document.createElement('div');
		tailDiv.className = 'aircraft-tail';
		// Include aircraft model after tail number if available
		tailDiv.textContent = aircraftModel ? `${tailNumber} (${aircraftModel})` : tailNumber;

		const altDiv = document.createElement('div');
		altDiv.className = 'aircraft-altitude';
		altDiv.textContent = altitudeText;

		// Apply transparency if fix is old (>5 minutes)
		if (isOld) {
			aircraftIcon.style.opacity = '0.5';
			tailDiv.style.opacity = '0.5';
			altDiv.style.opacity = '0.5';
		}

		infoLabel.appendChild(tailDiv);
		infoLabel.appendChild(altDiv);

		markerContent.appendChild(aircraftIcon);
		markerContent.appendChild(infoLabel);

		// Create the marker with proper title including aircraft model and full timestamp
		const fullTimestamp = dayjs(fix.timestamp).format('YYYY-MM-DD HH:mm:ss UTC');
		const title = aircraft.aircraftModel
			? `${tailNumber} (${aircraft.aircraftModel}) - ${altitudeText} - Last seen: ${fullTimestamp}`
			: `${tailNumber} - ${altitudeText} - Last seen: ${fullTimestamp}`;

		console.log('[MARKER] Creating AdvancedMarkerElement with:', {
			position: { lat: fix.latitude, lng: fix.longitude },
			title,
			hasContent: !!markerContent
		});

		const marker = new google.maps.marker.AdvancedMarkerElement({
			position: { lat: fix.latitude, lng: fix.longitude },
			map: map,
			title: title,
			content: markerContent,
			zIndex: 1000 // Higher z-index for aircraft to appear on top of airports
		});

		// Add click event listener to open aircraft status modal
		marker.addListener('click', () => {
			handleAircraftClick(aircraft);
		});

		// Add hover listeners to bring marker to front when overlapping with other aircraft
		markerContent.addEventListener('mouseenter', () => {
			marker.zIndex = 10000; // Bring to front on hover
		});

		markerContent.addEventListener('mouseleave', () => {
			marker.zIndex = 1000; // Return to normal z-index
		});

		// Apply initial zoom-based scaling
		updateMarkerScale(markerContent, map.getZoom() || 4);

		console.log('[MARKER] AdvancedMarkerElement created successfully');
		return marker;
	}

	function updateAircraftMarkerPositionFromDevice(
		marker: google.maps.marker.AdvancedMarkerElement,
		aircraft: Aircraft,
		fix: Fix
	): void {
		console.log('[MARKER] Updating existing marker position:', {
			deviceId: aircraft.id,
			oldPosition: marker.position,
			newPosition: { lat: fix.latitude, lng: fix.longitude }
		});

		// Update position
		marker.position = { lat: fix.latitude, lng: fix.longitude };

		// Update icon rotation and label
		const markerContent = marker.content as HTMLElement;
		if (markerContent) {
			const aircraftIcon = markerContent.querySelector('.aircraft-icon') as HTMLElement;
			const infoLabel = markerContent.querySelector('.aircraft-label') as HTMLElement;
			const tailDiv = markerContent.querySelector('.aircraft-tail') as HTMLElement;
			const altDiv = markerContent.querySelector('.aircraft-altitude') as HTMLElement;

			// Calculate color based on active status and altitude
			const markerColor = getMarkerColor(fix);

			if (aircraftIcon) {
				const track = fix.trackDegrees || 0;
				aircraftIcon.style.transform = `rotate(${track}deg)`;
				aircraftIcon.style.background = markerColor;
				console.log('[MARKER] Updated icon rotation to:', track, 'degrees');
			}

			if (infoLabel) {
				infoLabel.style.background = markerColor.replace('rgb', 'rgba').replace(')', ', 0.75)');
				infoLabel.style.borderColor = markerColor;
			}

			if (tailDiv && altDiv) {
				// Use proper device registration, fallback to address
				const tailNumber = aircraft.registration || aircraft.address || 'Unknown';
				const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
				// Use aircraftModel string from device
				const aircraftModel = aircraft.aircraftModel || null;

				// Include aircraft model after tail number if available
				tailDiv.textContent = aircraftModel ? `${tailNumber} (${aircraftModel})` : tailNumber;
				altDiv.textContent = altitudeText;

				// Apply transparency if fix is old (>5 minutes)
				if (isOld) {
					aircraftIcon.style.opacity = '0.5';
					tailDiv.style.opacity = '0.5';
					altDiv.style.opacity = '0.5';
				} else {
					// Reset opacity for fresh fixes
					aircraftIcon.style.opacity = '1';
					tailDiv.style.opacity = '1';
					altDiv.style.opacity = '1';
				}

				console.log('[MARKER] Updated label info:', {
					tailNumber,
					altitudeText,
					aircraftModel,
					isOld
				});
			}
		} else {
			console.warn('[MARKER] No marker content found for position update');
		}

		// Update scaling for the current zoom level
		const currentZoom = map.getZoom() || 4;
		updateMarkerScale(markerContent, currentZoom);

		// Update the marker title with full timestamp
		const { altitudeText } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
		const fullTimestamp = dayjs(fix.timestamp).format('YYYY-MM-DD HH:mm:ss UTC');
		const title = aircraft.aircraftModel
			? `${aircraft.registration || aircraft.address} (${aircraft.aircraftModel}) - ${altitudeText} - Last seen: ${fullTimestamp}`
			: `${aircraft.registration || aircraft.address} - ${altitudeText} - Last seen: ${fullTimestamp}`;

		marker.title = title;
		console.log('[MARKER] Updated marker title:', title);
	}

	function clearAircraftMarkers(): void {
		console.log('[MARKER] Clearing all aircraft markers. Count:', aircraftMarkers.size);
		aircraftMarkers.forEach((marker) => {
			marker.map = null;
		});
		aircraftMarkers.clear();
		latestFixes.clear();
		clearAllTrails();
		console.log('[MARKER] All aircraft markers and trails cleared');
	}

	// Cluster marker functions
	function createClusterMarker(cluster: AircraftCluster): google.maps.marker.AdvancedMarkerElement {
		console.log('[CLUSTER] Creating cluster marker:', {
			id: cluster.id,
			position: { lat: cluster.latitude, lng: cluster.longitude },
			count: cluster.count
		});

		// Create polygon outline for the cluster bounds
		// DEBUG: Using bright red outline to visualize grid cells
		console.log('[CLUSTER DEBUG] Bounds:', {
			id: cluster.id,
			north: cluster.bounds.north,
			south: cluster.bounds.south,
			east: cluster.bounds.east,
			west: cluster.bounds.west,
			width: cluster.bounds.east - cluster.bounds.west,
			height: cluster.bounds.north - cluster.bounds.south
		});

		const polygon = new google.maps.Polygon({
			paths: [
				{ lat: cluster.bounds.north, lng: cluster.bounds.west },
				{ lat: cluster.bounds.north, lng: cluster.bounds.east },
				{ lat: cluster.bounds.south, lng: cluster.bounds.east },
				{ lat: cluster.bounds.south, lng: cluster.bounds.west }
			],
			strokeColor: '#FF0000', // DEBUG: Bright red
			strokeOpacity: 1.0, // DEBUG: Fully opaque
			strokeWeight: 4, // DEBUG: Thick outline
			fillColor: '#FF0000', // DEBUG: Red fill
			fillOpacity: 0.1,
			map: map,
			zIndex: 400
		});

		// Store the polygon for later cleanup
		clusterPolygons.set(cluster.id, polygon);

		// Add click listener to polygon
		polygon.addListener('click', () => {
			handleClusterClick(cluster);
		});

		// Create label marker at centroid - no solid background, just text with shadow for visibility
		const markerContent = document.createElement('div');
		markerContent.className = 'cluster-label';
		markerContent.style.display = 'flex';
		markerContent.style.flexDirection = 'column';
		markerContent.style.alignItems = 'center';
		markerContent.style.justifyContent = 'center';
		markerContent.style.gap = '2px';
		markerContent.style.cursor = 'pointer';
		markerContent.style.pointerEvents = 'auto';
		markerContent.style.position = 'relative';

		// Airplane SVG icon with white fill and shadow - SMALLER
		const iconDiv = document.createElement('div');
		iconDiv.style.display = 'flex';
		iconDiv.style.alignItems = 'center';
		iconDiv.style.justifyContent = 'center';
		iconDiv.style.filter = 'drop-shadow(0 2px 4px rgba(0, 0, 0, 0.8))';
		iconDiv.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="white">
			<path d="M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z"/>
		</svg>`;

		// Count label with shadow for visibility - SMALLER
		const countLabel = document.createElement('div');
		countLabel.style.color = 'white';
		countLabel.style.fontWeight = 'bold';
		countLabel.style.fontSize = '14px';
		countLabel.style.textShadow = '0 2px 4px rgba(0, 0, 0, 0.8), 0 0 8px rgba(0, 0, 0, 0.6)';
		countLabel.style.whiteSpace = 'nowrap';
		countLabel.style.lineHeight = '1';
		countLabel.textContent = cluster.count.toString();

		markerContent.appendChild(iconDiv);
		markerContent.appendChild(countLabel);

		const marker = new google.maps.marker.AdvancedMarkerElement({
			position: { lat: cluster.latitude, lng: cluster.longitude },
			map: map,
			title: `${cluster.count} aircraft in this area`,
			content: markerContent,
			zIndex: 500
		});

		marker.addListener('click', () => {
			handleClusterClick(cluster);
		});

		markerContent.addEventListener('mouseenter', () => {
			markerContent.style.transform = 'scale(1.15)';
		});

		markerContent.addEventListener('mouseleave', () => {
			markerContent.style.transform = 'scale(1)';
		});

		return marker;
	}

	function handleClusterClick(cluster: AircraftCluster): void {
		console.log('[CLUSTER] Clicked on cluster:', cluster.id);

		if (!map) return;

		const bounds = new google.maps.LatLngBounds(
			{ lat: cluster.bounds.south, lng: cluster.bounds.west },
			{ lat: cluster.bounds.north, lng: cluster.bounds.east }
		);

		map.fitBounds(bounds);

		// Zoom in slightly more than just fitting bounds
		const currentZoom = map.getZoom() || 10;
		map.setZoom(currentZoom + 1);
	}

	function clearClusterMarkers(): void {
		console.log('[CLUSTER] Clearing all cluster markers. Count:', clusterMarkers.size);
		clusterMarkers.forEach((marker) => {
			marker.map = null;
		});
		clusterMarkers.clear();

		// Also clear cluster polygons
		console.log('[CLUSTER] Clearing all cluster polygons. Count:', clusterPolygons.size);
		clusterPolygons.forEach((polygon) => {
			polygon.setMap(null);
		});
		clusterPolygons.clear();
		console.log('[CLUSTER] All cluster markers and polygons cleared');
	}

	// Aircraft trail functions
	function updateAircraftTrail(aircraft: Aircraft): void {
		if (!map || currentSettings.positionFixWindow === 0) {
			// Remove trail if disabled
			clearTrailForAircraft(aircraft.id);
			return;
		}

		const fixes = aircraft.fixes || []; // Get all fixes from device

		// Filter fixes to those within the position fix window
		const cutoffTime = dayjs().subtract(currentSettings.positionFixWindow, 'hour');
		const trailFixes = fixes.filter((fix) => dayjs(fix.timestamp).isAfter(cutoffTime));

		if (trailFixes.length < 2) {
			// Need at least 2 points to draw a trail
			clearTrailForAircraft(aircraft.id);
			return;
		}

		// Clear existing trail
		clearTrailForAircraft(aircraft.id);

		// Create polyline segments with progressive transparency
		const polylines: google.maps.Polyline[] = [];
		for (let i = 0; i < trailFixes.length - 1; i++) {
			// Calculate opacity: newest segment (i=0) = 0.7, oldest = 0.2
			const segmentOpacity = 0.7 - (i / (trailFixes.length - 2)) * 0.5;

			// Use color based on active status and altitude from the newer fix in the segment
			const segmentColor = getMarkerColor(trailFixes[i]);

			const segment = new google.maps.Polyline({
				path: [
					{ lat: trailFixes[i].latitude, lng: trailFixes[i].longitude },
					{ lat: trailFixes[i + 1].latitude, lng: trailFixes[i + 1].longitude }
				],
				geodesic: true,
				strokeColor: segmentColor,
				strokeOpacity: segmentOpacity,
				strokeWeight: 2,
				map: map
			});

			polylines.push(segment);
		}

		// Create dots at each fix position
		const dots: google.maps.Circle[] = [];
		trailFixes.forEach((fix, index) => {
			// Calculate opacity: newest (index 0) = 0.7, oldest = 0.2
			const opacity = 0.7 - (index / (trailFixes.length - 1)) * 0.5;

			// Use color based on active status and altitude for each dot
			const dotColor = getMarkerColor(fix);

			const dot = new google.maps.Circle({
				center: { lat: fix.latitude, lng: fix.longitude },
				radius: 10, // 10 meters radius
				strokeColor: dotColor,
				strokeOpacity: opacity,
				strokeWeight: 1,
				fillColor: dotColor,
				fillOpacity: opacity * 0.5,
				map: map
			});

			dots.push(dot);
		});

		// Store trail data
		aircraftTrails.set(aircraft.id, { polylines, dots });
	}

	function clearTrailForAircraft(aircraftId: string): void {
		const trail = aircraftTrails.get(aircraftId);
		if (trail) {
			trail.polylines.forEach((polyline) => polyline.setMap(null));
			trail.dots.forEach((dot) => dot.setMap(null));
			aircraftTrails.delete(aircraftId);
		}
	}

	function clearAllTrails(): void {
		aircraftTrails.forEach((trail) => {
			trail.polylines.forEach((polyline) => polyline.setMap(null));
			trail.dots.forEach((dot) => dot.setMap(null));
		});
		aircraftTrails.clear();
	}

	function updateAllAircraftTrails(): void {
		activeDevices.forEach((device) => {
			updateAircraftTrail(device);
		});
	}

	// Area tracker functions
	async function toggleAreaTracker(): Promise<void> {
		if (!areaTrackerAvailable) {
			toaster.info({
				title: 'Please zoom in to use the area tracker feature. The current view area is too large.'
			});
			return;
		}

		areaTrackerActive = !areaTrackerActive;
		console.log('[AREA TRACKER] Area tracker toggled:', areaTrackerActive);
		saveAreaTrackerState();

		if (areaTrackerActive) {
			// Hybrid approach: Fetch immediate snapshot then update WebSocket subscriptions
			await fetchAndDisplayDevicesInViewport();
			updateAreaSubscriptions();
		} else {
			clearAreaSubscriptions();
		}
	}

	function updateAreaTrackerAvailability(): void {
		if (!map) return;

		// When limit is disabled, area tracker is always available
		if (!AREA_TRACKER_LIMIT_ENABLED) {
			areaTrackerAvailable = true;
			return;
		}

		const area = calculateViewportArea();
		const wasAvailable = areaTrackerAvailable;
		areaTrackerAvailable = area <= 4000000; // 4,000,000 square miles limit (fits continental US)

		console.log(
			`[AREA TRACKER] Map area: ${area.toFixed(2)} sq miles, Available: ${areaTrackerAvailable}`
		);

		// If area tracker becomes unavailable while active, deactivate it
		if (!areaTrackerAvailable && areaTrackerActive) {
			areaTrackerActive = false;
			clearAreaSubscriptions();
		}

		// If availability changed, log it
		if (wasAvailable !== areaTrackerAvailable) {
			console.log(
				`[AREA TRACKER] Availability changed: ${wasAvailable} -> ${areaTrackerAvailable}`
			);
		}
	}

	function getVisibleLatLonSquares(): Array<{ lat: number; lon: number }> {
		if (!map) return [];

		const bounds = map.getBounds();
		if (!bounds) return [];

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		// Get integer degree boundaries
		const latMin = Math.floor(sw.lat());
		const latMax = Math.floor(ne.lat());
		const lonMin = Math.floor(sw.lng());
		const lonMax = Math.floor(ne.lng());

		const squares: Array<{ lat: number; lon: number }> = [];

		// Include all squares that intersect with the visible area
		for (let lat = latMin; lat <= latMax + 1; lat++) {
			for (let lon = lonMin; lon <= lonMax + 1; lon++) {
				squares.push({ lat, lon });
			}
		}

		console.log(
			`[AREA TRACKER] Visible squares: lat ${latMin}-${latMax + 1}, lon ${lonMin}-${lonMax + 1} (${squares.length} total)`
		);
		return squares;
	}

	function updateAreaSubscriptions(): void {
		if (!areaTrackerActive || !areaTrackerAvailable || !map) return;

		// Don't subscribe to area updates when in clustered mode
		// Clustered mode uses periodic REST API refreshes instead of real-time WebSocket updates
		if (isClusteredMode) {
			console.log('[AREA TRACKER] Skipping subscription - in clustered mode');
			return;
		}

		const bounds = map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		const geoBounds = {
			north: ne.lat(),
			south: sw.lat(),
			east: ne.lng(),
			west: sw.lng()
		};

		const message = {
			action: 'subscribe',
			type: 'area_bulk' as const,
			bounds: geoBounds
		};

		console.log('[AREA TRACKER] Bulk subscribe:', message);
		fixFeed.sendWebSocketMessage(message);

		// For debugging: calculate what squares this represents
		const visibleSquares = getVisibleLatLonSquares();
		console.log(`[AREA TRACKER] Bulk subscription covers ${visibleSquares.length} squares`);

		// Update current subscriptions for debugging display
		const newSubscriptions = new SvelteSet<string>();
		visibleSquares.forEach((square) => {
			const key = `area.${square.lat}.${square.lon}`;
			newSubscriptions.add(key);
		});
		currentAreaSubscriptions = newSubscriptions;

		// Update debug status to show area subscription count
		debugStatus.update((current) => ({
			...current,
			activeAreaSubscriptions: visibleSquares.length
		}));
	}

	function clearAreaSubscriptions(): void {
		if (!map) return;

		const message = {
			action: 'unsubscribe',
			type: 'area_bulk' as const,
			bounds: {
				north: 0,
				south: 0,
				east: 0,
				west: 0
			}
		};

		console.log('[AREA TRACKER] Bulk unsubscribe');
		fixFeed.sendWebSocketMessage(message);

		currentAreaSubscriptions.clear();

		// Update debug status
		debugStatus.update((current) => ({
			...current,
			activeAreaSubscriptions: 0
		}));
	}

	async function fetchAndDisplayDevicesInViewport(): Promise<void> {
		if (!map) return;

		const bounds = map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		try {
			console.log('[REST] Fetching aircraft in viewport...');

			const response = await fixFeed.fetchAircraftInBoundingBox(
				sw.lat(), // south
				ne.lat(), // north
				sw.lng(), // west
				ne.lng(), // east
				undefined,
				MAX_AIRCRAFT_DISPLAY
			);

			const { items, total, clustered } = response;

			console.log(
				`[REST] Received ${items.length} items (total: ${total}, clustered: ${clustered})`
			);

			if (clustered) {
				console.log('[REST] Response is clustered, rendering cluster markers');

				// Enter clustered mode
				isClusteredMode = true;
				startClusterRefreshTimer();

				// Clear WebSocket area subscriptions - clustered mode uses REST API polling instead
				clearAreaSubscriptions();

				// Clear aircraft registry - we're forgetting all aircraft outside viewport
				aircraftRegistry.clear();

				clearAircraftMarkers();
				clearClusterMarkers();

				for (const item of items) {
					if (isClusterItem(item)) {
						const marker = createClusterMarker(item.data);
						clusterMarkers.set(item.data.id, marker);
					}
				}

				console.log(`[AIRCRAFT COUNT] ${clusterMarkers.size} cluster markers on map`);
			} else {
				console.log('[REST] Response has individual aircraft, rendering aircraft markers');

				// Exit clustered mode
				isClusteredMode = false;
				stopClusterRefreshTimer();

				// Restore WebSocket area subscriptions for real-time updates
				if (areaTrackerActive) {
					updateAreaSubscriptions();
				}

				// Clear aircraft registry - we're forgetting all aircraft outside viewport
				aircraftRegistry.clear();

				clearClusterMarkers();

				for (const item of items) {
					if (isAircraftItem(item)) {
						await aircraftRegistry.updateAircraftFromAircraftData(item.data);
					}
				}

				// Log the count of aircraft now on the map
				console.log(`[AIRCRAFT COUNT] ${aircraftMarkers.size} aircraft markers on map`);
			}
		} catch (error) {
			console.error('[REST] Failed to fetch aircraft in viewport:', error);
		}
	}

	// Start the cluster refresh timer (refreshes every 60 seconds when tab is visible)
	function startClusterRefreshTimer(): void {
		// Clear any existing timer
		stopClusterRefreshTimer();

		console.log('[CLUSTER REFRESH] Starting 60-second refresh timer');

		clusterRefreshTimer = setInterval(async () => {
			// Only refresh if the page is visible (user has the tab active)
			if (browser && document.visibilityState === 'visible') {
				console.log('[CLUSTER REFRESH] Tab is visible, refreshing clusters...');
				await fetchAndDisplayDevicesInViewport();
			} else {
				console.log('[CLUSTER REFRESH] Tab is hidden, skipping refresh');
			}
		}, 60000); // 60 seconds
	}

	// Stop the cluster refresh timer
	function stopClusterRefreshTimer(): void {
		if (clusterRefreshTimer) {
			console.log('[CLUSTER REFRESH] Stopping refresh timer');
			clearInterval(clusterRefreshTimer);
			clusterRefreshTimer = null;
		}
	}
</script>

<svelte:head>
	<title>Operations - Glider Flights</title>
</svelte:head>

<div class="fixed inset-x-0 top-[42px] bottom-0 w-full">
	<!-- Google Maps Container -->
	<div bind:this={mapContainer} class="h-full w-full"></div>

	<!-- Control Buttons -->
	<div class="absolute top-4 left-4 z-10 flex gap-2">
		<!-- Location Button -->
		<button
			bind:this={userLocationButton}
			class="location-btn"
			class:opacity-50={isLocating}
			disabled={isLocating}
			onclick={locateUser}
			title={isLocating ? 'Locating...' : 'Find My Location'}
		>
			{#if isLocating}
				<div
					class="h-5 w-5 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"
				></div>
			{:else}
				<span class="text-xl">📍</span>
			{/if}
		</button>

		<!-- Watchlist Button -->
		<button class="location-btn" onclick={() => (showWatchlistModal = true)} title="Watchlist">
			<ListChecks size={20} />
		</button>

		<!-- Area Tracker Button (only show when limit is enabled) -->
		{#if AREA_TRACKER_LIMIT_ENABLED}
			<button
				class="location-btn"
				class:area-tracker-active={areaTrackerActive}
				class:area-tracker-unavailable={!areaTrackerAvailable}
				onclick={toggleAreaTracker}
				title={areaTrackerAvailable
					? areaTrackerActive
						? 'Disable Area Tracker'
						: 'Enable Area Tracker'
					: 'Area Tracker unavailable (map too zoomed out)'}
			>
				{#if areaTrackerActive}
					<MapPlus size={20} />
				{:else}
					<MapMinus size={20} />
				{/if}
			</button>
		{/if}

		<!-- Settings Button -->
		<button class="location-btn" onclick={() => (showSettingsModal = true)} title="Settings">
			<Settings size={20} />
		</button>
	</div>

	<!-- Map Type Toggle Button -->
	<div class="absolute bottom-4 left-4 z-10">
		<button class="location-btn" onclick={toggleMapType} title="Toggle Map Type">
			<span class="text-sm font-medium">{mapType === 'satellite' ? 'Map' : 'Satellite'}</span>
		</button>
	</div>

	<!-- Compass Rose -->
	{#if isCompassActive && currentSettings.showCompassRose}
		<div class="compass-container absolute top-4 right-4 z-10">
			<div class="compass-rose" style="transform: rotate({compassHeading}deg)">
				<svg width="80" height="80" viewBox="0 0 80 80">
					<!-- Outer circle -->
					<circle
						cx="40"
						cy="40"
						r="38"
						fill="rgba(255, 255, 255, 0.9)"
						stroke="rgba(0, 0, 0, 0.3)"
						stroke-width="2"
					/>

					<!-- Cardinal direction markers -->
					<!-- North -->
					<line
						x1="40"
						y1="4"
						x2="40"
						y2="16"
						stroke="#dc2626"
						stroke-width="3"
						stroke-linecap="round"
					/>

					<!-- South -->
					<line
						x1="40"
						y1="64"
						x2="40"
						y2="76"
						stroke="#374151"
						stroke-width="2"
						stroke-linecap="round"
					/>

					<!-- East -->
					<line
						x1="64"
						y1="40"
						x2="76"
						y2="40"
						stroke="#374151"
						stroke-width="2"
						stroke-linecap="round"
					/>

					<!-- West -->
					<line
						x1="4"
						y1="40"
						x2="16"
						y2="40"
						stroke="#374151"
						stroke-width="2"
						stroke-linecap="round"
					/>

					<!-- Intercardinal directions -->
					<!-- Northeast -->
					<line
						x1="56.57"
						y1="23.43"
						x2="64.85"
						y2="15.15"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- Southeast -->
					<line
						x1="56.57"
						y1="56.57"
						x2="64.85"
						y2="64.85"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- Southwest -->
					<line
						x1="23.43"
						y1="56.57"
						x2="15.15"
						y2="64.85"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- Northwest -->
					<line
						x1="23.43"
						y1="23.43"
						x2="15.15"
						y2="15.15"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- North arrow (pointing up) -->
					<polygon
						points="40,8 44,18 40,16 36,18"
						fill="#dc2626"
						stroke="#dc2626"
						stroke-width="1"
					/>
				</svg>
			</div>
			<div
				class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-[24px] font-bold text-gray-700"
			>
				{displayHeading}°
			</div>
		</div>
	{/if}
</div>

<!-- Settings Modal -->
<SettingsModal bind:showModal={showSettingsModal} onSettingsChange={handleSettingsChange} />

<!-- Watchlist Modal -->
<WatchlistModal bind:showModal={showWatchlistModal} />

<!-- Aircraft Status Modal -->
<AircraftStatusModal bind:showModal={showAircraftStatusModal} bind:selectedAircraft />

<!-- Airport Modal -->
<AirportModal bind:showModal={showAirportModal} bind:selectedAirport />

<!-- Airspace Modal -->
<AirspaceModal bind:showModal={showAirspaceModal} bind:selectedAirspace />

<style>
	/* Location button styling */
	.location-btn {
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

	.location-btn:hover {
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
		transform: translateY(-1px);
	}

	.location-btn:focus {
		outline: none;
		box-shadow:
			0 0 0 2px rgba(59, 130, 246, 0.5),
			0 2px 8px rgba(0, 0, 0, 0.15);
	}

	.location-btn:disabled {
		cursor: not-allowed;
		opacity: 0.5;
		transform: none;
	}

	/* Area tracker button states */
	.area-tracker-active {
		background: #10b981; /* Green background when active */
		color: white;
	}

	.area-tracker-active:hover {
		background: #059669; /* Darker green on hover */
	}

	.area-tracker-unavailable {
		background: #ef4444; /* Red background when unavailable */
		color: white;
	}

	.area-tracker-unavailable:hover {
		background: #dc2626; /* Darker red on hover */
	}

	/* Loading spinner animation */
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.animate-spin {
		animation: spin 1s linear infinite;
	}

	/* Compass rose styling */
	.compass-container {
		filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.3));
	}

	.compass-rose {
		transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
	}

	/* Airport marker styling */
	:global(.airport-marker) {
		display: flex;
		flex-direction: column;
		align-items: center;
		pointer-events: auto;
		cursor: pointer;
	}

	:global(.airport-icon) {
		background: transparent;
		border: 2px solid #374151;
		border-radius: 50%;
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 12px;
		color: #fb923c;
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
	}

	:global(.airport-label) {
		background: rgba(255, 255, 255, 0.5);
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
	}

	/* Aircraft marker styling */
	:global(.aircraft-marker) {
		display: flex;
		flex-direction: column;
		align-items: center;
		pointer-events: auto;
		cursor: pointer;
		transform-origin: center center; /* Center the marker on the aircraft position */
		transition: all 0.2s ease;
	}

	:global(.aircraft-marker:hover) {
		transform: scale(1.15);
	}

	:global(.aircraft-icon) {
		background: #ef4444;
		border: 3px solid #ffffff;
		border-radius: 50%;
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		box-shadow: 0 3px 12px rgba(0, 0, 0, 0.5);
		transition: all 0.2s ease;
		position: relative;
	}

	:global(.aircraft-marker:hover .aircraft-icon) {
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.7);
		border-width: 4px;
	}

	:global(.aircraft-icon svg) {
		width: 20px;
		height: 20px;
		filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3));
	}

	:global(.aircraft-label) {
		background: rgba(255, 255, 255, 0.95); /* White background with 95% opacity */
		border: 2px solid;
		border-radius: 6px;
		padding: 4px 8px;
		margin-top: 6px;
		box-shadow: 0 3px 8px rgba(0, 0, 0, 0.4);
		min-width: 60px;
		text-align: center;
		transition: all 0.2s ease;
	}

	:global(.aircraft-marker:hover .aircraft-label) {
		background: rgba(255, 255, 255, 1); /* Fully opaque on hover */
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.6);
	}

	:global(.aircraft-tail) {
		font-size: 12px;
		font-weight: 700;
		line-height: 1.2;
		color: #1f2937;
		text-shadow: 0 1px 2px rgba(255, 255, 255, 0.8);
	}

	:global(.aircraft-altitude) {
		font-size: 10px;
		font-weight: 600;
		line-height: 1.1;
		color: #374151;
		text-shadow: 0 1px 2px rgba(255, 255, 255, 0.8);
		margin-top: 1px;
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
