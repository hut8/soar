import { browser, dev } from '$app/environment';
import type { Fix } from '$lib/types';
import { DeviceRegistry } from './DeviceRegistry';

// Event types for subscribers
export type FixFeedEvent =
	| { type: 'connection_opened' }
	| { type: 'connection_closed'; code: number; reason: string }
	| { type: 'connection_error'; error: Event }
	| { type: 'fix_received'; fix: Fix }
	| { type: 'subscription_added'; deviceId: string }
	| { type: 'subscription_removed'; deviceId: string }
	| { type: 'reconnecting'; attempt: number; maxAttempts: number };

export type FixFeedSubscriber = (event: FixFeedEvent) => void;

export interface DeviceSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: "device";
	id: string;
}

export interface AreaSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: "area";
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
	private readonly maxReconnectAttempts = 5;
	private readonly reconnectDelay = 1000; // Start with 1 second
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

		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const host = dev ? 'localhost:1337' : window.location.host;
		this.websocketUrl = `${protocol}//${host}/data/fixes/live`;
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
					const rawFix = JSON.parse(event.data);
					console.log('Received fix:', rawFix);

					// Transform WebSocket fix data to match Fix interface
					const fix: Fix = {
						id: rawFix.id,
						device_id: rawFix.device_id,
						device_address_hex: rawFix.device_address_hex,
						timestamp: rawFix.timestamp,
						latitude: rawFix.latitude,
						longitude: rawFix.longitude,
						altitude_feet: rawFix.altitude,
						track_degrees: rawFix.track,
						ground_speed_knots: rawFix.ground_speed,
						climb_fpm: rawFix.climb_rate,
						registration: rawFix.registration,
						model: rawFix.model,
						flight_id: rawFix.flight_id
					};

					// Add fix to device registry
					this.deviceRegistry.addFixToDevice(fix).catch(error => {
						console.warn('Failed to add fix to device registry:', error);
					});

					// Notify subscribers
					this.notifySubscribers({
						type: 'fix_received',
						fix
					});
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

				// Attempt to reconnect if it wasn't a clean close
				if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
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

	// Attempt to reconnect with exponential backoff
	private attemptReconnect(): void {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			console.log('Max reconnection attempts reached');
			return;
		}

		this.reconnectAttempts++;
		const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

		console.log(
			`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`
		);

		this.notifySubscribers({
			type: 'reconnecting',
			attempt: this.reconnectAttempts,
			maxAttempts: this.maxReconnectAttempts
		});

		setTimeout(() => {
			if (!this.websocket || this.websocket.readyState === WebSocket.CLOSED) {
				this.connect();
			}
		}, delay);
	}

	// Subscribe to a specific device
	public async subscribeToDevice(deviceId: string): Promise<void> {
		if (this.subscribedDevices.has(deviceId)) {
			return; // Already subscribed
		}

		// Connect if not already connected
		if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
			this.connect();
			// Wait a bit for connection to establish
			await new Promise((resolve) => setTimeout(resolve, 100));
		}

		if (this.websocket?.readyState === WebSocket.OPEN) {
			this.sendSubscriptionMessage('subscribe', deviceId);
			this.subscribedDevices.add(deviceId);

			this.notifySubscribers({
				type: 'subscription_added',
				deviceId
			});

			// Fetch device info and recent fixes from API
			await this.deviceRegistry.updateDeviceFromAPI(deviceId);
			await this.deviceRegistry.loadRecentFixesFromAPI(deviceId);
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
				type: "device",
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
}
