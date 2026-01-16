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

export type MapType = 'satellite' | 'roadmap';

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
 * Load map state from URL params or localStorage
 * @param urlParams - URL search params to check for lat/lng/zoom
 * @returns MapState with center and zoom
 */
export function loadMapState(urlParams?: URLSearchParams): MapState {
	// First check URL parameters
	if (browser && urlParams) {
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
				return { center: { lat: parsedLat, lng: parsedLng }, zoom: parsedZoom };
			}
		}
	}

	// Fall back to localStorage
	if (!browser) {
		return { center: CONUS_CENTER, zoom: 4 };
	}

	try {
		const saved = localStorage.getItem(MAP_STATE_KEY);
		if (saved) {
			const state: MapState = JSON.parse(saved);
			logger.debug('[MAP] Loaded saved map state: {state}', { state });
			return state;
		}
	} catch (e) {
		logger.warn('[MAP] Failed to load map state from localStorage: {error}', { error: e });
	}

	logger.debug('[MAP] Using default CONUS center');
	return { center: CONUS_CENTER, zoom: 4 };
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
		if (saved === 'satellite' || saved === 'roadmap') {
			logger.debug('[MAP TYPE] Loaded saved map type: {saved}', { saved });
			return saved;
		}
	} catch (e) {
		logger.warn('[MAP TYPE] Failed to load map type from localStorage: {error}', { error: e });
	}

	logger.debug('[MAP TYPE] Using default: satellite');
	return 'satellite';
}
