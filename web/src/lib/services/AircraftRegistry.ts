import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { Aircraft, Fix, DataListResponse, DataResponse } from '$lib/types';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'AircraftRegistry']);

// Initialize dayjs plugins
dayjs.extend(utc);

// Event types for subscribers
export type AircraftRegistryEvent =
	| { type: 'aircraft_updated'; aircraft: Aircraft }
	| { type: 'fix_added'; aircraft: Aircraft; fix: Fix }
	| { type: 'aircraft_changed'; aircraft: Aircraft[] };

export type AircraftRegistrySubscriber = (event: AircraftRegistryEvent) => void;

// Internal type to store aircraft with its fixes and cache metadata
interface AircraftWithFixesCache {
	aircraft: Aircraft;
	fixes: Fix[];
	cached_at: number; // Timestamp when this aircraft was last fetched/updated
}

export class AircraftRegistry {
	private static instance: AircraftRegistry | null = null;
	private aircraft = new Map<string, AircraftWithFixesCache>();
	private subscribers = new Set<AircraftRegistrySubscriber>();
	private readonly maxFixAge = 24 * 60 * 60 * 1000; // 24 hours in milliseconds
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

		// Optimize: For devices_changed events, we only need the most recent one
		// since it contains the complete list of all aircraft
		const hasDevicesChanged = events.some((e) => e.type === 'aircraft_changed');

		// Notify subscribers of all events except devices_changed
		const nonDevicesChangedEvents = events.filter((e) => e.type !== 'aircraft_changed');

		this.subscribers.forEach((subscriber) => {
			try {
				// Send all non-devices_changed events
				for (const event of nonDevicesChangedEvents) {
					subscriber(event);
				}

				// Send only one devices_changed event with current aircraft list
				if (hasDevicesChanged) {
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
			// Return aircraft with current fixes
			return { ...cached.aircraft, fixes: cached.fixes };
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

		// Get existing fixes if any, or use the ones from the aircraft, or empty array
		let existingFixes = this.aircraft.get(aircraft.id)?.fixes || aircraft.fixes || [];

		// If this is a new aircraft with a currentFix but no fixes array, use currentFix as the initial fix
		if (
			!this.aircraft.has(aircraft.id) &&
			aircraft.currentFix &&
			(!aircraft.fixes || aircraft.fixes.length === 0)
		) {
			try {
				// currentFix is already a Fix object (or should be)
				const fix = aircraft.currentFix as Fix;
				existingFixes = [fix];
				logger.debug('Initialized aircraft with currentFix: {aircraftId} {fixTimestamp}', {
					aircraftId: aircraft.id,
					fixTimestamp: fix.timestamp
				});
			} catch (error) {
				logger.warn('Failed to parse currentFix: {error}', { error });
			}
		}

		// Strip out currentFix after we've used it - fixes array is the source of truth
		const aircraftToStore = { ...aircraft, currentFix: null };

		const cached_at = Date.now();

		this.aircraft.set(aircraft.id, { aircraft: aircraftToStore, fixes: existingFixes, cached_at });

		// Notify subscribers
		this.notifySubscribers({
			type: 'aircraft_updated',
			aircraft: { ...aircraft, fixes: existingFixes }
		});

		this.notifySubscribers({
			type: 'aircraft_changed',
			aircraft: this.getAllAircraft()
		});
	}

	// Helper method to clean up old fixes
	private cleanupOldFixes(fixes: Fix[]): Fix[] {
		const cutoffTime = Date.now() - this.maxFixAge;
		return fixes.filter((fix) => {
			const fixTime = new Date(fix.timestamp).getTime();
			return fixTime > cutoffTime;
		});
	}

	// Add a new fix to aircraft's fixes array
	private addFixToAircraftCache(aircraftId: string, fix: Fix): void {
		const cached = this.aircraft.get(aircraftId);
		if (!cached) return;

		// Add fix to the beginning (most recent first)
		cached.fixes.unshift(fix);

		// Clean up old fixes (by age)
		cached.fixes = this.cleanupOldFixes(cached.fixes);

		// Limit to most recent 100 fixes
		if (cached.fixes.length > 100) {
			cached.fixes = cached.fixes.slice(0, 100);
		}

		// Update the map
		this.aircraft.set(aircraftId, cached);
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
			// The aircraft is already in the correct format
			this.setAircraft(aircraft);

			// Don't automatically load fixes - they will come from WebSocket
			// This reduces initial page load time and database load

			return this.getAircraft(aircraft.id);
		} catch (error) {
			logger.warn('Failed to update aircraft from aircraft data: {error}', { error });
			return null;
		}
	}

	// Add a fix to the appropriate aircraft
	// If the aircraft isn't in cache, fetches it from the backend API
	public async addFixToAircraft(fix: Fix): Promise<Aircraft | null> {
		logger.debug('Adding fix to aircraft: {aircraftId} {timestamp} {lat} {lng}', {
			aircraftId: fix.aircraftId,
			timestamp: fix.timestamp,
			lat: fix.latitude,
			lng: fix.longitude
		});

		const aircraftId = fix.aircraftId;
		if (!aircraftId) {
			logger.warn('No aircraftId in fix, cannot add');
			return null;
		}

		let aircraft = this.getAircraft(aircraftId);
		if (!aircraft) {
			logger.debug('Aircraft not found in cache for fix: {aircraftId}, fetching from API', {
				aircraftId
			});

			// Fetch aircraft from API - the backend is the source of truth
			try {
				aircraft = await this.updateAircraftFromAPI(aircraftId);
			} catch (error) {
				logger.warn('Failed to fetch aircraft from API for: {aircraftId} {error}', {
					aircraftId,
					error
				});
			}

			// If we still don't have aircraft data, we can't show this fix
			// Don't create "minimal" aircraft - the backend should have all aircraft data
			if (!aircraft) {
				logger.warn('Cannot display fix - aircraft not found in backend: {aircraftId}', {
					aircraftId
				});
				return null;
			}
		} else {
			logger.debug('Using existing aircraft: {aircraftId} {registration} {existingFixCount}', {
				aircraftId,
				registration: aircraft.registration,
				existingFixCount: aircraft.fixes?.length || 0
			});
		}

		// Add the fix using the helper method
		this.addFixToAircraftCache(aircraftId, fix);

		// Get the updated aircraft
		aircraft = this.getAircraft(aircraftId)!;

		// Always persist aircraft (setAircraft will normalize registration to "Unknown" if needed)
		this.setAircraft(aircraft);

		logger.debug('Fix added to aircraft. New fix count: {count}', {
			count: aircraft.fixes?.length || 0
		});

		// Notify subscribers about the fix
		this.notifySubscribers({
			type: 'fix_added',
			aircraft,
			fix
		});

		logger.debug('Notified subscribers about fix_added');

		return aircraft;
	}

	// Get all aircraft
	public getAllAircraft(): Aircraft[] {
		return Array.from(this.aircraft.values()).map((cached) => ({
			...cached.aircraft,
			fixes: cached.fixes
		}));
	}

	// Get all aircraft with recent fixes (within last hour)
	public getActiveAircraft(withinHours: number = 1): Aircraft[] {
		const cutoffTime = Date.now() - withinHours * 60 * 60 * 1000;

		return this.getAllAircraft().filter((aircraft) => {
			const fixes = aircraft.fixes || [];
			if (fixes.length === 0) return false;
			const latestFix = fixes[0]; // Most recent is first
			return new Date(latestFix.timestamp).getTime() > cutoffTime;
		});
	}

	// Get fixes for a specific aircraft
	public getFixesForAircraft(aircraftId: string): Fix[] {
		const cached = this.aircraft.get(aircraftId);
		return cached ? cached.fixes : [];
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
	private isAircraftStale(cached: AircraftWithFixesCache): boolean {
		const now = Date.now();
		const age = now - cached.cached_at;
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

	// Batch load recent fixes for an aircraft from API
	// Fetches fixes from the last 8 hours by default
	public async loadRecentFixesFromAPI(aircraftId: string, hoursBack: number = 8): Promise<Fix[]> {
		// Validate aircraftId is a non-empty string
		if (!aircraftId || typeof aircraftId !== 'string') {
			logger.warn('Invalid aircraftId provided to loadRecentFixesFromAPI: {aircraftId}', {
				aircraftId
			});
			return [];
		}

		try {
			// Calculate timestamp for N hours ago in ISO 8601 UTC format
			const after = dayjs().utc().subtract(hoursBack, 'hours').toISOString();

			const response = await serverCall<DataListResponse<Fix>>(`/aircraft/${aircraftId}/fixes`, {
				params: { after }
			});
			if (response.data) {
				// Add fixes to aircraft
				for (const fix of response.data) {
					await this.addFixToAircraft(fix);
				}
				return response.data;
			}
			return [];
		} catch (error) {
			logger.warn('Failed to load recent fixes for aircraft {aircraftId}: {error}', {
				aircraftId,
				error
			});
			return [];
		}
	}
}
