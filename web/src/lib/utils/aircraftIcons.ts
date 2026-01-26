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
	| 'fixedWing'
	| 'obstacle'
	| 'unknown';

/**
 * SVG path data for each icon shape
 * All paths are designed for a 24x24 viewBox, pointing north (up)
 */
const ICON_PATHS: Record<IconShape, string> = {
	// Glider/sailplane - long slender wings, no engine
	glider:
		'M12 2L12 6M12 6L3 10L3 12L12 10L12 18L8 20L8 22L12 20L16 22L16 20L12 18L12 10L21 12L21 10L12 6Z',

	// Hang glider - delta/triangular wing
	hangGlider: 'M12 4L3 20L12 16L21 20L12 4Z M12 16L12 22',

	// Paraglider/parachute - curved canopy shape
	paraglider:
		'M4 8C4 5 8 3 12 3C16 3 20 5 20 8C20 10 18 12 12 12C6 12 4 10 4 8Z M12 12L12 20M8 10L6 18M16 10L18 18',

	// Helicopter - side profile silhouette (based on Material Design)
	helicopter:
		'M12 2c-.55 0-1 .45-1 1v1H9.5c-.28 0-.5.22-.5.5s.22.5.5.5H11v1H4c-1.1 0-2 .9-2 2v3c0 1.1.9 2 2 2h1l1.5 5h1l.5-2h8l.5 2h1l1.5-5h1c1.1 0 2-.9 2-2v-3c0-1.1-.9-2-2-2h-7V5h1.5c.28 0 .5-.22.5-.5s-.22-.5-.5-.5H13V3c0-.55-.45-1-1-1z',

	// Tiltrotor/VTOL - twin rotors with wing
	tiltrotor:
		'M6 6A3 3 0 1 0 6 12A3 3 0 1 0 6 6M18 6A3 3 0 1 0 18 12A3 3 0 1 0 18 6M6 9L18 9M12 9L12 16L9 18L9 20L12 18L15 20L15 18L12 16',

	// Balloon/airship - envelope shape
	balloon: 'M12 2C7 2 4 6 4 11C4 15 7 18 10 19L10 22L14 22L14 19C17 18 20 15 20 11C20 6 17 2 12 2Z',

	// Drone/quadcopter - four rotors
	drone:
		'M6 6A2 2 0 1 0 6 10A2 2 0 1 0 6 6M18 6A2 2 0 1 0 18 10A2 2 0 1 0 18 6M6 14A2 2 0 1 0 6 18A2 2 0 1 0 6 14M18 14A2 2 0 1 0 18 18A2 2 0 1 0 18 14M8 8L16 16M16 8L8 16M12 10L12 14',

	// Fixed wing aircraft - standard airplane (Material Design icon)
	fixedWing:
		'M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z',

	// Static obstacle - tower/mast marker
	obstacle: 'M12 2L12 18M8 18L16 18M10 6L14 6M9 10L15 10M8 14L16 14M6 22L18 22L18 20L6 20L6 22Z',

	// Unknown - simple diamond/dot
	unknown: 'M12 4L18 12L12 20L6 12L12 4Z'
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

	// Use stroke for line-based icons, fill for solid icons
	const isSolidIcon =
		shape === 'fixedWing' || shape === 'helicopter' || shape === 'balloon' || shape === 'unknown';

	const svg = isSolidIcon
		? `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="${color}">
			<path d="${path}"/>
		</svg>`
		: `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
			<path d="${path}"/>
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
