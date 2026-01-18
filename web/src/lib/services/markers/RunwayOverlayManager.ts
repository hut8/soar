/**
 * RunwayOverlayManager - Manages runway polygon overlays on a Google Map
 *
 * Handles displaying and clearing runway polygons and endpoint markers.
 * Runway data comes from the AirportMarkerManager (runways are part of airport data).
 */

import { getLogger } from '$lib/logging';
import type { Runway } from '$lib/types';

const logger = getLogger(['soar', 'RunwayOverlayManager']);

/** Maximum viewport area (in square miles) at which runways are displayed */
const MAX_VIEWPORT_AREA_FOR_RUNWAYS = 5000;

/** Z-index for runway polygons (below airspaces) */
const RUNWAY_POLYGON_Z_INDEX = 40;

/** Z-index for runway endpoint markers */
const RUNWAY_ENDPOINT_Z_INDEX = 45;

/** Debounce delay for runway updates */
const DEBOUNCE_MS = 500;

export interface RunwayOverlayManagerOptions {
	/** Function to get runways from the airport manager */
	getRunways: () => Runway[];
}

export class RunwayOverlayManager {
	private map: google.maps.Map | null = null;
	private polygons: google.maps.Polygon[] = [];
	private endpointMarkers: google.maps.Circle[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: RunwayOverlayManagerOptions;

	constructor(options: RunwayOverlayManagerOptions) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: google.maps.Map): void {
		this.map = map;
	}

	/**
	 * Check viewport and update runway visibility/overlays
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether runway overlays are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates by 500ms to prevent excessive updates
		this.debounceTimer = setTimeout(() => {
			const shouldShow = viewportArea < MAX_VIEWPORT_AREA_FOR_RUNWAYS && settingEnabled;

			if (shouldShow !== this.shouldShow) {
				this.shouldShow = shouldShow;

				if (this.shouldShow) {
					this.displayRunways();
				} else {
					this.clear();
				}
			} else if (this.shouldShow) {
				// Still showing runways, update them
				this.displayRunways();
			}

			this.debounceTimer = null;
		}, DEBOUNCE_MS);
	}

	/**
	 * Force refresh the runway display (called when airport data changes)
	 */
	refresh(): void {
		if (this.shouldShow) {
			this.displayRunways();
		}
	}

	/**
	 * Force clear all overlays and reset state
	 */
	clear(): void {
		this.clearOverlays();
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
		this.clearOverlays();
		this.map = null;
	}

	/**
	 * Display runway overlays using data from the airport manager
	 */
	private displayRunways(): void {
		const runways = this.options.getRunways();
		this.displayRunwaysOnMap(runways);
	}

	/**
	 * Display polygon overlays for all provided runways
	 */
	private displayRunwaysOnMap(runways: Runway[]): void {
		// Clear existing overlays first
		this.clearOverlays();

		if (!this.map) return;

		runways.forEach((runway) => {
			// Only display if we have a valid polyline (4 corner points)
			if (runway.polyline && runway.polyline.length === 4) {
				// Convert [lat, lon] array to Google Maps LatLngLiteral
				const path = runway.polyline.map((coord) => ({
					lat: coord[0],
					lng: coord[1]
				}));

				// Create runway rectangle polygon with semi-transparent blue fill and thin orange outline
				const polygon = new google.maps.Polygon({
					paths: path,
					strokeColor: '#fb923c', // Orange (Tailwind orange-400)
					strokeOpacity: 1.0,
					strokeWeight: 2,
					fillColor: '#3b82f6', // Blue (Tailwind blue-500)
					fillOpacity: 0.4, // Semi-transparent
					map: this.map,
					zIndex: RUNWAY_POLYGON_Z_INDEX
				});

				this.polygons.push(polygon);
			}

			// Add endpoint markers (small dots at each end of runway)
			const endpointColor = '#F59E0B'; // Amber
			const endpointRadius = 18; // meters

			// Low end marker
			if (runway.low.latitudeDeg !== null && runway.low.longitudeDeg !== null) {
				const lowMarker = new google.maps.Circle({
					center: { lat: runway.low.latitudeDeg, lng: runway.low.longitudeDeg },
					radius: endpointRadius,
					strokeColor: endpointColor,
					strokeOpacity: 1,
					strokeWeight: 2,
					fillColor: endpointColor,
					fillOpacity: 0.8,
					map: this.map,
					zIndex: RUNWAY_ENDPOINT_Z_INDEX
				});
				this.endpointMarkers.push(lowMarker);
			}

			// High end marker
			if (runway.high.latitudeDeg !== null && runway.high.longitudeDeg !== null) {
				const highMarker = new google.maps.Circle({
					center: { lat: runway.high.latitudeDeg, lng: runway.high.longitudeDeg },
					radius: endpointRadius,
					strokeColor: endpointColor,
					strokeOpacity: 1,
					strokeWeight: 2,
					fillColor: endpointColor,
					fillOpacity: 0.8,
					map: this.map,
					zIndex: RUNWAY_ENDPOINT_Z_INDEX
				});
				this.endpointMarkers.push(highMarker);
			}
		});

		logger.debug(
			'[RUNWAYS] Displayed {runways} runways ({polygons} polygons, {markers} endpoint markers)',
			{
				runways: runways.length,
				polygons: this.polygons.length,
				markers: this.endpointMarkers.length
			}
		);
	}

	/**
	 * Clear all overlays from the map
	 */
	private clearOverlays(): void {
		this.polygons.forEach((polygon) => {
			polygon.setMap(null);
		});
		this.polygons = [];

		this.endpointMarkers.forEach((marker) => {
			marker.setMap(null);
		});
		this.endpointMarkers = [];
	}
}
