/**
 * AirspaceOverlayManager - Manages airspace polygon overlays on a Google Map
 *
 * Handles fetching, displaying, and clearing airspace polygons with automatic
 * visibility based on zoom level and viewport area.
 */

import { serverCall } from '$lib/api/server';
import { getLogger } from '$lib/logging';
import type { Airspace, AirspaceFeatureCollection, DataResponse } from '$lib/types';

const logger = getLogger(['soar', 'AirspaceOverlayManager']);

/** Maximum viewport area (in square miles) at which airspaces are displayed */
const MAX_VIEWPORT_AREA_FOR_AIRSPACES = 100000;

/** Z-index for airspace polygons (below airports and receivers) */
const AIRSPACE_POLYGON_Z_INDEX = 50;

/** Debounce delay for airspace updates (longer than markers due to complexity) */
const DEBOUNCE_MS = 500;

export interface AirspaceOverlayManagerOptions {
	/** Callback when an airspace polygon is clicked */
	onAirspaceClick?: (airspace: Airspace) => void;
}

/**
 * Get color for airspace based on its class
 */
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

export class AirspaceOverlayManager {
	private map: google.maps.Map | null = null;
	private polygons: google.maps.Polygon[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: AirspaceOverlayManagerOptions;

	constructor(options: AirspaceOverlayManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: google.maps.Map): void {
		this.map = map;
	}

	/**
	 * Check viewport and update airspace visibility/overlays
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether airspace overlays are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates by 500ms to prevent excessive API calls
		this.debounceTimer = setTimeout(() => {
			const shouldShow = viewportArea < MAX_VIEWPORT_AREA_FOR_AIRSPACES && settingEnabled;

			if (shouldShow !== this.shouldShow) {
				this.shouldShow = shouldShow;

				if (this.shouldShow) {
					this.fetchAndDisplay();
				} else {
					this.clear();
				}
			} else if (this.shouldShow) {
				// Still showing airspaces, update them for the new viewport
				this.fetchAndDisplay();
			}

			this.debounceTimer = null;
		}, DEBOUNCE_MS);
	}

	/**
	 * Force clear all polygons and reset state
	 */
	clear(): void {
		this.clearPolygons();
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
		this.clearPolygons();
		this.map = null;
	}

	/**
	 * Fetch airspaces in the current viewport and display them
	 */
	private async fetchAndDisplay(): Promise<void> {
		if (!this.map) return;

		const bounds = this.map.getBounds();
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
				this.displayPolygons(data.features);
			}
		} catch (error) {
			logger.error('Error fetching airspaces: {error}', { error });
		}
	}

	/**
	 * Display polygon overlays for all loaded airspaces
	 */
	private displayPolygons(airspaces: Airspace[]): void {
		// Clear existing polygons first
		this.clearPolygons();

		if (!this.map) return;

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
					map: this.map,
					zIndex: AIRSPACE_POLYGON_Z_INDEX
				});

				// Add click listener if callback provided
				if (this.options.onAirspaceClick) {
					const clickHandler = this.options.onAirspaceClick;
					polygon.addListener('click', () => {
						clickHandler(airspace);
					});
				}

				this.polygons.push(polygon);
			});
		});

		logger.debug('[AIRSPACES] Displayed {airspaces} airspaces ({polygons} polygons)', {
			airspaces: airspaces.length,
			polygons: this.polygons.length
		});
	}

	/**
	 * Clear all polygons from the map
	 */
	private clearPolygons(): void {
		this.polygons.forEach((polygon) => {
			polygon.setMap(null);
		});
		this.polygons = [];
	}
}
