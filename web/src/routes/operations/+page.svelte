<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import { page } from '$app/stores';
	import { Settings, ListChecks, MapPlus, MapMinus } from '@lucide/svelte';
	import WatchlistModal from '$lib/components/WatchlistModal.svelte';
	import SettingsModal from '$lib/components/SettingsModal.svelte';
	import AircraftStatusModal from '$lib/components/AircraftStatusModal.svelte';
	import AirportModal from '$lib/components/AirportModal.svelte';
	import AirspaceModal from '$lib/components/AirspaceModal.svelte';
	import CompassRose from '$lib/components/CompassRose.svelte';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { FixFeed } from '$lib/services/FixFeed';
	import {
		AirportMarkerManager,
		ReceiverMarkerManager,
		AirspaceOverlayManager,
		RunwayOverlayManager,
		AircraftMarkerManager,
		ClusterMarkerManager
	} from '$lib/services/markers';
	import type { Aircraft, Airport, Airspace, DeviceOrientationEventWithCompass } from '$lib/types';
	import { isAircraftItem, isClusterItem } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { debugStatus } from '$lib/stores/websocket-status';
	import { browser } from '$app/environment';
	import type { AircraftRegistryEvent } from '$lib/services/AircraftRegistry';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'Operations']);

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
	let compassHeading: number = $state(0);
	let previousCompassHeading: number = 0;
	let isCompassActive: boolean = $state(false);
	let displayHeading: number = $state(0);

	// Airport marker manager (created first as runway manager depends on it)
	const airportMarkerManager = new AirportMarkerManager({
		onAirportClick: (airport) => {
			selectedAirport = airport;
			showAirportModal = true;
		},
		onAirportsLoaded: () => {
			// Refresh runway overlays when airports are loaded
			runwayOverlayManager.refresh();
		}
	});

	// Receiver marker manager
	const receiverMarkerManager = new ReceiverMarkerManager();

	// Airspace overlay manager
	const airspaceOverlayManager = new AirspaceOverlayManager({
		onAirspaceClick: (airspace) => {
			selectedAirspace = airspace;
			showAirspaceModal = true;
		}
	});

	// Runway overlay manager (uses airport manager for runway data)
	const runwayOverlayManager = new RunwayOverlayManager({
		getRunways: () => airportMarkerManager.getRunways()
	});

	// Aircraft marker manager
	const aircraftMarkerManager = new AircraftMarkerManager({
		onAircraftClick: (aircraft) => {
			handleAircraftClick(aircraft);
		}
	});

	// Cluster marker manager
	const clusterMarkerManager = new ClusterMarkerManager({
		onClusterClick: (cluster) => {
			clusterMarkerManager.zoomToCluster(cluster);
		}
	});

	// Zoom debounce timer
	let zoomDebounceTimer: ReturnType<typeof setTimeout> | null = null;

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
				aircraftMarkerManager.updateAllTrails(activeDevices);
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
			aircraftMarkerManager.setPositionFixWindow(newSettings.positionFixWindow);
			debouncedUpdateAircraftTrails();
		}
	}

	// Handle aircraft marker click
	function handleAircraftClick(aircraft: Aircraft) {
		logger.debug('[AIRCRAFT CLICK] Aircraft clicked: {registration}', {
			registration: aircraft.registration || aircraft.address
		});
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
			logger.debug('[MAP] Saved map state: {state}', { state });
		} catch (e) {
			logger.warn('[MAP] Failed to save map state to localStorage: {error}', { error: e });
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
					logger.debug('[MAP] Using URL parameters: {params}', {
						params: {
							lat: parsedLat,
							lng: parsedLng,
							zoom: parsedZoom
						}
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
				logger.debug('[MAP] Loaded saved map state: {state}', { state });
				return state;
			}
		} catch (e) {
			logger.warn('[MAP] Failed to load map state from localStorage: {error}', { error: e });
		}

		logger.debug('[MAP] Using default CONUS center');
		return { center: CONUS_CENTER, zoom: 4 };
	}

	// Save area tracker state to localStorage
	function saveAreaTrackerState(): void {
		if (!browser) return;

		try {
			localStorage.setItem(AREA_TRACKER_KEY, JSON.stringify(areaTrackerActive));
			logger.debug('[AREA TRACKER] Saved state: {state}', { state: areaTrackerActive });
		} catch (e) {
			logger.warn('[AREA TRACKER] Failed to save state to localStorage: {error}', { error: e });
		}
	}

	// Load area tracker state from localStorage
	function loadAreaTrackerState(): boolean {
		if (!browser) return true;

		// When limit is disabled, area tracker is always on
		if (!AREA_TRACKER_LIMIT_ENABLED) {
			logger.debug('[AREA TRACKER] Limit disabled, area tracker always on');
			return true;
		}

		try {
			const saved = localStorage.getItem(AREA_TRACKER_KEY);
			if (saved !== null) {
				const state = JSON.parse(saved);
				logger.debug('[AREA TRACKER] Loaded saved state: {state}', { state });
				return state;
			}
		} catch (e) {
			logger.warn('[AREA TRACKER] Failed to load state from localStorage: {error}', { error: e });
		}

		logger.debug('[AREA TRACKER] Using default state: true');
		return true;
	}

	// Save map type to localStorage
	function saveMapType(): void {
		if (!browser) return;

		try {
			localStorage.setItem(MAP_TYPE_KEY, mapType);
			logger.debug('[MAP TYPE] Saved map type: {mapType}', { mapType });
		} catch (e) {
			logger.warn('[MAP TYPE] Failed to save map type to localStorage: {error}', { error: e });
		}
	}

	// Load map type from localStorage
	function loadMapType(): 'satellite' | 'roadmap' {
		if (!browser) return 'satellite';

		try {
			const saved = localStorage.getItem(MAP_TYPE_KEY);
			if (saved === 'satellite' || saved === 'roadmap') {
				logger.debug('[MAP TYPE] Loaded saved map type: {saved}', { saved });
				return saved;
			}
		} catch (e) {
			logger.warn('[MAP TYPE] Failed to load map type from localStorage: {error}', { error: e });
		}

		logger.debug('[MAP TYPE] Using default: satellite');
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
		logger.debug('[MAP TYPE] Toggled to: {mapType}', { mapType });
	}

	// Reactive effects for settings changes
	$effect(() => {
		if (map) {
			const area = calculateViewportArea();
			airportMarkerManager.checkAndUpdate(area, currentSettings.showAirportMarkers);
		}
	});

	$effect(() => {
		if (map) {
			const area = calculateViewportArea();
			receiverMarkerManager.checkAndUpdate(area, currentSettings.showReceiverMarkers);
		}
	});

	$effect(() => {
		if (map) {
			const area = calculateViewportArea();
			airspaceOverlayManager.checkAndUpdate(area, currentSettings.showAirspaceMarkers);
		}
	});

	$effect(() => {
		if (map) {
			const area = calculateViewportArea();
			runwayOverlayManager.checkAndUpdate(area, currentSettings.showRunwayOverlays);
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
				logger.debug('[CLUSTERED MODE] Ignoring aircraft registry event: {type}', {
					type: event.type
				});
				return;
			}

			if (event.type === 'aircraft_changed') {
				activeDevices = event.aircraft;
			} else if (event.type === 'aircraft_updated') {
				// When a device is updated, create or update its marker if we have a map
				if (map) {
					aircraftMarkerManager.updateMarkerFromAircraft(event.aircraft);
				}
			} else if (event.type === 'fix_added') {
				logger.debug('Fix added to aircraft: {aircraftId} {fix}', {
					aircraftId: event.aircraft.id,
					fix: event.fix
				});
				// Update the aircraft marker immediately when a new fix is added
				if (map && event.fix) {
					aircraftMarkerManager.updateMarkerFromDevice(event.aircraft, event.fix);
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
		logger.debug('[EFFECT] Map available, initializing markers for {count} aircraft', {
			count: activeDevices.length
		});
		activeDevices.forEach((aircraft) => {
			aircraftMarkerManager.updateMarkerFromAircraft(aircraft);
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
			aircraftMarkerManager.clear();
			clusterMarkerManager.clear();
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
		logger.debug('[MAP] Initializing Google Maps: {state}', {
			state: {
				hasContainer: !!mapContainer,
				hasGoogle: !!window.google
			}
		});

		if (!mapContainer || !window.google) {
			logger.error('[MAP] Missing requirements for map initialization');
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

		// Set map on marker managers
		airportMarkerManager.setMap(map);
		receiverMarkerManager.setMap(map);
		airspaceOverlayManager.setMap(map);
		runwayOverlayManager.setMap(map);
		aircraftMarkerManager.setMap(map);
		clusterMarkerManager.setMap(map);

		// Add event listeners for viewport changes
		map.addListener('zoom_changed', () => {
			setTimeout(() => {
				const area = calculateViewportArea();
				airportMarkerManager.checkAndUpdate(area, currentSettings.showAirportMarkers);
				receiverMarkerManager.checkAndUpdate(area, currentSettings.showReceiverMarkers);
				airspaceOverlayManager.checkAndUpdate(area, currentSettings.showAirspaceMarkers);
				runwayOverlayManager.checkAndUpdate(area, currentSettings.showRunwayOverlays);
			}, 100); // Small delay to ensure bounds are updated
			// Update aircraft marker scaling on zoom change
			aircraftMarkerManager.updateAllMarkersScale();
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
			const area = calculateViewportArea();
			airportMarkerManager.checkAndUpdate(area, currentSettings.showAirportMarkers);
			receiverMarkerManager.checkAndUpdate(area, currentSettings.showReceiverMarkers);
			airspaceOverlayManager.checkAndUpdate(area, currentSettings.showAirspaceMarkers);
			runwayOverlayManager.checkAndUpdate(area, currentSettings.showRunwayOverlays);

			// Always fetch aircraft in viewport after panning
			await fetchAndDisplayDevicesInViewport();

			// Update WebSocket subscriptions only if area tracker is active
			if (areaTrackerActive) {
				updateAreaSubscriptions();
			}

			// Save map state after panning
			saveMapState();
		});

		// Initial check for airports, receivers, airspaces, and runways
		setTimeout(() => {
			const area = calculateViewportArea();
			airportMarkerManager.checkAndUpdate(area, currentSettings.showAirportMarkers);
			receiverMarkerManager.checkAndUpdate(area, currentSettings.showReceiverMarkers);
			airspaceOverlayManager.checkAndUpdate(area, currentSettings.showAirspaceMarkers);
			runwayOverlayManager.checkAndUpdate(area, currentSettings.showRunwayOverlays);
		}, 1000); // Give map time to fully initialize

		// Initial area tracker availability check
		setTimeout(updateAreaTrackerAvailability, 1000);

		logger.debug('[MAP] Google Maps initialized for operations view. Map ready for markers.');
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

			logger.debug('User location found: {lat}, {lng}', {
				lat: userLocation.lat,
				lng: userLocation.lng
			});

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

			logger.debug('User located and animated to: {lat}, {lng}', {
				lat: userLocation.lat,
				lng: userLocation.lng
			});
		} catch (error) {
			logger.error('Error getting user location: {error}', { error });
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
					logger.debug('Device orientation permission denied');
					return;
				}
			} catch (error) {
				logger.debug('Error requesting device orientation permission: {error}', { error });
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
					logger.debug('Screen orientation locked to portrait');
				}
			} catch (error) {
				logger.debug('Could not lock screen orientation: {error}', { error });
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
		logger.debug('Viewport area: {area} square miles', {
			area: areaSquareMiles.toFixed(2)
		});
		return areaSquareMiles;
	}

	function handleOrientationChange(event: DeviceOrientationEventWithCompass): void {
		if (event.alpha !== null) {
			isCompassActive = true;

			// Get the magnetic heading from the device
			// iOS provides webkitCompassHeading which is the true magnetic heading
			const webkitHeading = event.webkitCompassHeading;
			let magneticHeading: number;

			if (webkitHeading !== undefined && webkitHeading !== null) {
				// iOS: Use webkitCompassHeading directly (already magnetic heading)
				magneticHeading = webkitHeading;
			} else if (event.absolute && event.alpha !== null) {
				// Android with absolute orientation: Convert alpha to magnetic heading
				// alpha is counter-clockwise from north, compass is clockwise from north
				magneticHeading = (360 - event.alpha) % 360;
			} else {
				// Fallback: Use alpha as-is (may not be accurate, default to 0 if somehow null)
				logger.warn(
					'Using raw alpha for heading (absolute={absolute}), compass may be inaccurate',
					{ absolute: event.absolute }
				);
				magneticHeading = event.alpha ?? 0;
			}

			// Display the magnetic heading (what direction the device is pointing)
			displayHeading = Math.round(magneticHeading);

			// For the compass rose, we need to rotate it opposite to the device heading
			// so that north always points north on the compass
			let newCompassRotation = (360 - magneticHeading) % 360;

			// Normalize to 0-360 range
			newCompassRotation = ((newCompassRotation % 360) + 360) % 360;

			// Calculate the shortest rotation path to avoid spinning around unnecessarily
			let delta = newCompassRotation - previousCompassHeading;

			// Adjust for boundary crossing to take the shortest path
			if (delta > 180) {
				compassHeading = newCompassRotation - 360;
			} else if (delta < -180) {
				compassHeading = newCompassRotation + 360;
			} else {
				compassHeading = newCompassRotation;
			}

			// Update previous heading
			previousCompassHeading = compassHeading;
		}
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
		logger.debug('[AREA TRACKER] Area tracker toggled: {active}', {
			active: areaTrackerActive
		});
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

		logger.debug('[AREA TRACKER] Map area: {area} sq miles, Available: {available}', {
			area: area.toFixed(2),
			available: areaTrackerAvailable
		});

		// If area tracker becomes unavailable while active, deactivate it
		if (!areaTrackerAvailable && areaTrackerActive) {
			areaTrackerActive = false;
			clearAreaSubscriptions();
		}

		// If availability changed, log it
		if (wasAvailable !== areaTrackerAvailable) {
			logger.debug('[AREA TRACKER] Availability changed: {from} -> {to}', {
				from: wasAvailable,
				to: areaTrackerAvailable
			});
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

		logger.debug(
			'[AREA TRACKER] Visible squares: lat {latMin}-{latMax}, lon {lonMin}-{lonMax} ({total} total)',
			{
				latMin,
				latMax: latMax + 1,
				lonMin,
				lonMax: lonMax + 1,
				total: squares.length
			}
		);
		return squares;
	}

	function updateAreaSubscriptions(): void {
		if (!areaTrackerActive || !areaTrackerAvailable || !map) return;

		// Don't subscribe to area updates when in clustered mode
		// Clustered mode uses periodic REST API refreshes instead of real-time WebSocket updates
		if (isClusteredMode) {
			logger.debug('[AREA TRACKER] Skipping subscription - in clustered mode');
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

		logger.debug('[AREA TRACKER] Bulk subscribe: {message}', { message });
		fixFeed.sendWebSocketMessage(message);

		// For debugging: calculate what squares this represents
		const visibleSquares = getVisibleLatLonSquares();
		logger.debug('[AREA TRACKER] Bulk subscription covers {count} squares', {
			count: visibleSquares.length
		});

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

		logger.debug('[AREA TRACKER] Bulk unsubscribe');
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
			logger.debug('[REST] Fetching aircraft in viewport...');

			const response = await fixFeed.fetchAircraftInBoundingBox(
				sw.lat(), // south
				ne.lat(), // north
				sw.lng(), // west
				ne.lng(), // east
				undefined,
				MAX_AIRCRAFT_DISPLAY
			);

			const { items, total, clustered } = response;

			logger.debug('[REST] Received {count} items (total: {total}, clustered: {clustered})', {
				count: items.length,
				total,
				clustered
			});

			if (clustered) {
				logger.debug('[REST] Response is clustered, rendering cluster markers');

				// Enter clustered mode
				isClusteredMode = true;
				startClusterRefreshTimer();

				// Clear WebSocket area subscriptions - clustered mode uses REST API polling instead
				clearAreaSubscriptions();

				// Clear aircraft registry - we're forgetting all aircraft outside viewport
				aircraftRegistry.clear();

				aircraftMarkerManager.clear();
				clusterMarkerManager.clear();

				for (const item of items) {
					if (isClusterItem(item)) {
						clusterMarkerManager.createMarker(item.data);
					}
				}

				logger.debug('[AIRCRAFT COUNT] {count} cluster markers on map', {
					count: clusterMarkerManager.getMarkers().size
				});
			} else {
				logger.debug('[REST] Response has individual aircraft, rendering aircraft markers');

				// Exit clustered mode
				isClusteredMode = false;
				stopClusterRefreshTimer();

				// Restore WebSocket area subscriptions for real-time updates
				if (areaTrackerActive) {
					updateAreaSubscriptions();
				}

				// Clear aircraft registry - we're forgetting all aircraft outside viewport
				aircraftRegistry.clear();

				clusterMarkerManager.clear();

				for (const item of items) {
					if (isAircraftItem(item)) {
						await aircraftRegistry.updateAircraftFromAircraftData(item.data);
					}
				}

				// Log the count of aircraft now on the map
				logger.debug('[AIRCRAFT COUNT] {count} aircraft markers on map', {
					count: aircraftMarkerManager.getMarkers().size
				});
			}
		} catch (error) {
			logger.error('[REST] Failed to fetch aircraft in viewport: {error}', { error });
		}
	}

	// Start the cluster refresh timer (refreshes every 60 seconds when tab is visible)
	function startClusterRefreshTimer(): void {
		// Clear any existing timer
		stopClusterRefreshTimer();

		logger.debug('[CLUSTER REFRESH] Starting 60-second refresh timer');

		clusterRefreshTimer = setInterval(async () => {
			// Only refresh if the page is visible (user has the tab active)
			if (browser && document.visibilityState === 'visible') {
				logger.debug('[CLUSTER REFRESH] Tab is visible, refreshing clusters...');
				await fetchAndDisplayDevicesInViewport();
			} else {
				logger.debug('[CLUSTER REFRESH] Tab is hidden, skipping refresh');
			}
		}, 60000); // 60 seconds
	}

	// Stop the cluster refresh timer
	function stopClusterRefreshTimer(): void {
		if (clusterRefreshTimer) {
			logger.debug('[CLUSTER REFRESH] Stopping refresh timer');
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
		<div class="absolute top-4 right-4 z-10">
			<CompassRose rotation={compassHeading} {displayHeading} />
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
