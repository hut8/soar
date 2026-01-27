/**
 * Aircraft icon utilities for MapLibre maps
 *
 * SVG paths from tar1090 project (https://github.com/wiedehopf/tar1090)
 * Icons use SDF (Signed Distance Field) rendering for runtime coloring
 * with fluid altitude-based color gradients.
 */

import type { AircraftCategory } from '$lib/types';
import type { ExpressionSpecification } from 'maplibre-gl';

/**
 * Icon shape names
 */
export type IconShape =
	| 'helicopter'
	| 'glider'
	| 'balloon'
	| 'cessna'
	| 'jet'
	| 'airliner'
	| 'unknown';

/**
 * SVG icon definitions from tar1090
 * Each shape has a viewBox and path data
 */
const ICON_SHAPES: Record<IconShape, { viewBox: string; path: string }> = {
	helicopter: {
		viewBox: '-13 -13 90 90',
		path: 'm 24.698,60.712 c 0,0 -0.450,2.134 -0.861,2.142 -0.561,0.011 -0.480,-3.836 -0.593,-5.761 -0.064,-1.098 1.381,-1.192 1.481,-0.042 l 5.464,0.007 -0.068,-9.482 -0.104,-1.108 c -2.410,-2.131 -3.028,-3.449 -3.152,-7.083 l -12.460,13.179 c -0.773,0.813 -2.977,0.599 -3.483,-0.428 L 26.920,35.416 26.866,29.159 11.471,14.513 c -0.813,-0.773 -0.599,-2.977 0.428,-3.483 l 14.971,14.428 0.150,-5.614 c -0.042,-1.324 1.075,-4.784 3.391,-5.633 0.686,-0.251 2.131,-0.293 3.033,0.008 2.349,0.783 3.433,4.309 3.391,5.633 l 0.073,4.400 12.573,-12.763 c 0.779,-0.807 2.977,-0.599 3.483,0.428 L 37.054,28.325 37.027,35.027 52.411,49.365 c 0.813,0.773 0.599,2.977 -0.428,3.483 L 36.992,38.359 c -0.124,3.634 -0.742,5.987 -3.152,8.118 l -0.104,1.108 -0.068,9.482 5.321,-0.068 c 0.101,-1.150 1.546,-1.057 1.481,0.042 -0.113,1.925 -0.032,5.772 -0.593,5.761 -0.412,-0.008 -0.861,-2.142 -0.861,-2.142 l -5.387,-0.011 0.085,9.377 -1.094,2.059 -1.386,-0.018 -1.093,-2.049 0.085,-9.377 z'
	},
	glider: {
		viewBox: '-5.8 -10 76 76',
		path: 'm 32.000,45.932 -0.215,0.314 c -0.118,0.173 -0.196,0.239 -0.378,0.401 -0.226,0.145 -0.850,-0.045 -1.196,-0.137 -0.658,-0.204 -1.909,-0.478 -2.984,-0.718 -0.065,-0.021 -0.186,-0.136 -0.406,-0.344 -0.342,-0.323 -0.409,-0.463 -0.459,-0.961 -0.074,-0.730 0.183,-1.127 0.795,-1.228 0.218,-0.036 0.732,-0.130 1.143,-0.210 0.411,-0.080 1.132,-0.201 1.602,-0.271 1.252,-0.185 1.635,-0.299 1.701,-0.507 0.059,-0.186 -0.006,-2.549 -0.101,-3.654 -0.110,-2.092 -0.181,-3.601 -0.281,-5.738 0.039,-0.214 -0.274,-0.732 -0.553,-0.915 l -5.180,-0.560 c -0.611,-0.069 -3.989,-0.350 -5.732,-0.476 -1.476,-0.108 -2.940,-0.246 -4.432,-0.362 l -3.097,-0.439 C 7.935,29.593 4.497,29.014 2.499e-5,28.410 l 0.019,-2.401 5.562,-0.286 c 2.699,-0.023 6.207,-0.092 9.264,-0.183 0.646,-0.019 4.548,-0.040 8.671,-0.047 l 7.496,-0.012 -0.017,-2.376 c -0.007,-1.423 -0.104,-3.049 0.253,-4.827 0.028,-0.088 0.121,-0.396 0.344,-0.722 0.071,-0.090 0.213,-0.175 0.408,-0.255 0.195,0.080 0.337,0.165 0.408,0.255 0.223,0.325 0.316,0.633 0.343,0.722 0.357,1.778 0.261,3.405 0.253,4.827 l -0.016,2.376 7.496,0.012 c 4.123,0.007 8.025,0.028 8.671,0.047 3.057,0.091 6.564,0.160 9.264,0.183 l 5.562,0.286 0.019,2.401 c -4.497,0.605 -7.935,1.183 -12.228,1.717 l -3.097,0.439 c -1.492,0.116 -2.956,0.254 -4.432,0.362 -1.743,0.127 -5.121,0.408 -5.732,0.476 l -5.180,0.560 c -0.278,0.182 -0.592,0.701 -0.553,0.915 -0.100,2.136 -0.171,3.646 -0.281,5.738 -0.095,1.105 -0.160,3.468 -0.101,3.654 0.066,0.208 0.449,0.322 1.701,0.507 0.470,0.069 1.191,0.191 1.602,0.271 0.411,0.080 0.926,0.174 1.143,0.210 0.612,0.101 0.870,0.498 0.795,1.228 -0.051,0.499 -0.118,0.638 -0.460,0.961 -0.220,0.208 -0.341,0.323 -0.406,0.344 -1.075,0.240 -2.326,0.513 -2.984,0.718 -0.346,0.091 -0.970,0.282 -1.196,0.137 -0.182,-0.162 -0.260,-0.228 -0.378,-0.401 z'
	},
	balloon: {
		viewBox: '-2 -2 13 17',
		path: 'M3.56,12.75a.49.49,0,0,1-.46-.34L2.63,11a.51.51,0,0,1,.07-.44l.1-.1-2-3.68a.48.48,0,0,1-.05-.17,4.39,4.39,0,0,1-.48-2A4.29,4.29,0,0,1,4.5.25,4.29,4.29,0,0,1,8.75,4.58a4.39,4.39,0,0,1-.48,2,.45.45,0,0,1-.05.17l-2,3.68a.44.44,0,0,1,.1.1.51.51,0,0,1,.07.45L5.9,12.41a.49.49,0,0,1-.46.34Zm1.6-2.43L6.1,8.59A4.22,4.22,0,0,1,5,8.88v1.44ZM4,10.32V8.88A4.22,4.22,0,0,1,2.9,8.59l.94,1.73Z'
	},
	cessna: {
		viewBox: '0 -1 32 31',
		path: 'M16.36 20.96l2.57.27s.44.05.4.54l-.02.63s-.03.47-.45.54l-2.31.34-.44-.74-.22 1.63-.25-1.62-.38.73-2.35-.35s-.44-.1-.43-.6l-.02-.6s0-.5.48-.5l2.5-.27-.56-5.4-3.64-.1-5.83-1.02h-.45v-2.06s-.07-.37.46-.34l5.8-.17 3.55.12s-.1-2.52.52-2.82l-1.68-.04s-.1-.06 0-.14l1.94-.03s.35-1.18.7 0l1.91.04s.11.05 0 .14l-1.7.02s.62-.09.56 2.82l3.54-.1 5.81.17s.51-.04.48.35l-.01 2.06h-.47l-5.8 1-3.67.11z'
	},
	jet: {
		viewBox: '-1 -1 20 26',
		path: 'M9.44,23c-.1.6-.35.6-.44.6s-.34,0-.44-.6l-3,.67V22.6A.54.54,0,0,1,6,22.05l2.38-1.12L8,19.33H6.69l0-.2a8.23,8.23,0,0,1-.14-3.85l.06-.18H7.73V13.19h-2L.26,14.29v-.93c0-.28.07-.46.22-.53l7.25-3.6V3.85A4.47,4.47,0,0,1,8.83.49L9,.34l.17.15a4.47,4.47,0,0,1,1.1,3.36V9.23l7.25,3.6c.14.07.22.25.22.53v.93l-5.51-1.1h-2V15.1h1.17l.06.18a8.24,8.24,0,0,1-.15,3.84l0,.2H10l-.36,1.6,2.43,1.14a.52.52,0,0,1,.35.53v1.08Z'
	},
	airliner: {
		viewBox: '-1 -2 34 34',
		path: 'M16 1c-.17 0-.67.58-.9 1.03-.6 1.21-.6 1.15-.65 5.2-.04 2.97-.08 3.77-.18 3.9-.15.17-1.82 1.1-1.98 1.1-.08 0-.1-.25-.05-.83.03-.5.01-.92-.05-1.08-.1-.25-.13-.26-.71-.26-.82 0-.86.07-.78 1.5.03.6.08 1.17.11 1.25.05.12-.02.2-.25.33l-8 4.2c-.2.2-.18.1-.19 1.29 3.9-1.2 3.71-1.21 3.93-1.21.06 0 .1 0 .13.14.08.3.28.3.28-.04 0-.25.03-.27 1.16-.6.65-.2 1.22-.35 1.28-.35.05 0 .12.04.15.17.07.3.27.27.27-.08 0-.25.01-.27.7-.47.68-.1.98-.09 1.47-.1.18 0 .22 0 .26.18.06.34.22.35.27-.01.04-.2.1-.17 1.06-.14l1.07.02.05 4.2c.05 3.84.07 4.28.26 5.09.11.49.2.99.2 1.11 0 .19-.31.43-1.93 1.5l-1.93 1.26v1.02l4.13-.95.63 1.54c.05.07.12.09.19.09s.14-.02.19-.09l.63-1.54 4.13.95V29.3l-1.93-1.27c-1.62-1.06-1.93-1.3-1.93-1.49 0-.12.09-.62.2-1.11.19-.81.2-1.25.26-5.09l.05-4.2 1.07-.02c.96-.03 1.02-.05 1.06.14.05.36.21.35.27 0 .04-.17.08-.16.26-.16.49 0 .8-.02 1.48.1.68.2.69.21.69.46 0 .35.2.38.27.08.03-.13.1-.17.15-.17.06 0 .63.15 1.28.34 1.13.34 1.16.36 1.16.61 0 .35.2.34.28.04.03-.13.07-.14.13-.14.22 0 .03 0 3.93 1.2-.01-1.18.02-1.07-.19-1.27l-8-4.21c-.23-.12-.3-.21-.25-.33.03-.08.08-.65.11-1.25.08-1.43.04-1.5-.78-1.5-.58 0-.61.01-.71.26-.06.16-.08.58-.05 1.08.04.58.03.83-.05.83-.16 0-1.83-.93-1.98-1.1-.1-.13-.14-.93-.18-3.9-.05-4.05-.05-3.99-.65-5.2C16.67 1.58 16.17 1 16 1z'
	},
	unknown: {
		viewBox: '-2.5 -2.5 22 22',
		path: 'M 4.256,15.496 C 3.979,14.340 7.280,13.606 7.280,13.606 V 8.650 l -6,2 c -0.680,0 -1,-0.350 -1,-0.660 C 0.242,9.595 0.496,9.231 0.880,9.130 1.140,9 4.800,7 7.280,5.630 V 3 C 7.280,1.890 7.720,0.290 8.510,0.290 9.300,0.290 9.770,1.840 9.770,3 v 2.630 c 2.450,1.370 6.100,3.370 6.370,3.500 0.390,0.093 0.651,0.461 0.610,0.860 -0.050,0.310 -0.360,0.670 -1.050,0.670 l -5.930,-2 v 4.946 c 0,0 3.300,0.734 3.024,1.890 -0.331,1.384 -2.830,0.378 -4.254,0.378 -1.434,4.520e-4 -3.950,1.016 -4.284,-0.378 z'
	}
};

/**
 * All available icon shapes
 */
const ALL_SHAPES: IconShape[] = [
	'helicopter',
	'glider',
	'balloon',
	'cessna',
	'jet',
	'airliner',
	'unknown'
];

/**
 * Map AircraftCategory to icon shape
 */
export function getIconShapeForCategory(category: AircraftCategory | null | undefined): IconShape {
	if (!category) return 'unknown';

	switch (category) {
		// Gliders
		case 'Glider':
		case 'HangGlider':
			return 'glider';

		// Paragliders/parachutes use balloon (similar round canopy shape)
		case 'Paraglider':
		case 'PoweredParachute':
		case 'SkydiverParachute':
			return 'balloon';

		// Rotorcraft
		case 'Helicopter':
		case 'Rotorcraft':
		case 'Gyroplane':
			return 'helicopter';

		// Tiltrotor uses airliner
		case 'Tiltrotor':
		case 'Vtol':
			return 'airliner';

		// Balloon/airship
		case 'Balloon':
		case 'Airship':
			return 'balloon';

		// Drones use unknown marker
		case 'Drone':
			return 'unknown';

		// Small fixed-wing aircraft
		case 'Landplane':
		case 'Seaplane':
		case 'Amphibian':
		case 'TowTug':
		case 'Electric':
			return 'cessna';

		// Static obstacles
		case 'StaticObstacle':
			return 'unknown';

		case 'Unknown':
		default:
			return 'unknown';
	}
}

/**
 * Get all icon shapes
 */
export function getAllIconShapes(): IconShape[] {
	return ALL_SHAPES;
}

/**
 * Create an SVG data URL for an aircraft icon (white fill for SDF coloring)
 *
 * @param shape - The icon shape
 * @param size - Icon size in pixels
 * @returns Data URL for the SVG image
 */
export function createAircraftIconDataUrl(shape: IconShape, size: number): string {
	const iconDef = ICON_SHAPES[shape];
	// White fill with black stroke - will be colored at runtime via icon-color
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${iconDef.viewBox}" width="${size}" height="${size}">
		<path d="${iconDef.path}" fill="#ffffff" stroke="#000000" stroke-width="1"/>
	</svg>`;
	return `data:image/svg+xml;base64,${btoa(svg)}`;
}

/**
 * Get icon definition for a shape
 */
export function getIconDefinition(shape: IconShape): { viewBox: string; path: string } {
	return ICON_SHAPES[shape];
}

/**
 * Get the MapLibre icon name for an aircraft shape
 * (no longer includes color - color is applied at runtime)
 */
export function getAircraftIconName(shape: IconShape): string {
	return `aircraft-${shape}`;
}

/**
 * Interpolate between two colors
 * @param color1 - Start color as [r, g, b]
 * @param color2 - End color as [r, g, b]
 * @param t - Interpolation factor (0-1)
 * @returns Interpolated color as [r, g, b]
 */
function lerpColor(
	color1: [number, number, number],
	color2: [number, number, number],
	t: number
): [number, number, number] {
	return [
		Math.round(color1[0] + (color2[0] - color1[0]) * t),
		Math.round(color1[1] + (color2[1] - color1[1]) * t),
		Math.round(color1[2] + (color2[2] - color1[2]) * t)
	];
}

/**
 * Convert RGB to hex color string
 */
function rgbToHex(rgb: [number, number, number]): string {
	return `#${rgb.map((c) => c.toString(16).padStart(2, '0')).join('')}`;
}

// Color gradient stops for altitude (red -> orange -> yellow -> green -> cyan -> light blue)
const ALTITUDE_GRADIENT: Array<{ altitude: number; color: [number, number, number] }> = [
	{ altitude: 0, color: [239, 68, 68] }, // Red #ef4444
	{ altitude: 5000, color: [249, 115, 22] }, // Orange #f97316
	{ altitude: 10000, color: [234, 179, 8] }, // Yellow #eab308
	{ altitude: 20000, color: [34, 197, 94] }, // Green #22c55e
	{ altitude: 30000, color: [6, 182, 212] }, // Cyan #06b6d4
	{ altitude: 40000, color: [56, 189, 248] } // Light blue (sky-400) #38bdf8
];

// Gray color for unknown altitude
const UNKNOWN_ALTITUDE_COLOR = '#6b7280';

/**
 * Get a fluid color based on altitude
 * Interpolates smoothly from red (ground) to light blue (40,000 ft)
 *
 * @param altitude - Altitude in feet MSL
 * @returns Hex color string
 */
export function getAltitudeColor(altitude: number | null | undefined): string {
	if (altitude === null || altitude === undefined) {
		return UNKNOWN_ALTITUDE_COLOR;
	}

	// Clamp altitude to valid range
	const clampedAlt = Math.max(0, Math.min(40000, altitude));

	// Find the two gradient stops we're between
	let lowerStop = ALTITUDE_GRADIENT[0];
	let upperStop = ALTITUDE_GRADIENT[ALTITUDE_GRADIENT.length - 1];

	for (let i = 0; i < ALTITUDE_GRADIENT.length - 1; i++) {
		if (
			clampedAlt >= ALTITUDE_GRADIENT[i].altitude &&
			clampedAlt < ALTITUDE_GRADIENT[i + 1].altitude
		) {
			lowerStop = ALTITUDE_GRADIENT[i];
			upperStop = ALTITUDE_GRADIENT[i + 1];
			break;
		}
	}

	// Calculate interpolation factor
	const range = upperStop.altitude - lowerStop.altitude;
	const t = range > 0 ? (clampedAlt - lowerStop.altitude) / range : 0;

	// Interpolate color
	const interpolatedColor = lerpColor(lowerStop.color, upperStop.color, t);
	return rgbToHex(interpolatedColor);
}

/**
 * Create a MapLibre expression for fluid altitude-based coloring
 * This expression interpolates colors smoothly based on altitude
 *
 * @returns MapLibre expression for icon-color property
 */
export function createAltitudeColorExpression(): ExpressionSpecification {
	return [
		'case',
		// If altitude is null/missing, use gray
		['==', ['get', 'altitude'], null],
		UNKNOWN_ALTITUDE_COLOR,
		// Otherwise interpolate based on altitude
		[
			'interpolate',
			['linear'],
			['get', 'altitude'],
			0,
			'#ef4444', // Red
			5000,
			'#f97316', // Orange
			10000,
			'#eab308', // Yellow
			20000,
			'#22c55e', // Green
			30000,
			'#06b6d4', // Cyan
			40000,
			'#38bdf8' // Light blue
		]
	];
}
