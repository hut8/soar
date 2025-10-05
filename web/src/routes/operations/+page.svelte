<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { serverCall } from '$lib/api/server';
	import { Loader } from '@googlemaps/js-api-loader';
	import { Settings, ListChecks, MapPlus, MapMinus } from '@lucide/svelte';
	import WatchlistModal from '$lib/components/WatchlistModal.svelte';
	import SettingsModal from '$lib/components/SettingsModal.svelte';
	import AircraftStatusModal from '$lib/components/AircraftStatusModal.svelte';
	import AirportModal from '$lib/components/AirportModal.svelte';
	import { DeviceRegistry } from '$lib/services/DeviceRegistry';
	import { FixFeed } from '$lib/services/FixFeed';
	import { Device } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { debugStatus } from '$lib/stores/watchlist';
	import { browser } from '$app/environment';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import type { Fix } from '$lib/types';
	import type { DeviceRegistryEvent } from '$lib/services/DeviceRegistry';

	// Extend dayjs with relative time plugin
	dayjs.extend(relativeTime);

	// TypeScript interfaces for airport data
	interface RunwayEndView {
		ident: string | null;
		latitude_deg: number | null;
		longitude_deg: number | null;
		elevation_ft: number | null;
		heading_degt: number | null;
		displaced_threshold_ft: number | null;
	}

	interface RunwayView {
		id: number;
		length_ft: number | null;
		width_ft: number | null;
		surface: string | null;
		lighted: boolean;
		closed: boolean;
		low: RunwayEndView;
		high: RunwayEndView;
	}

	interface AirportView {
		id: number;
		ident: string;
		airport_type: string;
		name: string;
		latitude_deg: string | null; // BigDecimal comes as string from API
		longitude_deg: string | null; // BigDecimal comes as string from API
		elevation_ft: number | null;
		continent: string | null;
		iso_country: string | null;
		iso_region: string | null;
		municipality: string | null;
		scheduled_service: boolean;
		icao_code: string | null;
		iata_code: string | null;
		gps_code: string | null;
		local_code: string | null;
		home_link: string | null;
		wikipedia_link: string | null;
		keywords: string | null;
		runways: RunwayView[];
	}
	// Placeholder for Google Maps API key - to be added later
	const GOOGLE_MAPS_API_KEY = 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

	let mapContainer: HTMLElement;
	let map: google.maps.Map;
	let userLocationButton: HTMLButtonElement;
	let isLocating = $state(false);
	let userMarker: google.maps.marker.AdvancedMarkerElement | null = null;

	// Compass rose variables
	let deviceHeading: number = 0;
	let compassHeading: number = $state(0);
	let previousCompassHeading: number = 0;
	let isCompassActive: boolean = $state(false);
	let displayHeading: number = $state(0);

	// Airport display variables
	let airports: AirportView[] = [];
	let airportMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let shouldShowAirports: boolean = false;
	let airportUpdateDebounceTimer: number | null = null;

	// Aircraft display variables
	let aircraftMarkers = new SvelteMap<string, google.maps.marker.AdvancedMarkerElement>();
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
	let selectedAircraft: Device | null = $state(null);

	// Airport modal state
	let showAirportModal = $state(false);
	let selectedAirport: AirportView | null = $state(null);

	// Settings state - these will be updated by the SettingsModal
	let currentSettings = $state({
		showCompassRose: true,
		showAirportMarkers: true,
		showRunwayOverlays: false,
		trailLength: 0
	});

	// Handle settings changes from SettingsModal
	function handleSettingsChange(newSettings: {
		showCompassRose: boolean;
		showAirportMarkers: boolean;
		showRunwayOverlays: boolean;
		trailLength: number;
	}) {
		// Replace entire object to ensure Svelte 5 reactivity triggers
		currentSettings = { ...newSettings };
	}

	// Handle aircraft marker click
	function handleAircraftClick(device: Device) {
		console.log('[AIRCRAFT CLICK] Aircraft clicked:', device.registration || device.address);
		selectedAircraft = device;
		showAircraftStatusModal = true;
	}

	// Center of continental US
	const CONUS_CENTER = {
		lat: 39.8283,
		lng: -98.5795
	};

	// Map persistence key for localStorage
	const MAP_STATE_KEY = 'operations-map-state';
	const AREA_TRACKER_KEY = 'operations-area-tracker';

	// Interface for stored map state
	interface MapState {
		center: google.maps.LatLngLiteral;
		zoom: number;
	}

	// Helper function to format altitude with relative time and check if fix is old
	function formatAltitudeWithTime(
		altitude_feet: number | null | undefined,
		timestamp: string
	): {
		altitudeText: string;
		isOld: boolean;
	} {
		const altitudeFt = altitude_feet ? `${altitude_feet}ft` : '---ft';

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

	// Load map state from localStorage, fallback to CONUS center
	function loadMapState(): MapState {
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
		if (!browser) return false;

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

		console.log('[AREA TRACKER] Using default state: false');
		return false;
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

	// Reactive effect for trail length settings
	$effect(() => {
		// Access trailLength to make this effect reactive to changes
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		const _trailLength = currentSettings.trailLength;
		if (map) {
			// Update all aircraft trails when trail length changes
			updateAllAircraftTrails();
		}
	});

	// Get singleton instances
	const deviceRegistry = DeviceRegistry.getInstance();
	const fixFeed = FixFeed.getInstance();

	// Subscribe to device registry and update aircraft markers
	let activeDevices: Device[] = $state([]);
	let initialMarkersRendered = false;

	// Area tracker state
	let areaTrackerActive = $state(false);
	let areaTrackerAvailable = $state(true); // Whether area tracker can be enabled (based on map area)
	let currentAreaSubscriptions = new SvelteSet<string>(); // Track subscribed areas

	$effect(() => {
		const unsubscribeRegistry = deviceRegistry.subscribe((event: DeviceRegistryEvent) => {
			if (event.type === 'devices_changed') {
				activeDevices = event.devices;
			} else if (event.type === 'device_updated') {
				// When a device is updated, create or update its marker if we have a map
				if (map) {
					const latestFix = event.device.getLatestFix();
					if (latestFix) {
						updateAircraftMarkerFromDevice(event.device, latestFix);
					}
				}
			} else if (event.type === 'fix_added') {
				console.log('Fix added to device:', event.device.id, event.fix);
				// Update the aircraft marker immediately when a new fix is added
				if (map && event.fix) {
					updateAircraftMarkerFromDevice(event.device, event.fix);
				}
			}
		});

		// Initialize active devices
		activeDevices = deviceRegistry.getAllDevices();

		return () => {
			unsubscribeRegistry();
		};
	});

	// Effect to initialize aircraft markers when map becomes available (runs once)
	$effect(() => {
		if (!map || initialMarkersRendered) return;

		// When map first becomes available, render markers for all active devices
		console.log(
			'[EFFECT] Map available, initializing markers for',
			activeDevices.length,
			'devices'
		);
		activeDevices.forEach((device) => {
			const latestFix = device.getLatestFix();
			if (latestFix) {
				updateAircraftMarkerFromDevice(device, latestFix);
			}
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
		};
	});

	async function loadGoogleMapsScript(): Promise<void> {
		const loader = new Loader({
			apiKey: GOOGLE_MAPS_API_KEY,
			version: 'weekly'
		});

		// Import the required libraries
		await loader.importLibrary('maps');
		await loader.importLibrary('geometry');
		await loader.importLibrary('marker');
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

		// Initialize map with saved or default state
		map = new google.maps.Map(mapContainer, {
			mapId: 'SOAR_MAP', // Required for AdvancedMarkerElement
			center: mapState.center,
			zoom: mapState.zoom,
			mapTypeId: window.google.maps.MapTypeId.TERRAIN,
			mapTypeControl: true,
			mapTypeControlOptions: {
				style: window.google.maps.MapTypeControlStyle.HORIZONTAL_BAR,
				position: window.google.maps.ControlPosition.LEFT_BOTTOM
			},
			zoomControl: false,
			scaleControl: true,
			streetViewControl: true,
			streetViewControlOptions: {
				position: window.google.maps.ControlPosition.RIGHT_TOP
			},
			fullscreenControl: true,
			fullscreenControlOptions: {
				position: window.google.maps.ControlPosition.RIGHT_TOP
			}
		});

		// Add event listeners for viewport changes
		map.addListener('zoom_changed', () => {
			setTimeout(checkAndUpdateAirports, 100); // Small delay to ensure bounds are updated
			// Update aircraft marker scaling on zoom change
			updateAllAircraftMarkersScale();
			// Update area tracker availability and subscriptions
			updateAreaTrackerAvailability();
			if (areaTrackerActive) {
				// Hybrid approach: Fetch immediate snapshot then update WebSocket subscriptions
				setTimeout(async () => {
					await fetchAndDisplayDevicesInViewport();
					updateAreaSubscriptions();
				}, 100);
			}
			// Save map state after zoom changes
			saveMapState();
		});

		map.addListener('dragend', async () => {
			checkAndUpdateAirports();
			// Update area subscriptions after panning
			if (areaTrackerActive) {
				// Hybrid approach: Fetch immediate snapshot then update WebSocket subscriptions
				await fetchAndDisplayDevicesInViewport();
				updateAreaSubscriptions();
			}
			// Save map state after panning
			saveMapState();
		});

		// Initial check for airports
		setTimeout(checkAndUpdateAirports, 1000); // Give map time to fully initialize

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
				nw_lat: nwLat.toString(),
				nw_lng: nwLng.toString(),
				se_lat: seLat.toString(),
				se_lng: seLng.toString(),
				limit: '100' // Limit to avoid too many markers
			});

			const data = await serverCall(`/airports?${params}`);
			// Type guard to ensure we have the correct data structure
			if (!Array.isArray(data)) {
				throw new Error('Invalid response format: expected array');
			}

			airports = data.filter((airport: unknown): airport is AirportView => {
				return (
					typeof airport === 'object' &&
					airport !== null &&
					'id' in airport &&
					'ident' in airport &&
					'name' in airport &&
					'latitude_deg' in airport &&
					'longitude_deg' in airport
				);
			});

			displayAirportsOnMap();
		} catch (error) {
			console.error('Error fetching airports:', error);
		}
	}

	function displayAirportsOnMap(): void {
		// Clear existing airport markers
		clearAirportMarkers();

		airports.forEach((airport) => {
			if (!airport.latitude_deg || !airport.longitude_deg) return;

			// Convert BigDecimal strings to numbers with validation
			const lat = parseFloat(airport.latitude_deg);
			const lng = parseFloat(airport.longitude_deg);

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

	function handleOrientationChange(event: DeviceOrientationEvent): void {
		if (event.alpha !== null) {
			isCompassActive = true;
			// Normalize the heading to ensure it's always between 0 and 360
			deviceHeading = event.alpha;
			displayHeading = Math.round(deviceHeading);

			// Calculate the new compass heading (inverted to keep north arrow pointing north)
			let newHeading = -deviceHeading;

			// Normalize to 0-360 range
			newHeading = ((newHeading % 360) + 360) % 360;

			// Calculate the shortest rotation path
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

			// Update previous heading for next comparison (using normalized value)
			previousCompassHeading = newHeading;
		}
	}

	function updateMarkerScale(markerContent: HTMLElement, zoom: number): void {
		if (!markerContent) return;

		// Calculate scale based on zoom level
		// Zoom levels typically range from 1 (world) to 20+ (street level)
		// We want markers to be very small at low zoom and normal size at high zoom
		let scale: number;

		if (zoom <= 4) {
			// Very zoomed out (world/continental view) - minimum size
			scale = 0.3;
		} else if (zoom <= 8) {
			// Country/state level - small size
			scale = 0.4 + (zoom - 4) * 0.15; // 0.4 to 1.0
		} else if (zoom <= 12) {
			// Regional level - medium size
			scale = 1.0 + (zoom - 8) * 0.1; // 1.0 to 1.4
		} else {
			// City/street level - full size
			scale = 1.4 + Math.min(zoom - 12, 6) * 0.05; // 1.4 to 1.7 max
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

	function updateAircraftMarkerFromDevice(device: Device, latestFix: Fix): void {
		console.log('[MARKER] updateAircraftMarkerFromDevice called:', {
			deviceId: device.id,
			registration: device.registration,
			latestFix: {
				lat: latestFix.latitude,
				lng: latestFix.longitude,
				alt: latestFix.altitude_feet,
				timestamp: latestFix.timestamp
			},
			mapExists: !!map
		});

		if (!map) {
			console.warn('[MARKER] No map available for marker update');
			return;
		}

		const deviceKey = device.id;
		if (!deviceKey) {
			console.warn('[MARKER] No device ID available');
			return;
		}

		// Update latest fix for this device
		latestFixes.set(deviceKey, latestFix);
		console.log('[MARKER] Updated latest fix for device:', deviceKey);

		// Get or create marker for this aircraft
		let marker = aircraftMarkers.get(deviceKey);

		if (!marker) {
			console.log('[MARKER] Creating new marker for device:', deviceKey);
			// Create new aircraft marker with device info
			marker = createAircraftMarkerFromDevice(device, latestFix);
			aircraftMarkers.set(deviceKey, marker);
			console.log('[MARKER] New marker created and stored. Total markers:', aircraftMarkers.size);
		} else {
			console.log('[MARKER] Updating existing marker for device:', deviceKey);
			// Update existing marker position and info
			updateAircraftMarkerPositionFromDevice(marker, device, latestFix);
		}

		// Update trail for this aircraft
		updateAircraftTrail(device);
	}

	function createAircraftMarkerFromDevice(
		device: Device,
		fix: Fix
	): google.maps.marker.AdvancedMarkerElement {
		console.log('[MARKER] Creating marker for device:', {
			deviceId: device.id,
			registration: device.registration,
			address: device.address,
			position: { lat: fix.latitude, lng: fix.longitude },
			track: fix.track_degrees
		});

		// Create aircraft icon with rotation based on track
		const markerContent = document.createElement('div');
		markerContent.className = 'aircraft-marker';

		// Aircraft icon (rotated based on track) - using a more visible SVG plane
		const aircraftIcon = document.createElement('div');
		aircraftIcon.className = 'aircraft-icon';

		// Create SVG airplane icon that's more visible and oriented correctly
		aircraftIcon.innerHTML = `
			<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
				<path d="M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z"/>
			</svg>
		`;

		// Rotate icon based on track degrees (default to 0 if not available)
		const track = fix.track_degrees || 0;
		aircraftIcon.style.transform = `rotate(${track}deg)`;
		console.log('[MARKER] Set icon rotation to:', track, 'degrees');

		// Info label below the icon - show proper aircraft information
		const infoLabel = document.createElement('div');
		infoLabel.className = 'aircraft-label';

		// Use proper device registration, fallback to address
		const tailNumber = device.registration || device.address || 'Unknown';
		const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitude_feet, fix.timestamp);
		const aircraftModel = device.aircraft_model;

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
		const title = device.aircraft_model
			? `${tailNumber} (${device.aircraft_model}) - ${altitudeText} - Last seen: ${fullTimestamp}`
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
			handleAircraftClick(device);
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
		device: Device,
		fix: Fix
	): void {
		console.log('[MARKER] Updating existing marker position:', {
			deviceId: device.id,
			oldPosition: marker.position,
			newPosition: { lat: fix.latitude, lng: fix.longitude }
		});

		// Update position
		marker.position = { lat: fix.latitude, lng: fix.longitude };

		// Update icon rotation and label
		const markerContent = marker.content as HTMLElement;
		if (markerContent) {
			const aircraftIcon = markerContent.querySelector('.aircraft-icon') as HTMLElement;
			const tailDiv = markerContent.querySelector('.aircraft-tail') as HTMLElement;
			const altDiv = markerContent.querySelector('.aircraft-altitude') as HTMLElement;

			if (aircraftIcon) {
				const track = fix.track_degrees || 0;
				aircraftIcon.style.transform = `rotate(${track}deg)`;
				console.log('[MARKER] Updated icon rotation to:', track, 'degrees');
			}

			if (tailDiv && altDiv) {
				// Use proper device registration, fallback to address
				const tailNumber = device.registration || device.address || 'Unknown';
				const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitude_feet, fix.timestamp);
				const aircraftModel = device.aircraft_model;

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
		const { altitudeText } = formatAltitudeWithTime(fix.altitude_feet, fix.timestamp);
		const fullTimestamp = dayjs(fix.timestamp).format('YYYY-MM-DD HH:mm:ss UTC');
		const title = device.aircraft_model
			? `${device.registration || device.address} (${device.aircraft_model}) - ${altitudeText} - Last seen: ${fullTimestamp}`
			: `${device.registration || device.address} - ${altitudeText} - Last seen: ${fullTimestamp}`;

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

	// Aircraft trail functions
	function updateAircraftTrail(device: Device): void {
		if (!map || currentSettings.trailLength === 0) {
			// Remove trail if disabled
			clearTrailForDevice(device.id);
			return;
		}

		const fixes = device.getRecentFixes(24); // Get last 24 hours of fixes
		const trailFixCount = Math.min(fixes.length, currentSettings.trailLength);

		if (trailFixCount < 2) {
			// Need at least 2 points to draw a trail
			clearTrailForDevice(device.id);
			return;
		}

		// Get the fixes to display (most recent N fixes)
		const trailFixes = fixes.slice(0, trailFixCount);

		// Clear existing trail
		clearTrailForDevice(device.id);

		// Create polyline segments with progressive transparency
		const polylines: google.maps.Polyline[] = [];
		for (let i = 0; i < trailFixes.length - 1; i++) {
			// Calculate opacity: newest segment (i=0) = 0.7, oldest = 0.2
			const segmentOpacity = 0.7 - (i / (trailFixes.length - 2)) * 0.5;

			const segment = new google.maps.Polyline({
				path: [
					{ lat: trailFixes[i].latitude, lng: trailFixes[i].longitude },
					{ lat: trailFixes[i + 1].latitude, lng: trailFixes[i + 1].longitude }
				],
				geodesic: true,
				strokeColor: '#ef4444', // Red color matching aircraft marker
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
			const opacity = 0.7 - (index / (trailFixCount - 1)) * 0.5;

			const dot = new google.maps.Circle({
				center: { lat: fix.latitude, lng: fix.longitude },
				radius: 10, // 10 meters radius
				strokeColor: '#ef4444',
				strokeOpacity: opacity,
				strokeWeight: 1,
				fillColor: '#ef4444',
				fillOpacity: opacity * 0.5,
				map: map
			});

			dots.push(dot);
		});

		// Store trail data
		aircraftTrails.set(device.id, { polylines, dots });
	}

	function clearTrailForDevice(deviceId: string): void {
		const trail = aircraftTrails.get(deviceId);
		if (trail) {
			trail.polylines.forEach((polyline) => polyline.setMap(null));
			trail.dots.forEach((dot) => dot.setMap(null));
			aircraftTrails.delete(deviceId);
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
		if (!areaTrackerActive || !areaTrackerAvailable) return;

		const visibleSquares = getVisibleLatLonSquares();
		const newSubscriptions = new SvelteSet<string>();

		// Create subscription keys for visible squares
		visibleSquares.forEach((square) => {
			const key = `area.${square.lat}.${square.lon}`;
			newSubscriptions.add(key);
		});

		// Find squares to unsubscribe from (no longer visible)
		const toUnsubscribe = new SvelteSet<string>();
		currentAreaSubscriptions.forEach((key) => {
			if (!newSubscriptions.has(key)) {
				toUnsubscribe.add(key);
			}
		});

		// Find squares to subscribe to (newly visible)
		const toSubscribe = new SvelteSet<string>();
		newSubscriptions.forEach((key) => {
			if (!currentAreaSubscriptions.has(key)) {
				toSubscribe.add(key);
			}
		});

		// Unsubscribe from areas no longer visible
		toUnsubscribe.forEach((key) => {
			const [, lat, lon] = key.split('.');
			unsubscribeFromArea(parseInt(lat), parseInt(lon));
		});

		// Subscribe to newly visible areas
		toSubscribe.forEach((key) => {
			const [, lat, lon] = key.split('.');
			subscribeToArea(parseInt(lat), parseInt(lon));
		});

		// Update current subscriptions
		currentAreaSubscriptions = newSubscriptions;

		console.log(
			`[AREA TRACKER] Updated subscriptions: ${toSubscribe.size} new, ${toUnsubscribe.size} removed, ${currentAreaSubscriptions.size} total`
		);

		// Explicitly update debug status to ensure WebSocket status panel shows area subscriptions
		debugStatus.update((current) => ({
			...current,
			activeAreaSubscriptions: currentAreaSubscriptions.size
		}));
	}

	function clearAreaSubscriptions(): void {
		currentAreaSubscriptions.forEach((key) => {
			const [, lat, lon] = key.split('.');
			unsubscribeFromArea(parseInt(lat), parseInt(lon));
		});
		currentAreaSubscriptions.clear();
		console.log('[AREA TRACKER] Cleared all area subscriptions');

		// Explicitly update debug status to ensure WebSocket status panel updates
		debugStatus.update((current) => ({
			...current,
			activeAreaSubscriptions: 0
		}));
	}

	function subscribeToArea(latitude: number, longitude: number): void {
		const message = {
			action: 'subscribe',
			type: 'area' as const,
			latitude,
			longitude
		};
		console.log('[AREA TRACKER] Subscribe to area:', message);
		fixFeed.sendWebSocketMessage(message);
	}

	function unsubscribeFromArea(latitude: number, longitude: number): void {
		const message = {
			action: 'unsubscribe',
			type: 'area' as const,
			latitude,
			longitude
		};
		console.log('[AREA TRACKER] Unsubscribe from area:', message);
		fixFeed.sendWebSocketMessage(message);
	}

	async function fetchAndDisplayDevicesInViewport(): Promise<void> {
		if (!map) return;

		const bounds = map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		try {
			console.log('[REST] Fetching devices in viewport...');

			// Fetch from REST endpoint
			const devicesWithFixes = await fixFeed.fetchDevicesInBoundingBox(
				sw.lat(), // latMin
				ne.lat(), // latMax
				sw.lng(), // lonMin
				ne.lng() // lonMax
			);

			console.log(`[REST] Received ${devicesWithFixes.length} devices`);

			// Process each device and add to registry
			for (const deviceData of devicesWithFixes) {
				// Add fixes to device registry
				for (const fix of deviceData.recent_fixes) {
					await deviceRegistry.addFixToDevice(fix, false);
				}

				// Update device info with aircraft data
				await deviceRegistry.updateDeviceInfo(
					deviceData.device.id,
					deviceData.device,
					deviceData.aircraft_registration,
					deviceData.aircraft_model
				);
			}

			console.log('[REST] Devices loaded, WebSocket subscriptions will provide live updates');
		} catch (error) {
			console.error('[REST] Failed to fetch devices in viewport:', error);
		}
	}
</script>

<svelte:head>
	<title>Operations - Glider Flights</title>
</svelte:head>

<div class="fixed inset-0 w-full" style="top: 64px;">
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

		<!-- Area Tracker Button -->
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

		<!-- Settings Button -->
		<button class="location-btn" onclick={() => (showSettingsModal = true)} title="Settings">
			<Settings size={20} />
		</button>
	</div>

	<!-- Compass Rose -->
	{#if isCompassActive && currentSettings.showCompassRose}
		<div class="compass-container absolute bottom-8 left-1/2 z-10 -translate-x-1/2 transform">
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
<AircraftStatusModal
	bind:showModal={showAircraftStatusModal}
	bind:selectedDevice={selectedAircraft}
/>

<!-- Airport Modal -->
<AirportModal bind:showModal={showAirportModal} bind:selectedAirport />

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
		background: rgba(239, 68, 68, 0.75); /* Changed to 75% opacity */
		border: 2px solid #dc2626;
		border-radius: 6px;
		padding: 4px 8px;
		margin-top: 6px;
		box-shadow: 0 3px 8px rgba(0, 0, 0, 0.4);
		min-width: 60px;
		text-align: center;
		transition: all 0.2s ease;
	}

	:global(.aircraft-marker:hover .aircraft-label) {
		background: rgba(239, 68, 68, 1); /* Fully opaque on hover */
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.6);
	}

	:global(.aircraft-tail) {
		font-size: 12px;
		font-weight: 700;
		color: white;
		line-height: 1.2;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
	}

	:global(.aircraft-altitude) {
		font-size: 10px;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.9);
		line-height: 1.1;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
		margin-top: 1px;
	}
</style>
