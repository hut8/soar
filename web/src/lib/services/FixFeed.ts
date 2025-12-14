import { browser, dev } from '$app/environment';
import type { Aircraft, Fix } from '$lib/types';
import { AircraftRegistry } from './AircraftRegistry';
import { backendMode } from '$lib/stores/backend';
import { get } from 'svelte/store';

// Event types for subscribers
export type FixFeedEvent =
	| { type: 'connection_opened' }
	| { type: 'connection_closed'; code: number; reason: string }
	| { type: 'connection_error'; error: Event }
	| { type: 'fix_received'; fix: Fix }
	| { type: 'aircraft_received'; aircraft: Aircraft }
	| { type: 'subscription_added'; aircraftId: string }
	| { type: 'subscription_removed'; aircraftId: string }
	| { type: 'reconnecting'; attempt: number };

export type FixFeedSubscriber = (event: FixFeedEvent) => void;

export interface AircraftSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: 'aircraft';
	id: string;
}

export interface AreaSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: 'area';
	latitude: number;
	longitude: number;
}

export type SubscriptionMessage = AircraftSubscriptionMessage | AreaSubscriptionMessage;

export class FixFeed {
	private static instance: FixFeed | null = null;
	private websocket: WebSocket | null = null;
	private websocketUrl = '';
	private subscribers = new Set<FixFeedSubscriber>();
	private subscribedAircraft = new Set<string>();
	private reconnectAttempts = 0;
	private readonly reconnectDelay = 5000; // Fixed 5 second delay
	private operationsPageActive = false;
	private aircraftRegistry: AircraftRegistry;

	private constructor() {
		this.aircraftRegistry = AircraftRegistry.getInstance();
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

		if (!dev) {
			// Production mode - use current host
			const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
			this.websocketUrl = `${protocol}//${window.location.host}/data/fixes/live`;
		} else {
			// Development mode - check backend mode setting
			const mode = get(backendMode);
			if (mode === 'dev') {
				// Dev mode with local backend
				this.websocketUrl = 'ws://localhost:1337/data/fixes/live';
			} else {
				// Dev mode using production backend
				this.websocketUrl = 'wss://glider.flights/data/fixes/live';
			}
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

				// Re-subscribe to all previously subscribed aircraft
				setTimeout(() => {
					if (this.websocket?.readyState === WebSocket.OPEN) {
						this.subscribedAircraft.forEach((aircraftId) => {
							this.sendSubscriptionMessage('subscribe', aircraftId);
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
							aircraft_id: rawMessage.aircraft_id,
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

						// Add fix to aircraft registry
						// For fixes from WebSocket, assume aircraft data is provided via aircraft messages
						// so don't attempt API fallback to avoid N+1 calls
						this.aircraftRegistry.addFixToAircraft(fix, false).catch((error) => {
							console.warn('Failed to add fix to aircraft registry:', error);
						});

						// Notify subscribers
						this.notifySubscribers({
							type: 'fix_received',
							fix
						});
					} else if (rawMessage.type === 'aircraft') {
						// Handle Aircraft message
						const aircraft: Aircraft = rawMessage;

						// Add all recent fixes to aircraft registry
						const aircraftFixes = aircraft.fixes || [];
						if (aircraftFixes.length > 0) {
							for (const fix of aircraftFixes) {
								this.aircraftRegistry.addFixToAircraft(fix, false).catch((error) => {
									console.warn('Failed to add aircraft fix to registry:', error);
								});
							}
						}

						// Update aircraft registry with complete aircraft info
						this.aircraftRegistry.updateAircraftFromAircraftData(aircraft).catch((error) => {
							console.warn('Failed to update aircraft info in registry:', error);
						});

						// Notify subscribers
						this.notifySubscribers({
							type: 'aircraft_received',
							aircraft: aircraft
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

	// Subscribe to a specific aircraft
	public async subscribeToAircraft(aircraftId: string): Promise<void> {
		if (this.subscribedAircraft.has(aircraftId)) {
			return; // Already subscribed
		}

		// Connect if not already connected
		if (!this.websocket || this.websocket.readyState === WebSocket.CLOSED) {
			this.connect();
		}

		// Wait for connection to be established
		const isConnected = await this.waitForConnection();

		if (isConnected && this.websocket?.readyState === WebSocket.OPEN) {
			this.sendSubscriptionMessage('subscribe', aircraftId);
			this.subscribedAircraft.add(aircraftId);

			this.notifySubscribers({
				type: 'subscription_added',
				aircraftId
			});

			// Fetch aircraft info and recent fixes from API
			await this.aircraftRegistry.updateAircraftFromAPI(aircraftId);
			await this.aircraftRegistry.loadRecentFixesFromAPI(aircraftId);
		} else {
			console.error(`Failed to subscribe to aircraft ${aircraftId}: WebSocket not connected`);
		}
	}

	// Unsubscribe from a specific aircraft
	public unsubscribeFromAircraft(aircraftId: string): void {
		if (!this.subscribedAircraft.has(aircraftId)) {
			return; // Not subscribed
		}

		if (this.websocket?.readyState === WebSocket.OPEN) {
			this.sendSubscriptionMessage('unsubscribe', aircraftId);
		}

		this.subscribedAircraft.delete(aircraftId);

		this.notifySubscribers({
			type: 'subscription_removed',
			aircraftId
		});

		// Disconnect if no subscriptions and operations page not active
		if (this.subscribedAircraft.size === 0 && !this.operationsPageActive) {
			this.disconnect();
		}
	}

	// Send subscription message to server
	private sendSubscriptionMessage(action: string, aircraftId: string): void {
		if (this.websocket?.readyState === WebSocket.OPEN) {
			const message: AircraftSubscriptionMessage = {
				action,
				type: 'aircraft',
				id: aircraftId
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
		if (this.subscribedAircraft.size === 0) {
			this.disconnect();
		}
	}

	// Get current connection status
	public getConnectionStatus(): {
		connected: boolean;
		reconnecting: boolean;
		subscribedAircraft: string[];
		operationsPageActive: boolean;
	} {
		return {
			connected: this.websocket?.readyState === WebSocket.OPEN,
			reconnecting: this.reconnectAttempts > 0,
			subscribedAircraft: Array.from(this.subscribedAircraft),
			operationsPageActive: this.operationsPageActive
		};
	}

	// Subscribe to multiple aircraft from watchlist
	public async subscribeToWatchlist(deviceIds: string[]): Promise<void> {
		// Unsubscribe from aircraft no longer in the list
		const devicesToUnsubscribe = Array.from(this.subscribedAircraft).filter(
			(aircraftId) => !deviceIds.includes(aircraftId)
		);

		for (const aircraftId of devicesToUnsubscribe) {
			this.unsubscribeFromAircraft(aircraftId);
		}

		// Subscribe to new aircraft
		const devicesToSubscribe = deviceIds.filter(
			(aircraftId) => !this.subscribedAircraft.has(aircraftId)
		);

		for (const aircraftId of devicesToSubscribe) {
			await this.subscribeToAircraft(aircraftId);
		}
	}

	// Fetch aircraft in bounding box via REST API
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
			const response = await serverCall('/aircraft', {
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
			console.error('Failed to fetch aircraft in bounding box:', error);
			return [];
		}
	}
}
