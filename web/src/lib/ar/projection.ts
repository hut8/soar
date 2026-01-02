// AR projection utilities for converting 3D world coordinates to 2D screen positions

import type {
	ARScreenPosition,
	ARAircraftPosition,
	ARDeviceOrientation,
	ARSettings
} from './types';
import { normalizeBearing } from './calculations';

/**
 * Project 3D aircraft position to 2D screen coordinates
 * Uses camera field of view and device orientation to calculate screen position
 *
 * @param aircraft Aircraft position with bearing and elevation from user
 * @param deviceOrientation Current device heading, pitch, and roll
 * @param settings AR settings including FOV
 * @param screenWidth Screen width in pixels
 * @param screenHeight Screen height in pixels
 * @returns Screen position with x, y coordinates and visibility flag
 */
export function projectToScreen(
	aircraft: ARAircraftPosition,
	deviceOrientation: ARDeviceOrientation,
	settings: ARSettings,
	screenWidth: number,
	screenHeight: number
): ARScreenPosition {
	// Calculate bearing relative to device heading
	const relativeBearing = normalizeBearing(aircraft.bearing - deviceOrientation.heading);

	// Adjust elevation for device pitch (tilt)
	const adjustedElevation = aircraft.elevation - deviceOrientation.pitch;

	// Check if aircraft is within camera field of view
	const withinHorizontalFOV = Math.abs(relativeBearing) <= settings.fovHorizontal / 2;
	const withinVerticalFOV = Math.abs(adjustedElevation) <= settings.fovVertical / 2;
	const visible = withinHorizontalFOV && withinVerticalFOV;

	if (!visible) {
		return {
			x: -1000,
			y: -1000,
			visible: false,
			distance: aircraft.distance,
			bearing: relativeBearing,
			elevation: adjustedElevation
		};
	}

	// Map relative bearing to screen X coordinate
	// Center of screen = 0°, left edge = -FOV/2, right edge = +FOV/2
	const normalizedX = relativeBearing / (settings.fovHorizontal / 2); // -1 to 1
	const x = screenWidth / 2 + (normalizedX * screenWidth) / 2;

	// Map adjusted elevation to screen Y coordinate
	// Center of screen = 0°, top = +FOV/2, bottom = -FOV/2
	const normalizedY = -adjustedElevation / (settings.fovVertical / 2); // -1 to 1 (inverted)
	const y = screenHeight / 2 + (normalizedY * screenHeight) / 2;

	return {
		x: Math.round(x),
		y: Math.round(y),
		visible: true,
		distance: aircraft.distance,
		bearing: relativeBearing,
		elevation: adjustedElevation
	};
}

/**
 * Throttle function execution to max once per interval
 * Useful for limiting expensive projection calculations
 *
 * @param func Function to throttle
 * @param limitMs Minimum milliseconds between calls
 * @returns Throttled function
 */
export function throttle<T extends (...args: never[]) => unknown>(
	func: T,
	limitMs: number
): (...args: Parameters<T>) => void {
	let lastRan = 0;
	let timeout: number | null = null;

	return function (...args: Parameters<T>) {
		const now = Date.now();

		if (now - lastRan >= limitMs) {
			func(...args);
			lastRan = now;
		} else {
			if (timeout !== null) clearTimeout(timeout);
			timeout = window.setTimeout(
				() => {
					func(...args);
					lastRan = Date.now();
				},
				limitMs - (now - lastRan)
			);
		}
	};
}
