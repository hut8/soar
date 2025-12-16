/**
 * Shared map color utilities for both Google Maps and Cesium globe
 * Extracted from operations page and flights map page to avoid code duplication
 */

import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';

// Extend dayjs with relative time plugin
dayjs.extend(relativeTime);

/**
 * Default altitude range for color mapping (in feet MSL)
 * Red at 500 ft, blue at 18,000+ ft
 */
export const DEFAULT_MIN_ALTITUDE = 500;
export const DEFAULT_MAX_ALTITUDE = 18000;

/**
 * Map altitude to color using red→blue gradient
 * Red = low altitude, Blue = high altitude
 *
 * @param altitude - Altitude in feet MSL
 * @param min - Minimum altitude for color scale (default: 500 ft)
 * @param max - Maximum altitude for color scale (default: 18000 ft)
 * @returns RGB color string like "rgb(239, 68, 68)"
 */
export function altitudeToColor(
	altitude: number | null | undefined,
	min: number = DEFAULT_MIN_ALTITUDE,
	max: number = DEFAULT_MAX_ALTITUDE
): string {
	if (altitude === null || altitude === undefined || max === min) {
		return '#888888'; // Gray for unknown altitude
	}

	// Clamp altitude to min-max range
	const clampedAltitude = Math.max(min, Math.min(max, altitude));

	// Normalize altitude to 0-1 range
	const normalized = (clampedAltitude - min) / (max - min);

	// Interpolate from red (low) to blue (high)
	// Red: rgb(239, 68, 68) - #ef4444
	// Blue: rgb(59, 130, 246) - #3b82f6
	const r = Math.round(239 - normalized * (239 - 59));
	const g = Math.round(68 + normalized * (130 - 68));
	const b = Math.round(68 + normalized * (246 - 68));

	return `rgb(${r}, ${g}, ${b})`;
}

/**
 * Alias for altitudeToColor with default min/max values
 * Used by operations page for consistency
 */
export const getAltitudeColor = altitudeToColor;

/**
 * Map time/index to color using purple→orange gradient
 * Purple = early in flight, Orange = late in flight
 *
 * @param fixIndex - Index of the fix in the flight (0-based)
 * @param totalFixes - Total number of fixes in the flight
 * @returns RGB color string like "rgb(147, 51, 234)"
 */
export function timeToColor(fixIndex: number, totalFixes: number): string {
	if (totalFixes <= 1) {
		return '#888888'; // Gray for single fix
	}

	// Normalize index to 0-1 range
	const normalized = fixIndex / (totalFixes - 1);

	// Interpolate from purple (early) to orange (late)
	// Purple: rgb(147, 51, 234) - #9333ea (Tailwind purple-600)
	// Orange: rgb(251, 146, 60) - #fb923c (Tailwind orange-400)
	const r = Math.round(147 + normalized * (251 - 147));
	const g = Math.round(51 + normalized * (146 - 51));
	const b = Math.round(234 - normalized * (234 - 60));

	return `rgb(${r}, ${g}, ${b})`;
}

/**
 * Format altitude with relative time
 * Shows altitude in feet with time since last fix (e.g., "5000ft 2 minutes ago")
 *
 * @param altitude_msl_feet - Altitude in feet MSL
 * @param timestamp - ISO 8601 timestamp string
 * @returns Object with formatted text and isOld flag
 */
export function formatAltitudeWithTime(
	altitude_msl_feet: number | null | undefined,
	timestamp: string
): {
	altitudeText: string;
	isOld: boolean;
} {
	const altitudeFt = altitude_msl_feet ? `${altitude_msl_feet}ft` : '---ft';

	// Calculate relative time, handling edge cases
	const fixTime = dayjs(timestamp);
	const now = dayjs();
	const diffSeconds = now.diff(fixTime, 'second');

	// If timestamp is in the future or within 10 seconds, show "just now"
	let relativeTimeText: string;
	if (diffSeconds >= -10 && diffSeconds <= 10) {
		relativeTimeText = 'just now';
	} else {
		relativeTimeText = fixTime.fromNow();
	}

	const altitudeText = `${altitudeFt} ${relativeTimeText}`;

	// Check if fix is more than 5 minutes old
	const diffMinutes = now.diff(fixTime, 'minute');
	const isOld = diffMinutes > 5;

	return { altitudeText, isOld };
}

/**
 * Get marker color based on active status and altitude
 * Inactive fixes show as gray, active fixes use altitude-based color
 *
 * @param isActive - Whether the fix belongs to an active flight
 * @param altitude_msl_feet - Altitude in feet MSL
 * @returns RGB color string
 */
export function getMarkerColor(
	isActive: boolean,
	altitude_msl_feet: number | null | undefined
): string {
	// Use gray for inactive fixes (no current flight)
	if (!isActive) {
		return 'rgb(156, 163, 175)'; // Tailwind gray-400
	}
	// Use altitude-based color for active fixes
	return altitudeToColor(altitude_msl_feet);
}
