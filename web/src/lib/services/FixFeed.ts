import { browser, dev } from '$app/environment';
import type { Aircraft, FixWithExtras, AircraftSearchResponse } from '$lib/types';
import { AircraftRegistry } from './AircraftRegistry';
import { backendMode } from '$lib/stores/backend';
import { get } from 'svelte/store';
import { getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'FixFeed']);

// Connection sources status from backend
export interface ConnectionSources {
	ogn: boolean;
	adsb: boolean;
}

// Event types for subscribers
export type FixFeedEvent =
	| { type: 'connection_opened' }
	| { type: 'connection_closed'; code: number; reason: string }
	| { type: 'connection_error'; error: Event }
	| { type: 'fix_received'; fix: FixWithExtras }
	| { type: 'aircraft_received'; aircraft: Aircraft }
	| { type: 'connection_status'; status: ConnectionSources }
	| { type: 'subscription_added'; aircraftId: string }
	| { type: 'subscription_removed'; aircraftId: string }
	| { type: 'reconnecting'; attempt: number };

export type FixFeedSubscriber = (event: FixFeedEvent) => void;

export interface GeoBounds {
	north: number;
	south: number;
	east: number;
	west: number;
}

export interface AircraftSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: 'aircraft';
	id: string;
}

export interface BulkAreaSubscriptionMessage {
	action: string; // "subscribe" or "unsubscribe"
	type: 'area_bulk';
	bounds: GeoBounds;
}

export type SubscriptionMessage = AircraftSubscriptionMessage | BulkAreaSubscriptionMessage;

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
				logger.error('Error in FixFeed subscriber: {error}', { error });
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
				// Dev mode using staging backend
				this.websocketUrl = 'wss://staging.glider.flights/data/fixes/live';
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
			logger.debug('WebSocket already connected or connecting, skipping connection attempt');
			return;
		}

		try {
			this.websocket = new WebSocket(this.websocketUrl);

			this.websocket.onopen = () => {
				logger.info('WebSocket connected to live fixes feed');
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
					logger.debug('Received WebSocket message: {message}', { message: rawMessage });

					// Handle different message types based on the "type" field
					if (rawMessage.type === 'fix') {
						// Transform WebSocket fix data to match FixWithExtras interface
						const fix: FixWithExtras = {
							id: rawMessage.id,
							aircraftId: rawMessage.aircraftId,
							source: rawMessage.source || '',
							timestamp: rawMessage.timestamp,
							latitude: rawMessage.latitude,
							longitude: rawMessage.longitude,
							altitudeMslFeet: rawMessage.altitudeMslFeet,
							altitudeAglFeet: rawMessage.altitudeAglFeet,
							flightNumber: rawMessage.flightNumber || null,
							squawk: rawMessage.squawk || null,
							trackDegrees: rawMessage.trackDegrees,
							groundSpeedKnots: rawMessage.groundSpeedKnots,
							climbFpm: rawMessage.climbFpm,
							turnRateRot: rawMessage.turnRateRot || null,
							sourceMetadata: rawMessage.sourceMetadata || null,
							flightId: rawMessage.flightId,
							receivedAt: rawMessage.receivedAt || new Date().toISOString(),
							active: rawMessage.active,
							receiverId: rawMessage.receiverId || '',
							rawMessageId: rawMessage.rawMessageId || '',
							altitudeAglValid: rawMessage.altitudeAglValid ?? false,
							timeGapSeconds: rawMessage.timeGapSeconds || null
						};

						// Update aircraft's current fix in the registry
						this.aircraftRegistry.updateCurrentFix(fix).catch((error) => {
							logger.warn('Failed to update aircraft current fix: {error}', { error });
						});

						// Notify subscribers
						this.notifySubscribers({
							type: 'fix_received',
							fix
						});
					} else if (rawMessage.type === 'aircraft') {
						// Handle Aircraft message
						const aircraft: Aircraft = rawMessage;

						// Register/update the aircraft in the registry
						this.aircraftRegistry.updateAircraftFromAircraftData(aircraft).catch((error) => {
							logger.warn('Failed to update aircraft info in registry: {error}', { error });
						});

						// Notify subscribers
						this.notifySubscribers({
							type: 'aircraft_received',
							aircraft: aircraft
						});
					} else if (rawMessage.type === 'connection_status') {
						// Handle connection status update from backend
						this.notifySubscribers({
							type: 'connection_status',
							status: {
								ogn: rawMessage.ogn ?? false,
								adsb: rawMessage.adsb ?? false
							}
						});
					} else {
						logger.warn('Unknown WebSocket message type: {type}', { type: rawMessage.type });
					}
				} catch (e) {
					logger.warn('Failed to parse WebSocket message: {error}', { error: e });
				}
			};

			this.websocket.onclose = (event) => {
				logger.info('WebSocket disconnected: {code} {reason}', {
					code: event.code,
					reason: event.reason
				});
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
				logger.error('WebSocket error: {error}', { error });
				this.notifySubscribers({
					type: 'connection_error',
					error
				});
			};
		} catch (e) {
			logger.error('Failed to create WebSocket connection: {error}', { error: e });
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

		logger.info('Attempting to reconnect in {delay}ms (attempt {attempt})', {
			delay: this.reconnectDelay,
			attempt: this.reconnectAttempts
		});

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
		// Validate aircraftId is a non-empty string
		if (!aircraftId || typeof aircraftId !== 'string') {
			logger.warn('Invalid aircraftId provided to subscribeToAircraft: {aircraftId}', {
				aircraftId
			});
			return;
		}

		if (this.subscribedAircraft.has(aircraftId)) {
			return; // Already subscribed
		}

		// Add to set immediately to prevent duplicate subscriptions from concurrent calls
		this.subscribedAircraft.add(aircraftId);

		// Connect if not already connected
		if (!this.websocket || this.websocket.readyState === WebSocket.CLOSED) {
			this.connect();
		}

		// Wait for connection to be established
		const isConnected = await this.waitForConnection();

		if (isConnected && this.websocket?.readyState === WebSocket.OPEN) {
			this.sendSubscriptionMessage('subscribe', aircraftId);

			this.notifySubscribers({
				type: 'subscription_added',
				aircraftId
			});

			// Fetch aircraft info from API (currentFix will be included)
			await this.aircraftRegistry.updateAircraftFromAPI(aircraftId);
		} else {
			// Remove from set if subscription failed
			this.subscribedAircraft.delete(aircraftId);
			logger.error('Failed to subscribe to aircraft {aircraftId}: WebSocket not connected', {
				aircraftId
			});
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

		logger.info('Starting live fixes feed for operations page');
		this.operationsPageActive = true;
		this.connect();
	}

	// Stop live fixes feed for operations page
	public stopLiveFixesFeed(): void {
		if (!browser) return;

		logger.info('Stopping live fixes feed for operations page');
		this.operationsPageActive = false;

		// Only disconnect if there are no active aircraft subscriptions
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
	public async subscribeToWatchlist(aircraftIds: string[]): Promise<void> {
		// Unsubscribe from aircraft no longer in the list
		const aircraftToUnsubscribe = Array.from(this.subscribedAircraft).filter(
			(aircraftId) => !aircraftIds.includes(aircraftId)
		);

		for (const aircraftId of aircraftToUnsubscribe) {
			this.unsubscribeFromAircraft(aircraftId);
		}

		// Subscribe to new aircraft
		const aircraftToSubscribe = aircraftIds.filter(
			(aircraftId) => !this.subscribedAircraft.has(aircraftId)
		);

		for (const aircraftId of aircraftToSubscribe) {
			await this.subscribeToAircraft(aircraftId);
		}
	}

	// Fetch aircraft in bounding box via REST API
	public async fetchAircraftInBoundingBox(
		south: number,
		north: number,
		west: number,
		east: number,
		afterTimestamp?: string, // Expected in ISO 8601 format
		limit?: number
	): Promise<AircraftSearchResponse> {
		if (!browser) return { items: [], total: 0n, clustered: false };

		try {
			const { serverCall } = await import('$lib/api/server');
			const response = await serverCall<AircraftSearchResponse>('/aircraft', {
				params: {
					south,
					north,
					west,
					east,
					...(afterTimestamp && { after: afterTimestamp }),
					...(limit && { limit })
				}
			});
			return response;
		} catch (error) {
			logger.error('Failed to fetch aircraft in bounding box: {error}', { error });
			return { items: [], total: 0n, clustered: false };
		}
	}
}
