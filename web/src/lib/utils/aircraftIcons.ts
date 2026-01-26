/**
 * Aircraft icon utilities for MapLibre maps
 *
 * Provides PNG icons for different aircraft categories, colored by altitude.
 * Icons are pre-rendered sprites stored in /static/icons/aircraft/
 */

import type { AircraftCategory } from '$lib/types';

/**
 * Icon shape names matching available PNG sprite files
 */
export type IconShape =
	| 'balloon'
	| 'drone'
	| 'fixedwing'
	| 'helicopter'
	| 'jet-2engine'
	| 'jet-4engine'
	| 'single-prop';

/**
 * All available icon shapes
 */
const ALL_SHAPES: IconShape[] = [
	'balloon',
	'drone',
	'fixedwing',
	'helicopter',
	'jet-2engine',
	'jet-4engine',
	'single-prop'
];

/**
 * Map AircraftCategory to icon shape
 */
export function getIconShapeForCategory(category: AircraftCategory | null | undefined): IconShape {
	if (!category) return 'fixedwing';

	switch (category) {
		// Gliders use single-prop silhouette (visible wings)
		case 'Glider':
		case 'HangGlider':
			return 'single-prop';

		// Paragliders/parachutes use balloon (round shape)
		case 'Paraglider':
		case 'PoweredParachute':
		case 'SkydiverParachute':
			return 'balloon';

		// Rotorcraft
		case 'Helicopter':
		case 'Rotorcraft':
		case 'Gyroplane':
			return 'helicopter';

		// Tiltrotor uses twin-engine turboprop
		case 'Tiltrotor':
		case 'Vtol':
			return 'fixedwing';

		// Balloon/airship
		case 'Balloon':
		case 'Airship':
			return 'balloon';

		// Drones
		case 'Drone':
			return 'drone';

		// Fixed-wing aircraft - use appropriate size
		case 'Landplane':
		case 'Seaplane':
		case 'Amphibian':
		case 'TowTug':
		case 'Electric':
			return 'fixedwing';

		// Static obstacles use single-prop as marker
		case 'StaticObstacle':
			return 'single-prop';

		case 'Unknown':
		default:
			return 'fixedwing';
	}
}

/**
 * Get the URL for an aircraft icon PNG
 *
 * @param shape - The icon shape
 * @param colorName - Color name (red, orange, amber, yellow, lime, green, cyan, blue, gray)
 * @returns URL path to the PNG file
 */
export function getAircraftIconUrl(shape: IconShape, colorName: AltitudeColorName): string {
	return `/icons/aircraft/${shape}-${colorName}.png`;
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
	// Map altitude ranges to colors (low=red to high=blue)
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
 * @returns Icon name string like "aircraft-helicopter-red"
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
	return ALL_SHAPES;
}
