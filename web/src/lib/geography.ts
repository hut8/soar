// Geographic calculations and utilities

const EARTH_RADIUS_NM = 3440.065; // Earth radius in nautical miles
const EARTH_RADIUS_MI = 3958.8; // Earth radius in statute miles
const EARTH_RADIUS_KM = 6371; // Earth radius in kilometers

/**
 * Convert degrees to radians
 */
function toRadians(degrees: number): number {
	return (degrees * Math.PI) / 180;
}

/**
 * Convert radians to degrees
 */
function toDegrees(radians: number): number {
	return (radians * 180) / Math.PI;
}

/**
 * Calculate the great circle distance between two points using the Haversine formula
 * Returns distance in nautical miles by default
 */
export function calculateDistance(
	lat1: number,
	lon1: number,
	lat2: number,
	lon2: number,
	unit: 'nm' | 'mi' | 'km' = 'nm'
): number {
	const lat1Rad = toRadians(lat1);
	const lat2Rad = toRadians(lat2);
	const dLat = toRadians(lat2 - lat1);
	const dLon = toRadians(lon2 - lon1);

	const a =
		Math.sin(dLat / 2) * Math.sin(dLat / 2) +
		Math.cos(lat1Rad) * Math.cos(lat2Rad) * Math.sin(dLon / 2) * Math.sin(dLon / 2);

	const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));

	// Select radius based on desired unit
	let radius: number;
	switch (unit) {
		case 'mi':
			radius = EARTH_RADIUS_MI;
			break;
		case 'km':
			radius = EARTH_RADIUS_KM;
			break;
		case 'nm':
		default:
			radius = EARTH_RADIUS_NM;
			break;
	}

	return radius * c;
}

/**
 * Calculate the bearing from point 1 to point 2
 * Returns bearing in degrees (0-360, where 0 is North, 90 is East, etc.)
 */
export function calculateBearing(lat1: number, lon1: number, lat2: number, lon2: number): number {
	const lat1Rad = toRadians(lat1);
	const lat2Rad = toRadians(lat2);
	const dLon = toRadians(lon2 - lon1);

	const y = Math.sin(dLon) * Math.cos(lat2Rad);
	const x =
		Math.cos(lat1Rad) * Math.sin(lat2Rad) - Math.sin(lat1Rad) * Math.cos(lat2Rad) * Math.cos(dLon);

	let bearing = toDegrees(Math.atan2(y, x));

	// Normalize to 0-360
	bearing = (bearing + 360) % 360;

	return bearing;
}

/**
 * Get compass direction from bearing
 * Returns a string like "N", "NE", "E", "SE", etc.
 */
export function getCompassDirection(bearing: number): string {
	const directions = [
		'N',
		'NNE',
		'NE',
		'ENE',
		'E',
		'ESE',
		'SE',
		'SSE',
		'S',
		'SSW',
		'SW',
		'WSW',
		'W',
		'WNW',
		'NW',
		'NNW'
	];
	const index = Math.round(bearing / 22.5) % 16;
	return directions[index];
}

/**
 * Format distance with appropriate unit
 */
export function formatDistance(distanceNm: number): { nm: string; mi: string } {
	const distanceMi = distanceNm * 1.15078; // Convert nautical miles to statute miles

	return {
		nm: distanceNm < 10 ? distanceNm.toFixed(1) : Math.round(distanceNm).toString(),
		mi: distanceMi < 10 ? distanceMi.toFixed(1) : Math.round(distanceMi).toString()
	};
}

/**
 * Format bearing with degree symbol and compass direction
 */
export function formatBearing(bearing: number): string {
	const rounded = Math.round(bearing);
	const direction = getCompassDirection(bearing);
	return `${rounded}° ${direction}`;
}
