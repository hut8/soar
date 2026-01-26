/**
 * RunwayLayerManager - Manages runway polygon layers on a MapLibre map
 *
 * Handles displaying and clearing runway polygons and endpoint markers.
 * Runway data comes from the AirportLayerManager (runways are part of airport data).
 */

import type maplibregl from 'maplibre-gl';
import { getLogger } from '$lib/logging';
import type { Runway } from '$lib/types';

const logger = getLogger(['soar', 'maplibre', 'RunwayLayerManager']);

/** Maximum viewport area (in square miles) at which runways are displayed */
const MAX_VIEWPORT_AREA_FOR_RUNWAYS = 5000;

/** Debounce delay for runway updates */
const DEBOUNCE_MS = 500;

/** Source and layer IDs */
const SOURCE_ID = 'runways-source';
const FILL_LAYER_ID = 'runways-fill';
const LINE_LAYER_ID = 'runways-line';
const ENDPOINTS_SOURCE_ID = 'runway-endpoints-source';
const ENDPOINTS_LAYER_ID = 'runway-endpoints';

export interface RunwayLayerManagerOptions {
	/** Function to get runways from the airport manager */
	getRunways: () => Runway[];
}

export class RunwayLayerManager {
	private map: maplibregl.Map | null = null;
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: RunwayLayerManagerOptions;
	private layersAdded: boolean = false;

	constructor(options: RunwayLayerManagerOptions) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: maplibregl.Map): void {
		this.map = map;
	}

	/**
	 * Check viewport and update runway visibility/layers
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether runway overlays are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates
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
	 * Force clear all layers and reset state
	 */
	clear(): void {
		this.removeLayers();
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
		this.map = null;
	}

	/**
	 * Display runway layers using data from the airport manager
	 */
	private displayRunways(): void {
		if (!this.map) return;

		const runways = this.options.getRunways();
		const { polygonGeojson, endpointsGeojson } = this.createGeoJson(runways);

		// Check if sources exist
		const existingSource = this.map.getSource(SOURCE_ID) as maplibregl.GeoJSONSource | undefined;
		const existingEndpointsSource = this.map.getSource(ENDPOINTS_SOURCE_ID) as
			| maplibregl.GeoJSONSource
			| undefined;

		if (existingSource && existingEndpointsSource) {
			// Update existing sources
			existingSource.setData(polygonGeojson);
			existingEndpointsSource.setData(endpointsGeojson);
		} else {
			// Add new sources and layers
			this.addSourcesAndLayers(polygonGeojson, endpointsGeojson);
		}

		logger.debug('[RUNWAYS] Displayed {count} runways', { count: runways.length });
	}

	/**
	 * Create GeoJSON for runway polygons and endpoints
	 */
	private createGeoJson(runways: Runway[]): {
		polygonGeojson: GeoJSON.FeatureCollection<GeoJSON.Polygon>;
		endpointsGeojson: GeoJSON.FeatureCollection<GeoJSON.Point>;
	} {
		const polygonFeatures: GeoJSON.Feature<GeoJSON.Polygon>[] = [];
		const endpointFeatures: GeoJSON.Feature<GeoJSON.Point>[] = [];

		for (const runway of runways) {
			// Create polygon if we have valid polyline (4 corner points)
			if (runway.polyline && runway.polyline.length === 4) {
				// Convert [lat, lon] to [lon, lat] for GeoJSON and close the polygon
				const coordinates = [
					...runway.polyline.map((coord) => [coord[1], coord[0]] as [number, number]),
					[runway.polyline[0][1], runway.polyline[0][0]] as [number, number] // Close the polygon
				];

				polygonFeatures.push({
					type: 'Feature',
					geometry: {
						type: 'Polygon',
						coordinates: [coordinates]
					},
					properties: {
						id: `${runway.airportIdent}-${runway.low.ident || ''}-${runway.high.ident || ''}`
					}
				});
			}

			// Add endpoint markers
			if (runway.low.latitudeDeg !== null && runway.low.longitudeDeg !== null) {
				endpointFeatures.push({
					type: 'Feature',
					geometry: {
						type: 'Point',
						coordinates: [runway.low.longitudeDeg, runway.low.latitudeDeg]
					},
					properties: {
						ident: runway.low.ident || ''
					}
				});
			}

			if (runway.high.latitudeDeg !== null && runway.high.longitudeDeg !== null) {
				endpointFeatures.push({
					type: 'Feature',
					geometry: {
						type: 'Point',
						coordinates: [runway.high.longitudeDeg, runway.high.latitudeDeg]
					},
					properties: {
						ident: runway.high.ident || ''
					}
				});
			}
		}

		return {
			polygonGeojson: { type: 'FeatureCollection', features: polygonFeatures },
			endpointsGeojson: { type: 'FeatureCollection', features: endpointFeatures }
		};
	}

	/**
	 * Add sources and layers to the map
	 */
	private addSourcesAndLayers(
		polygonGeojson: GeoJSON.FeatureCollection,
		endpointsGeojson: GeoJSON.FeatureCollection
	): void {
		if (!this.map) return;

		// Add runway polygon source
		this.map.addSource(SOURCE_ID, {
			type: 'geojson',
			data: polygonGeojson
		});

		// Add runway endpoints source
		this.map.addSource(ENDPOINTS_SOURCE_ID, {
			type: 'geojson',
			data: endpointsGeojson
		});

		// Add fill layer for runway polygons
		this.map.addLayer({
			id: FILL_LAYER_ID,
			type: 'fill',
			source: SOURCE_ID,
			paint: {
				'fill-color': '#3b82f6', // Blue (Tailwind blue-500)
				'fill-opacity': 0.4
			}
		});

		// Add line layer for runway outlines
		this.map.addLayer({
			id: LINE_LAYER_ID,
			type: 'line',
			source: SOURCE_ID,
			paint: {
				'line-color': '#fb923c', // Orange (Tailwind orange-400)
				'line-width': 2
			}
		});

		// Add circle layer for endpoints
		this.map.addLayer({
			id: ENDPOINTS_LAYER_ID,
			type: 'circle',
			source: ENDPOINTS_SOURCE_ID,
			paint: {
				'circle-radius': 4,
				'circle-color': '#F59E0B', // Amber
				'circle-stroke-width': 2,
				'circle-stroke-color': '#F59E0B'
			}
		});

		this.layersAdded = true;
	}

	/**
	 * Remove layers and sources from the map
	 */
	private removeLayers(): void {
		if (!this.map || !this.layersAdded) return;

		// Remove layers first
		if (this.map.getLayer(ENDPOINTS_LAYER_ID)) {
			this.map.removeLayer(ENDPOINTS_LAYER_ID);
		}
		if (this.map.getLayer(LINE_LAYER_ID)) {
			this.map.removeLayer(LINE_LAYER_ID);
		}
		if (this.map.getLayer(FILL_LAYER_ID)) {
			this.map.removeLayer(FILL_LAYER_ID);
		}

		// Then remove sources
		if (this.map.getSource(ENDPOINTS_SOURCE_ID)) {
			this.map.removeSource(ENDPOINTS_SOURCE_ID);
		}
		if (this.map.getSource(SOURCE_ID)) {
			this.map.removeSource(SOURCE_ID);
		}

		this.layersAdded = false;
	}
}
