// AR-specific geographic calculations

import { calculateDistance, calculateBearing } from '$lib/geography';
import type { ARUserPosition, ARAircraftPosition } from './types';
import type { Fix } from '$lib/types';
import type { GeoBounds } from '$lib/services/FixFeed';

const METERS_TO_FEET = 3.28084;
const NM_TO_FEET = 6076.12; // 1 nautical mile = 6076.12 feet

/**
 * Calculate elevation angle from user to aircraft
 * Returns angle in degrees above horizon (-90 to 90)
 */
export function calculateElevationAngle(
	userPosition: ARUserPosition,
	aircraftAltitudeFeet: number,
	groundDistanceNm: number
): number {
	const userAltitudeFeet = userPosition.altitude * METERS_TO_FEET;
	const altitudeDifferenceFeet = aircraftAltitudeFeet - userAltitudeFeet;
	const groundDistanceFeet = groundDistanceNm * NM_TO_FEET;

	if (groundDistanceFeet === 0) return 0;

	const elevationRadians = Math.atan2(altitudeDifferenceFeet, groundDistanceFeet);
	return elevationRadians * (180 / Math.PI); // Convert to degrees
}

/**
 * Convert aircraft Fix to AR position with calculated metrics
 * @param fix Aircraft fix data
 * @param userPosition Current user position
 * @param registration Optional aircraft registration (if not in fix)
 * @returns ARAircraftPosition or null if fix data is invalid
 */
export function fixToARPosition(
	fix: Fix,
	userPosition: ARUserPosition,
	registration?: string | null,
	clubName?: string | null
): ARAircraftPosition | null {
	if (!fix.aircraftId || fix.latitude == null || fix.longitude == null) {
		return null;
	}

	const distance = calculateDistance(
		userPosition.latitude,
		userPosition.longitude,
		fix.latitude,
		fix.longitude,
		'nm'
	);

	const bearing = calculateBearing(
		userPosition.latitude,
		userPosition.longitude,
		fix.latitude,
		fix.longitude
	);

	const altitudeFeet = fix.altitudeMslFeet ?? 0;
	const elevation = calculateElevationAngle(userPosition, altitudeFeet, distance);

	return {
		aircraftId: fix.aircraftId,
		registration: registration ?? null,
		clubName: clubName ?? null,
		latitude: fix.latitude,
		longitude: fix.longitude,
		altitudeFeet,
		groundSpeedKnots: fix.groundSpeedKnots ?? null,
		climbFpm: fix.climbFpm ?? null,
		timestamp: fix.receivedAt,
		distance,
		bearing,
		elevation
	};
}

const NM_TO_KM = 1.852; // 1 nautical mile = 1.852 km

/**
 * Calculate bounding box for aircraft subscription
 * Returns geographic bounds for a circle of given radius around center point
 */
export function calculateBoundingBox(
	centerLat: number,
	centerLon: number,
	radiusNm: number
): GeoBounds {
	const radiusKm = radiusNm * NM_TO_KM;
	const earthRadiusKm = 6371;
	const latDelta = (radiusKm / earthRadiusKm) * (180 / Math.PI);
	const lonDelta = latDelta / Math.abs(Math.cos((centerLat * Math.PI) / 180));

	return {
		north: Math.min(90, centerLat + latDelta),
		south: Math.max(-90, centerLat - latDelta),
		east: centerLon + lonDelta,
		west: centerLon - lonDelta
	};
}

/**
 * Normalize bearing to -180 to 180 range
 * Useful for calculating relative bearings
 */
export function normalizeBearing(bearing: number): number {
	let normalized = bearing;
	while (normalized > 180) normalized -= 360;
	while (normalized < -180) normalized += 360;
	return normalized;
}
