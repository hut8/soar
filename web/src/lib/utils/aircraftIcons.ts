/**
 * Aircraft icon generation for MapLibre maps
 *
 * Provides SVG icons for different aircraft categories, colored by altitude.
 */

import type { AircraftCategory } from '$lib/types';

/**
 * Icon shape names - each represents a distinct aircraft silhouette
 */
export type IconShape =
	| 'glider'
	| 'hangGlider'
	| 'paraglider'
	| 'helicopter'
	| 'tiltrotor'
	| 'balloon'
	| 'drone'
	| 'jet'
	| 'fixedWing'
	| 'obstacle'
	| 'unknown';

/**
 * SVG path data for each icon shape
 * All paths are designed for a 36x36 viewBox, pointing north (up)
 * Top-down planform views inspired by standard aviation symbology
 */
const ICON_PATHS: Record<IconShape, string> = {
	// Glider/sailplane - very long slender wings, narrow fuselage (high aspect ratio)
	glider:
		'M18 3L18 7L17 7L17 14L18 14L18 16L20 18L20 19L18 18L18 21L20 22L20 24L18 23L18 33L17 33L17 23L15 24L15 22L17 21L17 18L15 19L15 18L17 16L17 14L18 14L18 7L17 7L17 3L18 3Z M1 15L1 17L17 19L17 17L1 15Z M35 15L35 17L19 19L19 17L35 15Z',

	// Hang glider - delta/triangular wing with pilot
	hangGlider: 'M18 2L2 28L18 22L34 28L18 2Z M17 22L17 34L19 34L19 22L17 22Z',

	// Paraglider - curved rectangular canopy with lines to pilot
	paraglider:
		'M6 6Q6 2 18 2Q30 2 30 6Q30 10 18 12Q6 10 6 6Z M18 12L18 28 M10 9L8 24 M26 9L28 24 M8 24L18 28L28 24',

	// Helicopter - top-down with rotor disc and tail boom
	helicopter:
		'M18 2A14 14 0 0 1 18 30A14 14 0 0 1 18 2Z M16 16L16 34L14 34L13 33L13 31L14 30L14 28L20 28L20 16L16 16Z M13 31L9 31L9 33L13 33L13 31Z M22 30L22 34L24 34L24 30L22 30Z',

	// Tiltrotor/VTOL - V-22 Osprey style with rotors on wingtips
	tiltrotor:
		'M17 4L17 12L19 12L19 4L17 4Z M5 11A5 5 0 1 1 5 21A5 5 0 1 1 5 11Z M31 11A5 5 0 1 1 31 21A5 5 0 1 1 31 11Z M10 15L17 15L17 17L10 17L10 15Z M19 15L26 15L26 17L19 17L19 15Z M17 17L17 28L15 30L15 32L17 30L17 32L19 32L19 30L21 32L21 30L19 28L19 17L17 17Z',

	// Balloon - simple filled circle with basket
	balloon:
		'M18 2A12 12 0 0 1 18 26A12 12 0 0 1 18 2Z M15 26L15 30L21 30L21 26 M14 30L14 34L22 34L22 30L14 30Z',

	// Drone/quadcopter - X-frame with four rotors
	drone:
		'M6 6A4 4 0 1 1 6 14A4 4 0 1 1 6 6Z M30 6A4 4 0 1 1 30 14A4 4 0 1 1 30 6Z M6 22A4 4 0 1 1 6 30A4 4 0 1 1 6 22Z M30 22A4 4 0 1 1 30 30A4 4 0 1 1 30 22Z M10 10L15 15L15 21L10 26 M26 10L21 15L21 21L26 26 M15 15L21 15L21 21L15 21Z',

	// Jet airliner - twin engine, swept wings (like A320/737)
	jet: 'M17 1L17 10L19 10L19 1L17 1Z M17 10L5 18L5 21L17 18L17 28L12 31L12 34L17 31L17 35L19 35L19 31L24 34L24 31L19 28L19 18L31 21L31 18L19 10L17 10Z M5 18L5 24L8 24L9 18L5 18Z M27 18L31 18L31 24L28 24L27 18Z',

	// Single-engine prop plane - high wing, fixed gear style
	fixedWing:
		'M17 2L17 8L19 8L19 2L17 2Z M17 8L17 12L4 16L4 19L17 16L17 26L13 29L13 32L17 29L17 34L19 34L19 29L23 32L23 29L19 26L19 16L32 19L32 16L19 12L19 8L17 8Z',

	// Static obstacle - warning triangle with tower
	obstacle:
		'M18 2L32 30L4 30L18 2Z M17 10L17 20L19 20L19 10L17 10Z M17 23L17 27L19 27L19 23L17 23Z',

	// Unknown - diamond marker
	unknown: 'M18 4L30 18L18 32L6 18L18 4Z M18 10L24 18L18 26L12 18L18 10Z'
};

/**
 * Map AircraftCategory to icon shape
 */
export function getIconShapeForCategory(category: AircraftCategory | null | undefined): IconShape {
	if (!category) return 'fixedWing';

	switch (category) {
		case 'Glider':
			return 'glider';
		case 'HangGlider':
			return 'hangGlider';
		case 'Paraglider':
		case 'PoweredParachute':
		case 'SkydiverParachute':
			return 'paraglider';
		case 'Helicopter':
		case 'Rotorcraft':
		case 'Gyroplane':
			return 'helicopter';
		case 'Tiltrotor':
		case 'Vtol':
			return 'tiltrotor';
		case 'Balloon':
		case 'Airship':
			return 'balloon';
		case 'Drone':
			return 'drone';
		case 'StaticObstacle':
			return 'obstacle';
		case 'Landplane':
		case 'Seaplane':
		case 'Amphibian':
		case 'TowTug':
		case 'Electric':
			return 'fixedWing';
		case 'Unknown':
		default:
			return 'fixedWing';
	}
}

/**
 * Create SVG data URL for an aircraft icon
 *
 * @param shape - The icon shape to render
 * @param color - Fill color (hex or rgb)
 * @param size - SVG size in pixels (default 48)
 * @returns Data URL string for use as image src
 */
export function createAircraftIconDataUrl(
	shape: IconShape,
	color: string,
	size: number = 48
): string {
	const path = ICON_PATHS[shape];

	// All icons are now solid filled shapes for clean rendering at small sizes
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 36 36" fill="${color}">
		<path d="${path}" fill-rule="evenodd"/>
	</svg>`;

	return 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg);
}

/**
 * Altitude color bands with their hex colors
 */
export const ALTITUDE_COLORS = [
	{ name: 'red', color: '#ef4444' },
	{ name: 'orange', color: '#f97316' },
	{ name: 'amber', color: '#f59e0b' },
	{ name: 'yellow', color: '#eab308' },
	{ name: 'lime', color: '#84cc16' },
	{ name: 'green', color: '#22c55e' },
	{ name: 'cyan', color: '#06b6d4' },
	{ name: 'blue', color: '#3b82f6' },
	{ name: 'gray', color: '#6b7280' }
] as const;

export type AltitudeColorName = (typeof ALTITUDE_COLORS)[number]['name'];

/**
 * Get altitude color name based on altitude in feet
 */
export function getAltitudeColorName(altitude: number | null | undefined): AltitudeColorName {
	if (altitude === null || altitude === undefined) {
		return 'gray';
	}
	// Map altitude ranges to colors (500 ft red to 18000 ft blue)
	if (altitude < 2000) return 'red';
	if (altitude < 4000) return 'orange';
	if (altitude < 6000) return 'amber';
	if (altitude < 8000) return 'yellow';
	if (altitude < 10000) return 'lime';
	if (altitude < 12000) return 'green';
	if (altitude < 15000) return 'cyan';
	return 'blue';
}

/**
 * Get the MapLibre icon name for an aircraft based on category and altitude
 *
 * @param category - Aircraft category from API
 * @param altitude - Altitude in feet MSL
 * @returns Icon name string like "aircraft-glider-red"
 */
export function getAircraftIconName(
	category: AircraftCategory | null | undefined,
	altitude: number | null | undefined
): string {
	const shape = getIconShapeForCategory(category);
	const colorName = getAltitudeColorName(altitude);
	return `aircraft-${shape}-${colorName}`;
}

/**
 * Get all icon shapes
 */
export function getAllIconShapes(): IconShape[] {
	return Object.keys(ICON_PATHS) as IconShape[];
}
