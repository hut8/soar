/**
 * AirportMarkerManager - Manages airport markers on a Google Map
 *
 * Handles fetching, displaying, and clearing airport markers with automatic
 * visibility based on zoom level and viewport area.
 */

import { serverCall } from '$lib/api/server';
import { getLogger } from '$lib/logging';
import type { Airport, DataListResponse, Runway } from '$lib/types';

const logger = getLogger(['soar', 'AirportMarkerManager']);

/** Maximum viewport area (in square miles) at which airports are displayed */
const MAX_VIEWPORT_AREA_FOR_AIRPORTS = 10000;

/** Z-index for airport markers (below aircraft at 1000) */
const AIRPORT_MARKER_Z_INDEX = 100;

export interface AirportMarkerManagerOptions {
	/** Callback when an airport marker is clicked */
	onAirportClick?: (airport: Airport) => void;
	/** Callback when airports are loaded (for runway display) */
	onAirportsLoaded?: (airports: Airport[]) => void;
}

export class AirportMarkerManager {
	private map: google.maps.Map | null = null;
	private airports: Airport[] = [];
	private markers: google.maps.marker.AdvancedMarkerElement[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: AirportMarkerManagerOptions;

	constructor(options: AirportMarkerManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: google.maps.Map): void {
		this.map = map;
	}

	/**
	 * Get the currently loaded airports
	 */
	getAirports(): Airport[] {
		return this.airports;
	}

	/**
	 * Get all runways from loaded airports
	 */
	getRunways(): Runway[] {
		return this.airports.flatMap((airport) => airport.runways || []);
	}

	/**
	 * Check viewport and update airport visibility/markers
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether airport markers are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates by 100ms to prevent excessive API calls
		this.debounceTimer = setTimeout(() => {
			const shouldShow = viewportArea < MAX_VIEWPORT_AREA_FOR_AIRPORTS && settingEnabled;

			if (shouldShow !== this.shouldShow) {
				this.shouldShow = shouldShow;

				if (this.shouldShow) {
					this.fetchAndDisplay();
				} else {
					this.clear();
				}
			} else if (this.shouldShow) {
				// Still showing airports, update them for the new viewport
				this.fetchAndDisplay();
			}

			this.debounceTimer = null;
		}, 100);
	}

	/**
	 * Force clear all markers and reset state
	 */
	clear(): void {
		this.clearMarkers();
		this.airports = [];
		this.shouldShow = false;
	}

	/**
	 * Dispose of the manager and clean up resources
	 */
	dispose(): void {
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
			this.debounceTimer = null;
		}
		this.clearMarkers();
		this.airports = [];
		this.map = null;
	}

	/**
	 * Fetch airports in the current viewport and display them
	 */
	private async fetchAndDisplay(): Promise<void> {
		if (!this.map) return;

		const bounds = this.map.getBounds();
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
			logger.warn(
				'Invalid bounding box: northwest latitude must be greater than southeast latitude'
			);
			return;
		}

		// Validate latitude bounds
		if (nwLat > 90 || nwLat < -90 || seLat > 90 || seLat < -90) {
			logger.warn('Invalid latitude values in bounding box');
			return;
		}

		// Validate longitude bounds (allow wrapping around international date line)
		if (nwLng < -180 || nwLng > 180 || seLng < -180 || seLng > 180) {
			logger.warn('Invalid longitude values in bounding box');
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
			this.airports = response.data || [];

			this.displayMarkers();

			// Notify callback that airports were loaded
			if (this.options.onAirportsLoaded) {
				this.options.onAirportsLoaded(this.airports);
			}
		} catch (error) {
			logger.error('Error fetching airports: {error}', { error });
		}
	}

	/**
	 * Display markers for all loaded airports
	 */
	private displayMarkers(): void {
		// Clear existing markers first
		this.clearMarkers();

		if (!this.map) return;

		this.airports.forEach((airport) => {
			if (!airport.latitudeDeg || !airport.longitudeDeg) return;

			// Convert BigDecimal strings to numbers with validation
			const lat = parseFloat(airport.latitudeDeg);
			const lng = parseFloat(airport.longitudeDeg);

			// Validate coordinates are valid numbers and within expected ranges
			if (isNaN(lat) || isNaN(lng) || lat < -90 || lat > 90 || lng < -180 || lng > 180) {
				logger.warn('Invalid coordinates for airport {ident}: {lat}, {lng}', {
					ident: airport.ident,
					lat,
					lng
				});
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
				map: this.map,
				title: `${airport.name} (${airport.ident})`,
				content: markerContent,
				zIndex: AIRPORT_MARKER_Z_INDEX
			});

			// Add click listener if callback provided
			if (this.options.onAirportClick) {
				const clickHandler = this.options.onAirportClick;
				marker.addListener('click', () => {
					clickHandler(airport);
				});
			}

			this.markers.push(marker);
		});
	}

	/**
	 * Clear all markers from the map
	 */
	private clearMarkers(): void {
		this.markers.forEach((marker) => {
			marker.map = null;
		});
		this.markers = [];
	}
}
