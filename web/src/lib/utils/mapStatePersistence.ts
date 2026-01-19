/**
 * Map state persistence utilities for localStorage
 *
 * Handles saving and loading map state (center, zoom), area tracker state,
 * and map type preferences.
 */

import { browser } from '$app/environment';
import { getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'MapStatePersistence']);

// Storage keys
const MAP_STATE_KEY = 'operations-map-state';
const AREA_TRACKER_KEY = 'operations-area-tracker';
const MAP_TYPE_KEY = 'operations-map-type';

// Center of continental US (default fallback)
export const CONUS_CENTER = {
	lat: 39.8283,
	lng: -98.5795
};

export interface MapState {
	center: { lat: number; lng: number };
	zoom: number;
}

export interface MapBounds {
	north: number;
	south: number;
	west: number;
	east: number;
}

export interface LoadedMapState {
	state: MapState;
	bounds?: MapBounds;
}

export type MapType = 'satellite' | 'roadmap' | 'terrain' | 'hybrid';

/**
 * Save map state (center and zoom) to localStorage
 */
export function saveMapState(center: { lat: number; lng: number }, zoom: number): void {
	if (!browser) return;

	const state: MapState = { center, zoom };

	try {
		localStorage.setItem(MAP_STATE_KEY, JSON.stringify(state));
		logger.debug('[MAP] Saved map state: {state}', { state });
	} catch (e) {
		logger.warn('[MAP] Failed to save map state to localStorage: {error}', { error: e });
	}
}

/**
 * Update URL with current map bounds
 * @param map - Google Maps instance to get bounds from
 */
export function updateUrlWithBounds(map: google.maps.Map): void {
	if (!browser) return;

	const bounds = map.getBounds();
	if (!bounds) return;

	const ne = bounds.getNorthEast();
	const sw = bounds.getSouthWest();

	const url = new URL(window.location.href);
	url.searchParams.set('north', ne.lat().toFixed(6));
	url.searchParams.set('south', sw.lat().toFixed(6));
	url.searchParams.set('west', sw.lng().toFixed(6));
	url.searchParams.set('east', ne.lng().toFixed(6));

	// Remove old lat/lng/zoom params if present
	url.searchParams.delete('lat');
	url.searchParams.delete('lng');
	url.searchParams.delete('zoom');

	history.replaceState(null, '', url.toString());
	logger.debug('[MAP] Updated URL with bounds: {bounds}', {
		bounds: { north: ne.lat(), south: sw.lat(), west: sw.lng(), east: ne.lng() }
	});
}

/**
 * Load map state from URL params or localStorage
 * @param urlParams - URL search params to check for bounds or lat/lng/zoom
 * @returns LoadedMapState with center, zoom, and optional bounds
 */
export function loadMapState(urlParams?: URLSearchParams): LoadedMapState {
	// First check URL parameters for bounds (north, south, west, east)
	if (browser && urlParams) {
		const north = urlParams.get('north');
		const south = urlParams.get('south');
		const west = urlParams.get('west');
		const east = urlParams.get('east');

		if (north && south && west && east) {
			const parsedNorth = parseFloat(north);
			const parsedSouth = parseFloat(south);
			const parsedWest = parseFloat(west);
			const parsedEast = parseFloat(east);

			if (!isNaN(parsedNorth) && !isNaN(parsedSouth) && !isNaN(parsedWest) && !isNaN(parsedEast)) {
				logger.debug('[MAP] Using URL bounds parameters: {bounds}', {
					bounds: {
						north: parsedNorth,
						south: parsedSouth,
						west: parsedWest,
						east: parsedEast
					}
				});
				// Return center of bounds with bounds for fitBounds
				const centerLat = (parsedNorth + parsedSouth) / 2;
				const centerLng = (parsedWest + parsedEast) / 2;
				return {
					state: { center: { lat: centerLat, lng: centerLng }, zoom: 10 },
					bounds: {
						north: parsedNorth,
						south: parsedSouth,
						west: parsedWest,
						east: parsedEast
					}
				};
			}
		}

		// Check for legacy lat/lng/zoom parameters
		const lat = urlParams.get('lat');
		const lng = urlParams.get('lng');
		const zoom = urlParams.get('zoom');

		if (lat && lng) {
			const parsedLat = parseFloat(lat);
			const parsedLng = parseFloat(lng);
			const parsedZoom = zoom ? parseInt(zoom, 10) : 13;

			if (!isNaN(parsedLat) && !isNaN(parsedLng) && !isNaN(parsedZoom)) {
				logger.debug('[MAP] Using URL parameters: {params}', {
					params: { lat: parsedLat, lng: parsedLng, zoom: parsedZoom }
				});
				return { state: { center: { lat: parsedLat, lng: parsedLng }, zoom: parsedZoom } };
			}
		}
	}

	// Fall back to localStorage
	if (!browser) {
		return { state: { center: CONUS_CENTER, zoom: 4 } };
	}

	try {
		const saved = localStorage.getItem(MAP_STATE_KEY);
		if (saved) {
			const state: MapState = JSON.parse(saved);
			logger.debug('[MAP] Loaded saved map state: {state}', { state });
			return { state };
		}
	} catch (e) {
		logger.warn('[MAP] Failed to load map state from localStorage: {error}', { error: e });
	}

	logger.debug('[MAP] Using default CONUS center');
	return { state: { center: CONUS_CENTER, zoom: 4 } };
}

/**
 * Save area tracker state to localStorage
 */
export function saveAreaTrackerState(active: boolean): void {
	if (!browser) return;

	try {
		localStorage.setItem(AREA_TRACKER_KEY, JSON.stringify(active));
		logger.debug('[AREA TRACKER] Saved state: {state}', { state: active });
	} catch (e) {
		logger.warn('[AREA TRACKER] Failed to save state to localStorage: {error}', { error: e });
	}
}

/**
 * Load area tracker state from localStorage
 * @param limitEnabled - Whether the area tracker limit feature is enabled
 * @returns boolean indicating if area tracker should be active
 */
export function loadAreaTrackerState(limitEnabled: boolean): boolean {
	if (!browser) return true;

	// When limit is disabled, area tracker is always on
	if (!limitEnabled) {
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

/**
 * Save map type to localStorage
 */
export function saveMapType(mapType: MapType): void {
	if (!browser) return;

	try {
		localStorage.setItem(MAP_TYPE_KEY, mapType);
		logger.debug('[MAP TYPE] Saved map type: {mapType}', { mapType });
	} catch (e) {
		logger.warn('[MAP TYPE] Failed to save map type to localStorage: {error}', { error: e });
	}
}

/**
 * Load map type from localStorage
 */
export function loadMapType(): MapType {
	if (!browser) return 'satellite';

	try {
		const saved = localStorage.getItem(MAP_TYPE_KEY);
		if (saved === 'satellite' || saved === 'roadmap' || saved === 'terrain' || saved === 'hybrid') {
			logger.debug('[MAP TYPE] Loaded saved map type: {saved}', { saved });
			return saved;
		}
	} catch (e) {
		logger.warn('[MAP TYPE] Failed to load map type from localStorage: {error}', { error: e });
	}

	logger.debug('[MAP TYPE] Using default: satellite');
	return 'satellite';
}

/**
 * Map type labels for UI display
 */
export const MAP_TYPE_LABELS: Record<MapType, string> = {
	roadmap: 'Map',
	satellite: 'Satellite',
	terrain: 'Terrain',
	hybrid: 'Hybrid'
};
