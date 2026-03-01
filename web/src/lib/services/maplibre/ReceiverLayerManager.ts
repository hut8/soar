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
const ICON_LAYER_ID = 'receivers-icons';
const ICON_IMAGE_ID = 'receiver-icon';

/** Icon size in pixels for the receiver icon image */
const ICON_SIZE = 32;

/**
 * Create a data URL for the receiver radio/antenna SVG icon.
 * Uses the same radio wave pattern as the Google Maps ReceiverMarkerManager.
 */
function createReceiverIconDataUrl(size: number): string {
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/><path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/><path d="M19.1 4.9C23 8.8 23 15.1 19.1 19"/></svg>`;
	return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

export interface ReceiverLayerManagerOptions {
	/** Callback when a receiver marker is clicked */
	onReceiverClick?: (receiver: Receiver) => void;
}

export class ReceiverLayerManager {
	private map: maplibregl.Map | null = null;
	private receivers: Receiver[] = [];
	private receiverById: Map<string, Receiver> = new Map();
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private options: ReceiverLayerManagerOptions;
	private layersAdded: boolean = false;
	private iconLoaded: boolean = false;

	constructor(options: ReceiverLayerManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: maplibregl.Map): void {
		this.map = map;

		// Load the receiver icon image
		this.loadIcon();

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
	 * Look up a receiver by ID (O(1) via internal Map)
	 */
	getReceiverById(id: string): Receiver | undefined {
		return this.receiverById.get(id);
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
		this.receiverById = new Map();
		this.map = null;
	}

	/**
	 * Load the receiver icon image into MapLibre
	 */
	private loadIcon(): void {
		if (!this.map || this.iconLoaded) return;

		const iconUrl = createReceiverIconDataUrl(ICON_SIZE);
		const img = new Image();
		img.onload = () => {
			if (this.map && !this.map.hasImage(ICON_IMAGE_ID)) {
				this.map.addImage(ICON_IMAGE_ID, img, { sdf: true });
			}
			this.iconLoaded = true;
		};
		img.onerror = () => {
			logger.warn('Failed to load receiver icon');
		};
		img.src = iconUrl;
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

			// Rebuild id→receiver lookup map
			this.receiverById = new Map(this.receivers.map((r) => [r.id, r]));

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
					callsign: receiver.callsign
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

		// Add icon layer for receiver markers (replaces old circle layer)
		this.map.addLayer({
			id: ICON_LAYER_ID,
			type: 'symbol',
			source: SOURCE_ID,
			layout: {
				'icon-image': ICON_IMAGE_ID,
				'icon-size': ['interpolate', ['linear'], ['zoom'], 6, 0.5, 10, 0.7, 14, 0.9],
				'icon-allow-overlap': true
			},
			paint: {
				'icon-color': '#6b7280',
				'icon-halo-color': 'rgba(255, 255, 255, 0.9)',
				'icon-halo-width': 1
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
				'text-offset': [0, 1.5],
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
		if (this.map.getLayer(ICON_LAYER_ID)) {
			this.map.removeLayer(ICON_LAYER_ID);
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

			// Check if an aircraft was clicked at the same point - aircraft have priority
			const aircraftFeatures = this.map!.queryRenderedFeatures(e.point, {
				layers: ['aircraft-markers']
			});
			if (aircraftFeatures.length > 0) {
				return; // Aircraft clicked, let aircraft handler deal with it
			}

			const feature = e.features[0];
			const receiverId = feature.properties?.id;

			if (receiverId) {
				const receiver = this.receivers.find((r) => r.id === receiverId);
				if (receiver) {
					clickHandler(receiver);
				}
			}
		};

		// Register click handler on both the icon layer and symbol (label) layer
		this.map.on('click', ICON_LAYER_ID, handleClick);
		this.map.on('click', SYMBOL_LAYER_ID, handleClick);

		// Change cursor on hover for icon layer
		this.map.on('mouseenter', ICON_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = 'pointer';
		});

		this.map.on('mouseleave', ICON_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = '';
		});

		// Change cursor on hover for symbol (label) layer
		this.map.on('mouseenter', SYMBOL_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = 'pointer';
		});

		this.map.on('mouseleave', SYMBOL_LAYER_ID, () => {
			if (this.map) this.map.getCanvas().style.cursor = '';
		});
	}
}
