/**
 * AirportLayerManager - Manages airport symbol layers on a MapLibre map
 *
 * Handles fetching, displaying, and clearing airport markers with automatic
 * visibility based on zoom level and viewport area. Uses MapLibre's native
 * GeoJSON source and symbol layers for efficient rendering.
 */

import type maplibregl from 'maplibre-gl';
import { serverCall } from '$lib/api/server';
import { getLogger } from '$lib/logging';
import type { Airport, DataListResponse, Runway } from '$lib/types';

const logger = getLogger(['soar', 'maplibre', 'AirportLayerManager']);

/** Maximum viewport area (in square miles) at which airports are displayed */
const MAX_VIEWPORT_AREA_FOR_AIRPORTS = 10000;

/** Debounce delay for airport updates (ms) */
const DEBOUNCE_MS = 100;

/** Source and layer IDs */
const SOURCE_ID = 'airports-source';
const SYMBOL_LAYER_ID = 'airports-symbols';

export interface AirportLayerManagerOptions {
	/** Callback when an airport marker is clicked */
	onAirportClick?: (airport: Airport) => void;
	/** Callback when airports are loaded (for runway display) */
	onAirportsLoaded?: (airports: Airport[]) => void;
}

export class AirportLayerManager {
	private map: maplibregl.Map | null = null;
	private airports: Airport[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: AirportLayerManagerOptions;
	private layersAdded: boolean = false;

	constructor(options: AirportLayerManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: maplibregl.Map): void {
		this.map = map;

		// Set up click handler if callback provided
		if (this.options.onAirportClick) {
			this.setupClickHandler();
		}
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
	 * Check viewport and update airport visibility/layers
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether airport markers are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates to prevent excessive API calls
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
		}, DEBOUNCE_MS);
	}

	/**
	 * Force clear all layers and reset state
	 */
	clear(): void {
		this.removeLayers();
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
		this.removeLayers();
		this.airports = [];
		this.map = null;
	}

	/**
	 * Fetch airports in the current viewport and display them
	 */
	private async fetchAndDisplay(): Promise<void> {
		if (!this.map) return;

		const bounds = this.map.getBounds();
		const north = bounds.getNorth();
		const south = bounds.getSouth();
		const east = bounds.getEast();
		const west = bounds.getWest();

		// Validate bounding box
		if (north <= south) {
			logger.warn('Invalid bounding box: north must be greater than south');
			return;
		}

		try {
			const params = new URLSearchParams({
				north: north.toString(),
				west: west.toString(),
				south: south.toString(),
				east: east.toString(),
				limit: '100'
			});

			const response = await serverCall<DataListResponse<Airport>>(`/airports?${params}`);
			this.airports = response.data || [];

			this.updateLayers();

			// Notify callback that airports were loaded
			if (this.options.onAirportsLoaded) {
				this.options.onAirportsLoaded(this.airports);
			}
		} catch (error) {
			logger.error('Error fetching airports: {error}', { error });
		}
	}

	/**
	 * Update or create the map layers with current airport data
	 */
	private updateLayers(): void {
		if (!this.map) return;

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

		logger.debug('[AIRPORTS] Displayed {count} airports', { count: this.airports.length });
	}

	/**
	 * Create GeoJSON FeatureCollection from airports
	 */
	private createGeoJson(): GeoJSON.FeatureCollection<GeoJSON.Point> {
		const features: GeoJSON.Feature<GeoJSON.Point>[] = [];

		for (const airport of this.airports) {
			if (!airport.latitudeDeg || !airport.longitudeDeg) continue;

			const lat = airport.latitudeDeg;
			const lng = airport.longitudeDeg;

			// Validate coordinates
			if (lat < -90 || lat > 90 || lng < -180 || lng > 180) {
				continue;
			}

			features.push({
				type: 'Feature',
				geometry: {
					type: 'Point',
					coordinates: [lng, lat]
				},
				properties: {
					id: airport.id,
					ident: airport.ident
				}
			});
		}

		return {
			type: 'FeatureCollection',
			features
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

		// Add symbol layer
		this.map.addLayer({
			id: SYMBOL_LAYER_ID,
			type: 'symbol',
			source: SOURCE_ID,
			layout: {
				'text-field': ['get', 'ident'],
				'text-font': ['Open Sans Semibold', 'Arial Unicode MS Bold'],
				'text-size': 12,
				'text-offset': [0, 1.2],
				'text-anchor': 'top',
				'icon-allow-overlap': false,
				'text-allow-overlap': false
			},
			paint: {
				'text-color': '#ffffff',
				'text-halo-color': 'rgba(0, 0, 0, 0.85)',
				'text-halo-width': 2
			}
		});

		// Add a circle layer underneath for the airport marker
		this.map.addLayer(
			{
				id: `${SYMBOL_LAYER_ID}-circle`,
				type: 'circle',
				source: SOURCE_ID,
				paint: {
					'circle-radius': 6,
					'circle-color': '#3b82f6',
					'circle-stroke-width': 2,
					'circle-stroke-color': '#ffffff'
				}
			},
			SYMBOL_LAYER_ID
		); // Insert before symbol layer

		this.layersAdded = true;
	}

	/**
	 * Remove layers and source from the map
	 */
	private removeLayers(): void {
		if (!this.map || !this.layersAdded) return;

		// Remove layers first
		if (this.map.getLayer(SYMBOL_LAYER_ID)) {
			this.map.removeLayer(SYMBOL_LAYER_ID);
		}
		if (this.map.getLayer(`${SYMBOL_LAYER_ID}-circle`)) {
			this.map.removeLayer(`${SYMBOL_LAYER_ID}-circle`);
		}

		// Then remove source
		if (this.map.getSource(SOURCE_ID)) {
			this.map.removeSource(SOURCE_ID);
		}

		this.layersAdded = false;
	}

	/**
	 * Set up click handler for airport symbols
	 */
	private setupClickHandler(): void {
		if (!this.map || !this.options.onAirportClick) return;

		const clickHandler = this.options.onAirportClick;

		// Listen on both circle and symbol layers
		const handleClick = (e: maplibregl.MapLayerMouseEvent) => {
			if (!e.features || e.features.length === 0) return;

			const feature = e.features[0];
			const airportId = feature.properties?.id;

			if (airportId) {
				const airport = this.airports.find((a) => a.id === airportId);
				if (airport) {
					clickHandler(airport);
				}
			}
		};

		this.map.on('click', SYMBOL_LAYER_ID, handleClick);
		this.map.on('click', `${SYMBOL_LAYER_ID}-circle`, handleClick);

		// Change cursor on hover
		const setCursor = () => {
			if (this.map) this.map.getCanvas().style.cursor = 'pointer';
		};
		const resetCursor = () => {
			if (this.map) this.map.getCanvas().style.cursor = '';
		};

		this.map.on('mouseenter', SYMBOL_LAYER_ID, setCursor);
		this.map.on('mouseenter', `${SYMBOL_LAYER_ID}-circle`, setCursor);
		this.map.on('mouseleave', SYMBOL_LAYER_ID, resetCursor);
		this.map.on('mouseleave', `${SYMBOL_LAYER_ID}-circle`, resetCursor);
	}
}
