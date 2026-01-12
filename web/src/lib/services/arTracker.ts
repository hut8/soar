// AR tracking service for camera, GPS, and device orientation

import { browser } from '$app/environment';
import type { ARDeviceOrientation, ARUserPosition } from '$lib/ar/types';
import { getLogger } from '$lib/logging';

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

		window.addEventListener('deviceorientation', this.handleOrientationEvent);
	}

	private handleOrientationEvent = (event: DeviceOrientationEvent): void => {
		let heading = event.alpha ?? 0;

		// iOS provides webkitCompassHeading (true compass heading)
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		if ((event as any).webkitCompassHeading !== undefined) {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			heading = (event as any).webkitCompassHeading;
		} else if (event.absolute) {
			// Android: convert alpha to compass heading
			heading = 360 - heading;
		}

		this.currentOrientation = {
			heading,
			pitch: event.beta ?? 0,
			roll: event.gamma ?? 0,
			absolute: event.absolute
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

		window.removeEventListener('deviceorientation', this.handleOrientationEvent);

		this.currentOrientation = null;
		this.currentPosition = null;
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
