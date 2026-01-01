import { browser } from '$app/environment';
import { get } from 'svelte/store';
import { auth } from '$lib/stores/auth';
import { serverCall } from '$lib/api/server';
import { calculateDistance } from '$lib/geography';

// Configuration constants
const MIN_DISTANCE_KM = 0.03048; // 100 feet in km
const MIN_INTERVAL_MS = 60000; // 1 minute

interface LastFix {
	latitude: number;
	longitude: number;
	timestamp: number;
}

interface DeviceOrientation {
	alpha: number | null; // Compass direction (0-360, where 0 is north)
	beta: number | null; // Front/back tilt (-180 to 180)
	gamma: number | null; // Left/right tilt (-90 to 90)
	absolute: boolean;
	webkitCompassHeading?: number; // iOS-specific compass heading
}

interface UserFixPayload {
	latitude: number;
	longitude: number;
	heading: number | null;
	raw: Record<string, unknown> | null;
}

let watchId: number | null = null;
let lastFix: LastFix | null = null;
let isTracking = false;
let currentOrientation: DeviceOrientation | null = null;

/**
 * Calculate if we should send a new fix based on distance moved or time elapsed
 */
function shouldSendFix(newLat: number, newLon: number): boolean {
	const now = Date.now();

	if (!lastFix) {
		return true;
	}

	// Check if enough time has passed (1 minute)
	const timeElapsed = now - lastFix.timestamp;
	if (timeElapsed >= MIN_INTERVAL_MS) {
		return true;
	}

	// Check if moved more than 100 feet (~30 meters)
	const distanceKm = calculateDistance(lastFix.latitude, lastFix.longitude, newLat, newLon, 'km');
	if (distanceKm >= MIN_DISTANCE_KM) {
		return true;
	}

	return false;
}

/**
 * Get the best available compass heading
 * Prefers webkitCompassHeading (iOS) over alpha (Android/others)
 */
function getCompassHeading(): number | null {
	if (!currentOrientation) return null;

	// iOS provides webkitCompassHeading which is the true compass heading
	if (
		currentOrientation.webkitCompassHeading !== undefined &&
		currentOrientation.webkitCompassHeading !== null
	) {
		return currentOrientation.webkitCompassHeading;
	}

	// For other devices, use alpha (rotation around z-axis)
	// Note: alpha is relative to device orientation, not true north unless absolute is true
	if (currentOrientation.alpha !== null && currentOrientation.absolute) {
		// Convert alpha to compass heading (alpha is counter-clockwise, compass is clockwise)
		return (360 - currentOrientation.alpha) % 360;
	}

	return currentOrientation.alpha;
}

/**
 * Build the raw data object with all available sensor data
 */
function buildRawData(coords: GeolocationCoordinates): Record<string, unknown> {
	const raw: Record<string, unknown> = {};

	// Geolocation data
	if (coords.accuracy !== null) raw.accuracy = coords.accuracy;
	if (coords.altitude !== null) raw.altitude = coords.altitude;
	if (coords.altitudeAccuracy !== null) raw.altitudeAccuracy = coords.altitudeAccuracy;
	if (coords.speed !== null) raw.speed = coords.speed;
	if (coords.heading !== null) raw.gpsHeading = coords.heading; // GPS-based heading (direction of travel)

	// Device orientation data
	if (currentOrientation) {
		const orientation: Record<string, unknown> = {};
		if (currentOrientation.alpha !== null) orientation.alpha = currentOrientation.alpha;
		if (currentOrientation.beta !== null) orientation.beta = currentOrientation.beta;
		if (currentOrientation.gamma !== null) orientation.gamma = currentOrientation.gamma;
		orientation.absolute = currentOrientation.absolute;
		if (currentOrientation.webkitCompassHeading !== undefined) {
			orientation.webkitCompassHeading = currentOrientation.webkitCompassHeading;
		}
		if (Object.keys(orientation).length > 0) {
			raw.orientation = orientation;
		}
	}

	return Object.keys(raw).length > 0 ? raw : {};
}

/**
 * Send the user's location to the backend
 */
async function sendUserFix(position: GeolocationPosition): Promise<void> {
	const authState = get(auth);
	if (!authState.isAuthenticated || !authState.token) {
		return;
	}

	const { latitude, longitude } = position.coords;
	const heading = getCompassHeading();
	const raw = buildRawData(position.coords);

	const payload: UserFixPayload = {
		latitude,
		longitude,
		heading,
		raw: Object.keys(raw).length > 0 ? raw : null
	};

	try {
		// Use serverCall which handles 401 responses with toast notifications and logout
		await serverCall<void>('/user-fix', {
			method: 'POST',
			body: JSON.stringify(payload)
		});

		// Only update lastFix if the request succeeded
		lastFix = {
			latitude,
			longitude,
			timestamp: Date.now()
		};
	} catch (error) {
		// serverCall already handles 401 with toast + logout
		// Just log other errors silently to avoid spamming the console
		console.warn('Error sending user fix:', error);
	}
}

/**
 * Handle position updates from the geolocation API
 */
function handlePositionUpdate(position: GeolocationPosition): void {
	const { latitude, longitude } = position.coords;

	if (shouldSendFix(latitude, longitude)) {
		sendUserFix(position);
	}
}

/**
 * Handle geolocation errors
 */
function handlePositionError(error: GeolocationPositionError): void {
	switch (error.code) {
		case error.PERMISSION_DENIED:
			console.log('Location tracking: permission denied');
			stopTracking();
			break;
		case error.POSITION_UNAVAILABLE:
			console.warn('Location tracking: position unavailable');
			break;
		case error.TIMEOUT:
			console.warn('Location tracking: timeout');
			break;
	}
}

/**
 * Handle device orientation updates
 */
function handleDeviceOrientation(event: DeviceOrientationEvent): void {
	currentOrientation = {
		alpha: event.alpha,
		beta: event.beta,
		gamma: event.gamma,
		absolute: event.absolute,
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		webkitCompassHeading: (event as any).webkitCompassHeading
	};
}

/**
 * Start tracking the user's location
 * Only starts if the user is authenticated and has granted location permission
 */
export function startTracking(): void {
	if (!browser) return;
	if (isTracking) return;

	const authState = get(auth);
	if (!authState.isAuthenticated) {
		return;
	}

	const geolocation = navigator.geolocation;
	if (!geolocation) {
		console.log('Location tracking: geolocation not supported');
		return;
	}

	// Check permission state if available (modern browsers)
	if ('permissions' in navigator) {
		navigator.permissions.query({ name: 'geolocation' }).then((result) => {
			if (result.state === 'granted') {
				beginWatching();
			} else if (result.state === 'prompt') {
				// Don't prompt automatically - only track if already granted
				console.log('Location tracking: permission not yet granted');
			} else {
				console.log('Location tracking: permission denied');
			}
		});
	} else {
		// Fallback for browsers without permissions API
		// Try to get current position to check if permission is granted
		geolocation.getCurrentPosition(
			() => {
				beginWatching();
			},
			(error: GeolocationPositionError) => {
				if (error.code !== error.PERMISSION_DENIED) {
					// Permission might be granted but position unavailable
					beginWatching();
				}
			},
			{ maximumAge: 60000, timeout: 5000 }
		);
	}
}

/**
 * Begin watching position (called after permission is confirmed)
 */
function beginWatching(): void {
	if (watchId !== null) return;

	isTracking = true;

	// Start listening to device orientation for compass heading
	if ('DeviceOrientationEvent' in window) {
		window.addEventListener('deviceorientation', handleDeviceOrientation);
	}

	watchId = navigator.geolocation.watchPosition(handlePositionUpdate, handlePositionError, {
		enableHighAccuracy: false, // Save battery
		timeout: 30000,
		maximumAge: 30000 // Accept cached positions up to 30 seconds old
	});

	console.log('Location tracking: started');
}

/**
 * Stop tracking the user's location
 */
export function stopTracking(): void {
	if (!browser) return;

	if (watchId !== null) {
		navigator.geolocation.clearWatch(watchId);
		watchId = null;
	}

	// Stop listening to device orientation
	if ('DeviceOrientationEvent' in window) {
		window.removeEventListener('deviceorientation', handleDeviceOrientation);
	}

	isTracking = false;
	lastFix = null;
	currentOrientation = null;
	console.log('Location tracking: stopped');
}

/**
 * Check if location tracking is currently active
 */
export function isLocationTrackingActive(): boolean {
	return isTracking;
}
