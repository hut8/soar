// AR tracking service for camera, GPS, and device orientation

import { browser } from '$app/environment';
import type { ARDeviceOrientation, ARUserPosition } from '$lib/ar/types';
import type { DeviceOrientationEventWithCompass } from '$lib/types';
import { getLogger } from '$lib/logging';

/**
 * Check if deviceorientationabsolute event is supported.
 * This event provides compass-calibrated heading on Android devices.
 */
function hasDeviceOrientationAbsolute(): boolean {
	return typeof window !== 'undefined' && 'ondeviceorientationabsolute' in window;
}

const logger = getLogger(['soar', 'ARTracker']);

export type ARTrackerEvent =
	| { type: 'position_updated'; position: ARUserPosition }
	| { type: 'orientation_updated'; orientation: ARDeviceOrientation }
	| { type: 'camera_ready'; stream: MediaStream }
	| { type: 'camera_error'; error: string }
	| { type: 'permission_denied'; permission: 'camera' | 'location' | 'orientation' };

export type ARTrackerSubscriber = (event: ARTrackerEvent) => void;

/**
 * ARTracker singleton service
 * Manages camera stream, GPS tracking, and device orientation
 * Provides event-based updates to subscribers
 */
export class ARTracker {
	private static instance: ARTracker | null = null;
	private subscribers = new Set<ARTrackerSubscriber>();
	private cameraStream: MediaStream | null = null;
	private watchId: number | null = null;
	private currentOrientation: ARDeviceOrientation | null = null;
	private currentPosition: ARUserPosition | null = null;

	// Smoothing state: exponential moving average on heading/pitch
	// Alpha controls responsiveness: 0 = no update, 1 = no smoothing
	private smoothingAlpha = 0.3;
	private smoothedHeading: number | null = null;
	private smoothedPitch: number | null = null;

	private constructor() {}

	public static getInstance(): ARTracker {
		if (!ARTracker.instance) {
			ARTracker.instance = new ARTracker();
		}
		return ARTracker.instance;
	}

	public subscribe(subscriber: ARTrackerSubscriber): () => void {
		this.subscribers.add(subscriber);
		return () => this.subscribers.delete(subscriber);
	}

	private notify(event: ARTrackerEvent): void {
		this.subscribers.forEach((sub) => {
			try {
				sub(event);
			} catch (error) {
				logger.error('ARTracker subscriber error: {error}', { error });
			}
		});
	}

	/**
	 * Request camera permission and start video stream
	 */
	public async startCamera(): Promise<void> {
		if (!browser) return;

		try {
			const stream = await navigator.mediaDevices.getUserMedia({
				video: {
					facingMode: 'environment', // Rear camera
					width: { ideal: 1920 },
					height: { ideal: 1080 }
				}
			});

			this.cameraStream = stream;
			this.notify({ type: 'camera_ready', stream });
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Camera access denied';

			if (errorMessage.includes('Permission denied') || errorMessage.includes('NotAllowedError')) {
				this.notify({ type: 'permission_denied', permission: 'camera' });
			} else {
				this.notify({ type: 'camera_error', error: errorMessage });
			}
		}
	}

	/**
	 * Request location permission and start GPS tracking
	 */
	public startLocation(): void {
		if (!browser) return;

		if (!navigator.geolocation) {
			this.notify({ type: 'camera_error', error: 'Geolocation not supported' });
			return;
		}

		this.watchId = navigator.geolocation.watchPosition(
			(position) => {
				this.currentPosition = {
					latitude: position.coords.latitude,
					longitude: position.coords.longitude,
					altitude: position.coords.altitude ?? 0,
					accuracy: position.coords.accuracy
				};
				this.notify({ type: 'position_updated', position: this.currentPosition });
			},
			(error) => {
				if (error.code === error.PERMISSION_DENIED) {
					this.notify({ type: 'permission_denied', permission: 'location' });
				}
			},
			{
				enableHighAccuracy: true,
				timeout: 10000,
				maximumAge: 5000
			}
		);
	}

	// Track whether we have absolute orientation support
	private hasAbsoluteOrientation = false;
	private orientationEventType: 'deviceorientationabsolute' | 'deviceorientation' | null = null;

	/**
	 * Request device orientation permission (iOS 13+) and start tracking
	 */
	public async startOrientation(): Promise<void> {
		if (!browser) return;

		// iOS 13+ requires permission for device orientation
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		if (typeof (DeviceOrientationEvent as any).requestPermission === 'function') {
			try {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const permission = await (DeviceOrientationEvent as any).requestPermission();
				if (permission !== 'granted') {
					this.notify({ type: 'permission_denied', permission: 'orientation' });
					return;
				}
			} catch (error) {
				logger.error('Orientation permission error: {error}', { error });
				return;
			}
		}

		// Try deviceorientationabsolute first (better for Android compass)
		// Fall back to deviceorientation if absolute is not available
		if (hasDeviceOrientationAbsolute()) {
			window.addEventListener(
				'deviceorientationabsolute' as keyof WindowEventMap,
				this.handleOrientationEvent as EventListener
			);
			this.orientationEventType = 'deviceorientationabsolute';
			this.hasAbsoluteOrientation = true;
			logger.debug('Using deviceorientationabsolute for compass');
		} else {
			window.addEventListener('deviceorientation', this.handleOrientationEvent);
			this.orientationEventType = 'deviceorientation';
			logger.debug('Using deviceorientation for compass (absolute not available)');
		}
	}

	/**
	 * Smooth a circular value (like heading in degrees) using exponential
	 * moving average. Takes the shortest angular path to avoid jumps at
	 * the 0°/360° boundary.
	 */
	private smoothAngle(current: number, target: number, wrap: number): number {
		let delta = target - current;
		// Take the shortest path around the circle
		if (delta > wrap / 2) delta -= wrap;
		if (delta < -wrap / 2) delta += wrap;
		const result = current + this.smoothingAlpha * delta;
		return ((result % wrap) + wrap) % wrap;
	}

	private handleOrientationEvent = (event: DeviceOrientationEventWithCompass): void => {
		let rawHeading: number;

		// iOS provides webkitCompassHeading (true compass heading)
		if (event.webkitCompassHeading !== undefined && event.webkitCompassHeading !== null) {
			rawHeading = event.webkitCompassHeading;
		} else if (event.absolute || this.hasAbsoluteOrientation) {
			// Android with absolute orientation: convert alpha to compass heading
			// alpha is measured counter-clockwise from north, compass heading is clockwise
			rawHeading = (360 - (event.alpha ?? 0)) % 360;
		} else {
			// Fallback: use raw alpha (may be inaccurate)
			rawHeading = (360 - (event.alpha ?? 0)) % 360;
		}

		// Convert device beta to AR pitch
		// beta is the angle in degrees the device is tilted front-to-back:
		//   - beta = 0°: phone flat on table, screen facing up
		//   - beta = 90°: phone held vertically, screen facing user (portrait AR mode)
		//   - beta = 180°/-180°: phone flat, screen facing down
		// For AR, we want pitch = 0° when phone is held vertically (the typical AR pose).
		// Tilting the phone up (looking at sky) = positive pitch
		// Tilting the phone down (looking at ground) = negative pitch
		const beta = event.beta ?? 0;
		const rawPitch = beta - 90;

		// Apply low-pass filter to reduce jitter from noisy sensors
		if (this.smoothedHeading === null || this.smoothedPitch === null) {
			this.smoothedHeading = rawHeading;
			this.smoothedPitch = rawPitch;
		} else {
			this.smoothedHeading = this.smoothAngle(this.smoothedHeading, rawHeading, 360);
			this.smoothedPitch =
				this.smoothedPitch + this.smoothingAlpha * (rawPitch - this.smoothedPitch);
		}

		this.currentOrientation = {
			heading: this.smoothedHeading,
			pitch: this.smoothedPitch,
			roll: event.gamma ?? 0,
			absolute: event.absolute || this.hasAbsoluteOrientation
		};

		this.notify({
			type: 'orientation_updated',
			orientation: this.currentOrientation
		});
	};

	/**
	 * Stop all tracking and release camera
	 */
	public stop(): void {
		if (this.cameraStream) {
			this.cameraStream.getTracks().forEach((track) => track.stop());
			this.cameraStream = null;
		}

		if (this.watchId !== null) {
			navigator.geolocation.clearWatch(this.watchId);
			this.watchId = null;
		}

		// Remove the correct event listener type
		if (this.orientationEventType) {
			window.removeEventListener(
				this.orientationEventType as keyof WindowEventMap,
				this.handleOrientationEvent as EventListener
			);
			this.orientationEventType = null;
		}

		this.currentOrientation = null;
		this.currentPosition = null;
		this.hasAbsoluteOrientation = false;
		this.smoothedHeading = null;
		this.smoothedPitch = null;
	}

	public getCurrentPosition(): ARUserPosition | null {
		return this.currentPosition;
	}

	public getCurrentOrientation(): ARDeviceOrientation | null {
		return this.currentOrientation;
	}

	public getCameraStream(): MediaStream | null {
		return this.cameraStream;
	}
}
