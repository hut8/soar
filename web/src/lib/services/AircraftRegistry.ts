import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { Aircraft, Fix, DataResponse } from '$lib/types';
import { getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'AircraftRegistry']);

// Event types for subscribers
export type AircraftRegistryEvent =
	| { type: 'aircraft_updated'; aircraft: Aircraft }
	| { type: 'fix_received'; aircraft: Aircraft; fix: Fix }
	| { type: 'aircraft_changed'; aircraft: Aircraft[] };

export type AircraftRegistrySubscriber = (event: AircraftRegistryEvent) => void;

// Internal type to store aircraft with cache metadata
interface AircraftCache {
	aircraft: Aircraft;
	cachedAt: number; // Timestamp when this aircraft was last fetched/updated
}

export class AircraftRegistry {
	private static instance: AircraftRegistry | null = null;
	private aircraft = new Map<string, AircraftCache>();
	private subscribers = new Set<AircraftRegistrySubscriber>();
	private readonly aircraftCacheExpiration = 24 * 60 * 60 * 1000; // 24 hours in milliseconds
	private refreshIntervalId: number | null = null;

	// Debouncing for notifySubscribers - max 1 update per second
	private notifyDebounceTimer: number | null = null;
	private pendingEvents: AircraftRegistryEvent[] = [];
	private readonly notifyDebounceMs = 1000; // 1 second

	private constructor() {
		// Private constructor for singleton pattern
		// Start periodic refresh check when registry is created
		if (browser) {
			this.startPeriodicRefresh();
		}
	}

	// Singleton instance getter
	public static getInstance(): AircraftRegistry {
		if (!AircraftRegistry.instance) {
			AircraftRegistry.instance = new AircraftRegistry();
		}
		return AircraftRegistry.instance;
	}

	// Subscription management
	public subscribe(subscriber: AircraftRegistrySubscriber): () => void {
		this.subscribers.add(subscriber);

		// Return unsubscribe function
		return () => {
			this.subscribers.delete(subscriber);
		};
	}

	private notifySubscribers(event: AircraftRegistryEvent): void {
		// Add event to pending queue
		this.pendingEvents.push(event);

		// If no timer is running, start one
		if (this.notifyDebounceTimer === null) {
			this.notifyDebounceTimer = window.setTimeout(() => {
				this.flushPendingNotifications();
			}, this.notifyDebounceMs);
		}
	}

	private flushPendingNotifications(): void {
		// Clear the timer
		this.notifyDebounceTimer = null;

		// Get all pending events
		const events = this.pendingEvents;
		this.pendingEvents = [];

		// If no events, nothing to do
		if (events.length === 0) {
			return;
		}

		// Optimize: For aircraft_changed events, we only need the most recent one
		// since it contains the complete list of all aircraft
		const hasAircraftChanged = events.some((e) => e.type === 'aircraft_changed');

		// Notify subscribers of all events except aircraft_changed
		const nonAircraftChangedEvents = events.filter((e) => e.type !== 'aircraft_changed');

		this.subscribers.forEach((subscriber) => {
			try {
				// Send all non-aircraft_changed events
				for (const event of nonAircraftChangedEvents) {
					subscriber(event);
				}

				// Send only one aircraft_changed event with current aircraft list
				if (hasAircraftChanged) {
					subscriber({
						type: 'aircraft_changed',
						aircraft: this.getAllAircraft()
					});
				}
			} catch (error) {
				logger.error('Error in AircraftRegistry subscriber: {error}', { error });
			}
		});
	}

	// Get an aircraft by ID from memory cache
	public getAircraft(aircraftId: string): Aircraft | null {
		// Check in-memory cache
		if (this.aircraft.has(aircraftId)) {
			const cached = this.aircraft.get(aircraftId)!;
			return cached.aircraft;
		}

		return null;
	}

	// Get an aircraft by ID with automatic API fallback
	public async getAircraftWithFallback(aircraftId: string): Promise<Aircraft | null> {
		// Try to get from cache first
		let aircraft = this.getAircraft(aircraftId);

		// If not found in cache, fetch from API
		if (!aircraft) {
			logger.debug('Aircraft {aircraftId} not found in cache, fetching from API', { aircraftId });
			aircraft = await this.updateAircraftFromAPI(aircraftId);
		}

		return aircraft;
	}

	// Add or update an aircraft
	public setAircraft(aircraft: Aircraft): void {
		// Validate that aircraft has a valid ID
		if (!aircraft.id) {
			logger.warn('Attempted to set aircraft with undefined ID, skipping: {aircraft}', {
				aircraft
			});
			return;
		}

		// Check if we have a newer currentFix in the existing cache
		const existing = this.aircraft.get(aircraft.id);
		let currentFix = aircraft.currentFix;

		if (existing?.aircraft.currentFix && currentFix) {
			const existingFix = existing.aircraft.currentFix as Fix;
			const newFix = currentFix as Fix;
			// Keep the newer fix
			if (new Date(existingFix.timestamp) > new Date(newFix.timestamp)) {
				currentFix = existing.aircraft.currentFix;
			}
		} else if (existing?.aircraft.currentFix && !currentFix) {
			// Keep existing fix if new aircraft doesn't have one
			currentFix = existing.aircraft.currentFix;
		}

		// Store aircraft with the best currentFix
		const aircraftToStore: Aircraft = { ...aircraft, currentFix };
		const cachedAt = Date.now();

		this.aircraft.set(aircraft.id, { aircraft: aircraftToStore, cachedAt });

		// Notify subscribers
		this.notifySubscribers({
			type: 'aircraft_updated',
			aircraft: aircraftToStore
		});

		this.notifySubscribers({
			type: 'aircraft_changed',
			aircraft: this.getAllAircraft()
		});
	}

	// Create or update aircraft from backend API data
	public async updateAircraftFromAPI(aircraftId: string): Promise<Aircraft | null> {
		// Validate aircraftId is a non-empty string
		if (!aircraftId || typeof aircraftId !== 'string') {
			logger.warn('Invalid aircraftId provided to updateAircraftFromAPI: {aircraftId}', {
				aircraftId
			});
			return null;
		}

		try {
			const response = await serverCall<DataResponse<Aircraft>>(`/aircraft/${aircraftId}`);
			if (!response || !response.data) return null;

			this.setAircraft(response.data);
			return this.getAircraft(aircraftId);
		} catch (error) {
			logger.warn('Failed to fetch aircraft from API: {aircraftId} {error}', {
				aircraftId,
				error
			});
			return null;
		}
	}

	// Update aircraft from Aircraft data (from WebSocket or bbox search)
	public async updateAircraftFromAircraftData(aircraft: Aircraft): Promise<Aircraft | null> {
		try {
			this.setAircraft(aircraft);
			return this.getAircraft(aircraft.id);
		} catch (error) {
			logger.warn('Failed to update aircraft from aircraft data: {error}', { error });
			return null;
		}
	}

	// Update the current fix for an aircraft
	// If the aircraft isn't in cache, fetches it from the backend API
	public async updateCurrentFix(fix: Fix): Promise<Aircraft | null> {
		logger.debug('Updating current fix for aircraft: {aircraftId} {timestamp} {lat} {lng}', {
			aircraftId: fix.aircraftId,
			timestamp: fix.timestamp,
			lat: fix.latitude,
			lng: fix.longitude
		});

		const aircraftId = fix.aircraftId;
		if (!aircraftId) {
			logger.warn('No aircraftId in fix, cannot update');
			return null;
		}

		let cached = this.aircraft.get(aircraftId);
		if (!cached) {
			logger.debug('Aircraft not found in cache for fix: {aircraftId}, fetching from API', {
				aircraftId
			});

			// Fetch aircraft from API - the backend is the source of truth
			try {
				const aircraft = await this.updateAircraftFromAPI(aircraftId);
				if (!aircraft) {
					logger.warn('Cannot display fix - aircraft not found in backend: {aircraftId}', {
						aircraftId
					});
					return null;
				}
				cached = this.aircraft.get(aircraftId)!;
			} catch (error) {
				logger.warn('Failed to fetch aircraft from API for: {aircraftId} {error}', {
					aircraftId,
					error
				});
				return null;
			}
		}

		// Update the current fix (only if newer than existing)
		const existingFix = cached.aircraft.currentFix as Fix | null;
		const existingTimestamp = existingFix ? new Date(existingFix.timestamp).getTime() : 0;
		const newTimestamp = new Date(fix.timestamp).getTime();

		if (newTimestamp >= existingTimestamp) {
			cached.aircraft = { ...cached.aircraft, currentFix: fix };
			this.aircraft.set(aircraftId, cached);
		}

		// Get the updated aircraft
		const aircraft = this.getAircraft(aircraftId)!;

		// Notify subscribers about the fix
		this.notifySubscribers({
			type: 'fix_received',
			aircraft,
			fix
		});

		logger.debug('Updated current fix for aircraft');

		return aircraft;
	}

	// Get all aircraft
	public getAllAircraft(): Aircraft[] {
		return Array.from(this.aircraft.values()).map((cached) => cached.aircraft);
	}

	// Get all aircraft with recent fixes (within specified hours)
	public getActiveAircraft(withinHours: number = 1): Aircraft[] {
		const cutoffTime = Date.now() - withinHours * 60 * 60 * 1000;

		return this.getAllAircraft().filter((aircraft) => {
			const currentFix = aircraft.currentFix as Fix | null;
			if (!currentFix) return false;
			return new Date(currentFix.timestamp).getTime() > cutoffTime;
		});
	}

	// Get the current fix for a specific aircraft
	public getCurrentFix(aircraftId: string): Fix | null {
		const cached = this.aircraft.get(aircraftId);
		return (cached?.aircraft.currentFix as Fix) ?? null;
	}

	// Clear all aircraft (for cleanup)
	public clear(): void {
		this.aircraft.clear();
		this.stopPeriodicRefresh();

		// Clear debounce timer
		if (this.notifyDebounceTimer !== null) {
			window.clearTimeout(this.notifyDebounceTimer);
			this.notifyDebounceTimer = null;
		}
		this.pendingEvents = [];

		// Clean up any old localStorage entries from previous versions
		if (browser) {
			const keys = [];
			for (let i = 0; i < localStorage.length; i++) {
				const key = localStorage.key(i);
				if (key && key.startsWith('aircraft.')) {
					keys.push(key);
				}
			}
			keys.forEach((key) => localStorage.removeItem(key));
		}

		this.notifySubscribers({
			type: 'aircraft_changed',
			aircraft: []
		});
	}

	// Check if an aircraft cache entry is stale
	private isAircraftStale(cached: AircraftCache): boolean {
		const now = Date.now();
		const age = now - cached.cachedAt;
		return age > this.aircraftCacheExpiration;
	}

	// Get all stale aircraft that need refreshing
	private getStaleAircraft(): string[] {
		const staleAircraftIds: string[] = [];

		for (const [aircraftId, cached] of this.aircraft.entries()) {
			if (this.isAircraftStale(cached)) {
				staleAircraftIds.push(aircraftId);
			}
		}

		return staleAircraftIds;
	}

	// Refresh all stale aircraft from the API
	public async refreshStaleAircraft(): Promise<void> {
		const staleAircraftIds = this.getStaleAircraft();

		if (staleAircraftIds.length === 0) {
			logger.debug('No stale aircraft to refresh');
			return;
		}

		logger.debug('Refreshing {count} stale aircraft', { count: staleAircraftIds.length });

		// Refresh aircraft in parallel with rate limiting (max 5 at a time)
		const batchSize = 5;
		for (let i = 0; i < staleAircraftIds.length; i += batchSize) {
			const batch = staleAircraftIds.slice(i, i + batchSize);
			await Promise.allSettled(batch.map((aircraftId) => this.updateAircraftFromAPI(aircraftId)));
		}

		logger.debug('Finished refreshing stale aircraft');
	}

	// Start periodic refresh of stale aircraft
	private startPeriodicRefresh(): void {
		// Initial refresh on load
		void this.refreshStaleAircraft();

		// Set up periodic refresh every hour
		this.refreshIntervalId = window.setInterval(
			() => {
				void this.refreshStaleAircraft();
			},
			60 * 60 * 1000
		); // Every hour
	}

	// Stop periodic refresh (for cleanup)
	public stopPeriodicRefresh(): void {
		if (this.refreshIntervalId !== null) {
			window.clearInterval(this.refreshIntervalId);
			this.refreshIntervalId = null;
		}
	}
}
