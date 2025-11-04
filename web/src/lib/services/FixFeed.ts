import { browser, dev } from '$app/environment';
import type { Aircraft, Fix } from '$lib/types';
import { DeviceRegistry } from './DeviceRegistry';
import { FORCE_PRODUCTION_BACKEND } from '$lib/api/server';

// Event types for subscribers
export type FixFeedEvent =
	| { type: 'connection_opened' }
	| { type: 'connection_closed'; code: number; reason: string }
	| { type: 'connection_error'; error: Event }
	| { type: 'fix_received'; fix: Fix }
	| { type: 'device_received'; device: Aircraft }
	| { type: 'subscription_added'; deviceId: string }
	| { type: 'subscription_removed'; deviceId: string }
	| { type: 'reconnecting'; attempt: number };

export type FixFeedSubscriber = (event: FixFeedEvent) => void;

export interface DeviceSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: 'device';
	id: string;
}

export interface AreaSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: 'area';
	latitude: number;
	longitude: number;
}

export type SubscriptionMessage = DeviceSubscriptionMessage | AreaSubscriptionMessage;

export class FixFeed {
	private static instance: FixFeed | null = null;
	private websocket: WebSocket | null = null;
	private websocketUrl = '';
	private subscribers = new Set<FixFeedSubscriber>();
	private subscribedDevices = new Set<string>();
	private reconnectAttempts = 0;
	private readonly reconnectDelay = 5000; // Fixed 5 second delay
	private operationsPageActive = false;
	private deviceRegistry: DeviceRegistry;

	private constructor() {
		this.deviceRegistry = DeviceRegistry.getInstance();
		this.initializeWebSocketUrl();
	}

	// Singleton instance getter
	public static getInstance(): FixFeed {
		if (!FixFeed.instance) {
			FixFeed.instance = new FixFeed();
		}
		return FixFeed.instance;
	}

	// Subscription management
	public subscribe(subscriber: FixFeedSubscriber): () => void {
		this.subscribers.add(subscriber);

		// Return unsubscribe function
		return () => {
			this.subscribers.delete(subscriber);
		};
	}

	private notifySubscribers(event: FixFeedEvent): void {
		this.subscribers.forEach((subscriber) => {
			try {
				subscriber(event);
			} catch (error) {
				console.error('Error in FixFeed subscriber:', error);
			}
		});
	}

	// Initialize WebSocket URL based on environment
	private initializeWebSocketUrl(): void {
		if (!browser) return;

		if (dev && !FORCE_PRODUCTION_BACKEND) {
			// Dev mode with local backend
			this.websocketUrl = 'ws://localhost:1337/data/fixes/live';
		} else if (dev && FORCE_PRODUCTION_BACKEND) {
			// Dev mode forcing production backend
			this.websocketUrl = 'wss://glider.flights/data/fixes/live';
		} else {
			// Production mode - use current host
			const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
			this.websocketUrl = `${protocol}//${window.location.host}/data/fixes/live`;
		}
	}

	// Connect to WebSocket
	public connect(): void {
		if (!browser) return;

		// Prevent multiple connections
		if (
			this.websocket?.readyState === WebSocket.OPEN ||
			this.websocket?.readyState === WebSocket.CONNECTING
		) {
			console.log('WebSocket already connected or connecting, skipping connection attempt');
			return;
		}

		try {
			this.websocket = new WebSocket(this.websocketUrl);

			this.websocket.onopen = () => {
				console.log('WebSocket connected to live fixes feed');
				this.reconnectAttempts = 0;

				this.notifySubscribers({ type: 'connection_opened' });

				// Re-subscribe to all previously subscribed devices
				setTimeout(() => {
					if (this.websocket?.readyState === WebSocket.OPEN) {
						this.subscribedDevices.forEach((deviceId) => {
							this.sendSubscriptionMessage('subscribe', deviceId);
						});
					}
				}, 50);
			};

			this.websocket.onmessage = (event) => {
				try {
					const rawMessage = JSON.parse(event.data);
					console.log('Received WebSocket message:', rawMessage);

					// Handle different message types based on the "type" field
					if (rawMessage.type === 'fix') {
						// Transform WebSocket fix data to match Fix interface
						const fix: Fix = {
							id: rawMessage.id,
							device_id: rawMessage.device_id,
							device_address_hex: rawMessage.device_address_hex,
							timestamp: rawMessage.timestamp,
							latitude: rawMessage.latitude,
							longitude: rawMessage.longitude,
							altitude_msl_feet: rawMessage.altitude_msl_feet,
							altitude_agl_feet: rawMessage.altitude_agl_feet,
							track_degrees: rawMessage.track,
							ground_speed_knots: rawMessage.ground_speed,
							climb_fpm: rawMessage.climb_rate,
							registration: rawMessage.registration,
							model: rawMessage.model,
							flight_id: rawMessage.flight_id,
							active: rawMessage.active
						};

						// Add fix to device registry
						// For fixes from WebSocket, assume device data is provided via device messages
						// so don't attempt API fallback to avoid N+1 calls
						this.deviceRegistry.addFixToDevice(fix, false).catch((error) => {
							console.warn('Failed to add fix to device registry:', error);
						});

						// Notify subscribers
						this.notifySubscribers({
							type: 'fix_received',
							fix
						});
					} else if (rawMessage.type === 'device') {
						// Handle Aircraft message
						const aircraft: Aircraft = rawMessage;

						// Add all recent fixes to device registry
						const aircraftFixes = aircraft.fixes || [];
						if (aircraftFixes.length > 0) {
							for (const fix of aircraftFixes) {
								this.deviceRegistry.addFixToDevice(fix, false).catch((error) => {
									console.warn('Failed to add device fix to registry:', error);
								});
							}
						}

						// Update device registry with complete device info
						this.deviceRegistry.updateDeviceFromAircraft(aircraft).catch((error) => {
							console.warn('Failed to update device info in registry:', error);
						});

						// Notify subscribers
						this.notifySubscribers({
							type: 'device_received',
							device: aircraft
						});
					} else {
						console.warn('Unknown WebSocket message type:', rawMessage.type);
					}
				} catch (e) {
					console.warn('Failed to parse WebSocket message:', e);
				}
			};

			this.websocket.onclose = (event) => {
				console.log('WebSocket disconnected', event.code, event.reason);
				this.websocket = null;

				this.notifySubscribers({
					type: 'connection_closed',
					code: event.code,
					reason: event.reason
				});

				// Attempt to reconnect if it wasn't a clean close (always retry)
				if (event.code !== 1000) {
					this.attemptReconnect();
				}
			};

			this.websocket.onerror = (error) => {
				console.error('WebSocket error:', error);
				this.notifySubscribers({
					type: 'connection_error',
					error
				});
			};
		} catch (e) {
			console.error('Failed to create WebSocket connection:', e);
		}
	}

	// Disconnect from WebSocket
	public disconnect(): void {
		if (this.websocket) {
			this.websocket.close(1000, 'User requested disconnection'); // Clean close
			this.websocket = null;
		}

		// Reset state
		this.reconnectAttempts = 0;
		this.operationsPageActive = false;
	}

	// Attempt to reconnect with fixed delay (keeps retrying indefinitely)
	private attemptReconnect(): void {
		this.reconnectAttempts++;

		console.log(
			`Attempting to reconnect in ${this.reconnectDelay}ms (attempt ${this.reconnectAttempts})`
		);

		this.notifySubscribers({
			type: 'reconnecting',
			attempt: this.reconnectAttempts
		});

		setTimeout(() => {
			if (!this.websocket || this.websocket.readyState === WebSocket.CLOSED) {
				this.connect();
			}
		}, this.reconnectDelay);
	}

	// Wait for WebSocket connection to be open
	private async waitForConnection(timeoutMs = 2000): Promise<boolean> {
		if (this.websocket?.readyState === WebSocket.OPEN) {
			return true;
		}

		return new Promise((resolve) => {
			const startTime = Date.now();
			const checkInterval = setInterval(() => {
				if (this.websocket?.readyState === WebSocket.OPEN) {
					clearInterval(checkInterval);
					resolve(true);
				} else if (Date.now() - startTime > timeoutMs) {
					clearInterval(checkInterval);
					resolve(false);
				}
			}, 50);
		});
	}

	// Subscribe to a specific device
	public async subscribeToDevice(deviceId: string): Promise<void> {
		if (this.subscribedDevices.has(deviceId)) {
			return; // Already subscribed
		}

		// Connect if not already connected
		if (!this.websocket || this.websocket.readyState === WebSocket.CLOSED) {
			this.connect();
		}

		// Wait for connection to be established
		const isConnected = await this.waitForConnection();

		if (isConnected && this.websocket?.readyState === WebSocket.OPEN) {
			this.sendSubscriptionMessage('subscribe', deviceId);
			this.subscribedDevices.add(deviceId);

			this.notifySubscribers({
				type: 'subscription_added',
				deviceId
			});

			// Fetch device info and recent fixes from API
			await this.deviceRegistry.updateDeviceFromAPI(deviceId);
			await this.deviceRegistry.loadRecentFixesFromAPI(deviceId);
		} else {
			console.error(`Failed to subscribe to device ${deviceId}: WebSocket not connected`);
		}
	}

	// Unsubscribe from a specific device
	public unsubscribeFromDevice(deviceId: string): void {
		if (!this.subscribedDevices.has(deviceId)) {
			return; // Not subscribed
		}

		if (this.websocket?.readyState === WebSocket.OPEN) {
			this.sendSubscriptionMessage('unsubscribe', deviceId);
		}

		this.subscribedDevices.delete(deviceId);

		this.notifySubscribers({
			type: 'subscription_removed',
			deviceId
		});

		// Disconnect if no subscriptions and operations page not active
		if (this.subscribedDevices.size === 0 && !this.operationsPageActive) {
			this.disconnect();
		}
	}

	// Send subscription message to server
	private sendSubscriptionMessage(action: string, deviceId: string): void {
		if (this.websocket?.readyState === WebSocket.OPEN) {
			const message: DeviceSubscriptionMessage = {
				action,
				type: 'device',
				id: deviceId
			};
			this.websocket.send(JSON.stringify(message));
		}
	}

	// Send any subscription message to server (for area subscriptions)
	public sendWebSocketMessage(message: SubscriptionMessage): void {
		if (this.websocket?.readyState === WebSocket.OPEN) {
			this.websocket.send(JSON.stringify(message));
		}
	}

	// Start live fixes feed for operations page (connects without specific subscriptions)
	public startLiveFixesFeed(): void {
		if (!browser) return;

		console.log('Starting live fixes feed for operations page');
		this.operationsPageActive = true;
		this.connect();
	}

	// Stop live fixes feed for operations page
	public stopLiveFixesFeed(): void {
		if (!browser) return;

		console.log('Stopping live fixes feed for operations page');
		this.operationsPageActive = false;

		// Only disconnect if there are no active device subscriptions
		if (this.subscribedDevices.size === 0) {
			this.disconnect();
		}
	}

	// Get current connection status
	public getConnectionStatus(): {
		connected: boolean;
		reconnecting: boolean;
		subscribedDevices: string[];
		operationsPageActive: boolean;
	} {
		return {
			connected: this.websocket?.readyState === WebSocket.OPEN,
			reconnecting: this.reconnectAttempts > 0,
			subscribedDevices: Array.from(this.subscribedDevices),
			operationsPageActive: this.operationsPageActive
		};
	}

	// Subscribe to multiple devices from watchlist
	public async subscribeToWatchlist(deviceIds: string[]): Promise<void> {
		// Unsubscribe from devices no longer in the list
		const devicesToUnsubscribe = Array.from(this.subscribedDevices).filter(
			(deviceId) => !deviceIds.includes(deviceId)
		);

		for (const deviceId of devicesToUnsubscribe) {
			this.unsubscribeFromDevice(deviceId);
		}

		// Subscribe to new devices
		const devicesToSubscribe = deviceIds.filter(
			(deviceId) => !this.subscribedDevices.has(deviceId)
		);

		for (const deviceId of devicesToSubscribe) {
			await this.subscribeToDevice(deviceId);
		}
	}

	// Fetch devices in bounding box via REST API
	public async fetchDevicesInBoundingBox(
		latMin: number,
		latMax: number,
		lonMin: number,
		lonMax: number,
		afterTimestamp?: string // Expected in ISO 8601 format
	): Promise<Aircraft[]> {
		if (!browser) return [];

		try {
			const { serverCall } = await import('$lib/api/server');
			const response = await serverCall('/devices', {
				params: {
					latitude_min: latMin,
					latitude_max: latMax,
					longitude_min: lonMin,
					longitude_max: lonMax,
					...(afterTimestamp && { after: afterTimestamp })
				}
			});
			return response as Aircraft[];
		} catch (error) {
			console.error('Failed to fetch devices in bounding box:', error);
			return [];
		}
	}
}
