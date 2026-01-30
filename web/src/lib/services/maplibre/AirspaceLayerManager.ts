/**
 * AirspaceLayerManager - Manages airspace polygon layers on a MapLibre map
 *
 * Handles fetching, displaying, and clearing airspace polygons with automatic
 * visibility based on zoom level and viewport area. Uses MapLibre's native
 * GeoJSON source and fill/line layers for efficient rendering.
 */

import type maplibregl from 'maplibre-gl';
import { serverCall } from '$lib/api/server';
import { getLogger } from '$lib/logging';
import type { Airspace, AirspaceFeatureCollection, DataResponse } from '$lib/types';

const logger = getLogger(['soar', 'maplibre', 'AirspaceLayerManager']);

/** Maximum viewport area (in square miles) at which airspaces are displayed */
const MAX_VIEWPORT_AREA_FOR_AIRSPACES = 100000;

/** Debounce delay for airspace updates (ms) */
const DEBOUNCE_MS = 500;

/** Source and layer IDs */
const SOURCE_ID = 'airspaces-source';
const FILL_LAYER_ID = 'airspaces-fill';
const LINE_LAYER_ID = 'airspaces-line';

export interface AirspaceLayerManagerOptions {
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

export class AirspaceLayerManager {
	private map: maplibregl.Map | null = null;
	private airspaces: Airspace[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: AirspaceLayerManagerOptions;
	private layersAdded: boolean = false;

	constructor(options: AirspaceLayerManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: maplibregl.Map): void {
		this.map = map;

		// Set up click handler if callback provided
		if (this.options.onAirspaceClick) {
			this.setupClickHandler();
		}
	}

	/**
	 * Check viewport and update airspace visibility/layers
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether airspace overlays are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates to prevent excessive API calls
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
	 * Force clear all layers and reset state
	 */
	clear(): void {
		this.removeLayers();
		this.airspaces = [];
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
		this.removeLayers();
		this.airspaces = [];
		this.map = null;
	}

	/**
	 * Fetch airspaces in the current viewport and display them
	 */
	private async fetchAndDisplay(): Promise<void> {
		if (!this.map) return;

		const bounds = this.map.getBounds();

		try {
			const params = new URLSearchParams({
				west: bounds.getWest().toString(),
				south: bounds.getSouth().toString(),
				east: bounds.getEast().toString(),
				north: bounds.getNorth().toString(),
				limit: '500'
			});

			const response = await serverCall<DataResponse<AirspaceFeatureCollection>>(
				`/airspaces?${params}`
			);
			const data = response.data;

			if (data && data.type === 'FeatureCollection' && Array.isArray(data.features)) {
				this.airspaces = data.features;
				this.updateLayers();
			}
		} catch (error) {
			logger.error('Error fetching airspaces: {error}', { error });
		}
	}

	/**
	 * Update or create the map layers with current airspace data
	 */
	private updateLayers(): void {
		if (!this.map) return;

		// Convert airspaces to GeoJSON with color property
		const geojson = this.createGeoJson();

		// Check if source exists
		const existingSource = this.map.getSource(SOURCE_ID) as maplibregl.GeoJSONSource | undefined;

		if (existingSource) {
			// Update existing source
			existingSource.setData(geojson);
		} else {
			// Add new source and layers
			this.addSourceAndLayers(geojson);
		}

		logger.debug('[AIRSPACES] Displayed {count} airspaces', { count: this.airspaces.length });
	}

	/**
	 * Create GeoJSON FeatureCollection with color properties
	 */
	private createGeoJson(): GeoJSON.FeatureCollection {
		return {
			type: 'FeatureCollection',
			features: this.airspaces.map((airspace, index) => ({
				type: 'Feature' as const,
				geometry: airspace.geometry as GeoJSON.Polygon | GeoJSON.MultiPolygon,
				properties: {
					index,
					color: getAirspaceColor(airspace.properties.airspaceClass)
				}
			}))
		};
	}

	/**
	 * Add source and layers to the map
	 */
	private addSourceAndLayers(geojson: GeoJSON.FeatureCollection): void {
		if (!this.map) return;

		// Add source
		this.map.addSource(SOURCE_ID, {
			type: 'geojson',
			data: geojson
		});

		// Add fill layer
		this.map.addLayer({
			id: FILL_LAYER_ID,
			type: 'fill',
			source: SOURCE_ID,
			paint: {
				'fill-color': ['get', 'color'],
				'fill-opacity': 0.15
			}
		});

		// Add line layer for borders
		this.map.addLayer({
			id: LINE_LAYER_ID,
			type: 'line',
			source: SOURCE_ID,
			paint: {
				'line-color': ['get', 'color'],
				'line-width': 2,
				'line-opacity': 0.8
			}
		});

		this.layersAdded = true;
	}

	/**
	 * Remove layers and source from the map
	 */
	private removeLayers(): void {
		if (!this.map || !this.layersAdded) return;

		// Remove layers first
		if (this.map.getLayer(LINE_LAYER_ID)) {
			this.map.removeLayer(LINE_LAYER_ID);
		}
		if (this.map.getLayer(FILL_LAYER_ID)) {
			this.map.removeLayer(FILL_LAYER_ID);
		}

		// Then remove source
		if (this.map.getSource(SOURCE_ID)) {
			this.map.removeSource(SOURCE_ID);
		}

		this.layersAdded = false;
	}

	/**
	 * Set up click handler for airspace polygons
	 */
	private setupClickHandler(): void {
		if (!this.map || !this.options.onAirspaceClick) return;

		const clickHandler = this.options.onAirspaceClick;

		this.map.on('click', FILL_LAYER_ID, (e) => {
			if (!e.features || e.features.length === 0) return;

			// Check if an aircraft or airport was clicked - if so, don't show airspace modal
			// Aircraft and airport layers have higher priority for click handling
			const aircraftFeatures = this.map!.queryRenderedFeatures(e.point, {
				layers: ['aircraft-markers']
			});
			if (aircraftFeatures.length > 0) {
				return; // Aircraft clicked, let aircraft handler deal with it
			}

			const airportLayers = ['airports-symbols', 'airports-symbols-circle'].filter((id) =>
				this.map!.getLayer(id)
			);
			if (airportLayers.length > 0) {
				const airportFeatures = this.map!.queryRenderedFeatures(e.point, {
					layers: airportLayers
				});
				if (airportFeatures.length > 0) {
					return; // Airport clicked, let airport handler deal with it
				}
			}

			const feature = e.features[0];
			const airspaceIndex = feature.properties?.index;

			if (airspaceIndex !== undefined && airspaceIndex !== null) {
				const airspace = this.airspaces[airspaceIndex];
				if (airspace) {
					clickHandler(airspace);
				}
			}
		});

		// Change cursor on hover
		this.map.on('mouseenter', FILL_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = 'pointer';
		});

		this.map.on('mouseleave', FILL_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = '';
		});
	}
}
