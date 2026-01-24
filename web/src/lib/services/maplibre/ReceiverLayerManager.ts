/**
 * ReceiverLayerManager - Manages receiver (radio station) symbol layers on a MapLibre map
 *
 * Handles fetching, displaying, and clearing receiver markers with automatic
 * visibility based on zoom level and viewport area. Uses MapLibre's native
 * GeoJSON source and symbol layers for efficient rendering.
 */

import type maplibregl from 'maplibre-gl';
import { serverCall } from '$lib/api/server';
import { getLogger } from '$lib/logging';
import type { Receiver, DataListResponse } from '$lib/types';

const logger = getLogger(['soar', 'maplibre', 'ReceiverLayerManager']);

/** Maximum viewport area (in square miles) at which receivers are displayed */
const MAX_VIEWPORT_AREA_FOR_RECEIVERS = 10000;

/** Debounce delay for receiver updates (ms) */
const DEBOUNCE_MS = 100;

/** Source and layer IDs */
const SOURCE_ID = 'receivers-source';
const SYMBOL_LAYER_ID = 'receivers-symbols';
const CIRCLE_LAYER_ID = 'receivers-circles';

export interface ReceiverLayerManagerOptions {
	/** Callback when a receiver marker is clicked */
	onReceiverClick?: (receiver: Receiver) => void;
}

export class ReceiverLayerManager {
	private map: maplibregl.Map | null = null;
	private receivers: Receiver[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: ReceiverLayerManagerOptions;
	private layersAdded: boolean = false;

	constructor(options: ReceiverLayerManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: maplibregl.Map): void {
		this.map = map;

		// Set up click handler if callback provided
		if (this.options.onReceiverClick) {
			this.setupClickHandler();
		}
	}

	/**
	 * Get the currently loaded receivers
	 */
	getReceivers(): Receiver[] {
		return this.receivers;
	}

	/**
	 * Check viewport and update receiver visibility/layers
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether receiver markers are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates to prevent excessive API calls
		this.debounceTimer = setTimeout(() => {
			const shouldShow = viewportArea < MAX_VIEWPORT_AREA_FOR_RECEIVERS && settingEnabled;

			if (shouldShow !== this.shouldShow) {
				this.shouldShow = shouldShow;

				if (this.shouldShow) {
					this.fetchAndDisplay();
				} else {
					this.clear();
				}
			} else if (this.shouldShow) {
				// Still showing receivers, update them for the new viewport
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
		this.receivers = [];
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
		this.receivers = [];
		this.map = null;
	}

	/**
	 * Fetch receivers in the current viewport and display them
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
				south: south.toString(),
				east: east.toString(),
				west: west.toString()
			});

			const response = await serverCall<DataListResponse<Receiver>>(`/receivers?${params}`);
			this.receivers = response.data || [];

			this.updateLayers();
		} catch (error) {
			logger.error('Error fetching receivers: {error}', { error });
		}
	}

	/**
	 * Update or create the map layers with current receiver data
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

		logger.debug('[RECEIVERS] Displayed {count} receivers', { count: this.receivers.length });
	}

	/**
	 * Create GeoJSON FeatureCollection from receivers
	 */
	private createGeoJson(): GeoJSON.FeatureCollection<GeoJSON.Point> {
		const features: GeoJSON.Feature<GeoJSON.Point>[] = [];

		for (const receiver of this.receivers) {
			if (!receiver.latitude || !receiver.longitude) continue;

			// Validate coordinates
			if (
				isNaN(receiver.latitude) ||
				isNaN(receiver.longitude) ||
				receiver.latitude < -90 ||
				receiver.latitude > 90 ||
				receiver.longitude < -180 ||
				receiver.longitude > 180
			) {
				continue;
			}

			features.push({
				type: 'Feature',
				geometry: {
					type: 'Point',
					coordinates: [receiver.longitude, receiver.latitude]
				},
				properties: {
					id: receiver.id,
					callsign: receiver.callsign,
					description: receiver.description || '',
					// Store full receiver data for click handler
					_receiverData: JSON.stringify(receiver)
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

		// Add circle layer for receiver markers
		this.map.addLayer({
			id: CIRCLE_LAYER_ID,
			type: 'circle',
			source: SOURCE_ID,
			paint: {
				'circle-radius': 6,
				'circle-color': '#6b7280', // Gray color for receivers
				'circle-stroke-width': 2,
				'circle-stroke-color': '#ffffff'
			}
		});

		// Add symbol layer for labels
		this.map.addLayer({
			id: SYMBOL_LAYER_ID,
			type: 'symbol',
			source: SOURCE_ID,
			minzoom: 9, // Only show labels when zoomed in
			layout: {
				'text-field': ['get', 'callsign'],
				'text-font': ['Open Sans Regular', 'Arial Unicode MS Regular'],
				'text-size': 10,
				'text-offset': [0, 1.2],
				'text-anchor': 'top',
				'text-allow-overlap': false
			},
			paint: {
				'text-color': '#4b5563',
				'text-halo-color': '#ffffff',
				'text-halo-width': 1
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
		if (this.map.getLayer(SYMBOL_LAYER_ID)) {
			this.map.removeLayer(SYMBOL_LAYER_ID);
		}
		if (this.map.getLayer(CIRCLE_LAYER_ID)) {
			this.map.removeLayer(CIRCLE_LAYER_ID);
		}

		// Then remove source
		if (this.map.getSource(SOURCE_ID)) {
			this.map.removeSource(SOURCE_ID);
		}

		this.layersAdded = false;
	}

	/**
	 * Set up click handler for receiver symbols
	 */
	private setupClickHandler(): void {
		if (!this.map || !this.options.onReceiverClick) return;

		const clickHandler = this.options.onReceiverClick;

		const handleClick = (e: maplibregl.MapLayerMouseEvent) => {
			if (!e.features || e.features.length === 0) return;

			const feature = e.features[0];
			const receiverData = feature.properties?._receiverData;

			if (receiverData) {
				try {
					const receiver = JSON.parse(receiverData) as Receiver;
					clickHandler(receiver);
				} catch (err) {
					logger.error('Failed to parse receiver data: {error}', { error: err });
				}
			}
		};

		this.map.on('click', CIRCLE_LAYER_ID, handleClick);

		// Change cursor on hover
		this.map.on('mouseenter', CIRCLE_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = 'pointer';
		});

		this.map.on('mouseleave', CIRCLE_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = '';
		});
	}
}
