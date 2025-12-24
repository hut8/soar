import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { Aircraft, Fix } from '$lib/types';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';

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
	private readonly storageKeyPrefix = 'aircraft.';
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
				console.error('Error in AircraftRegistry subscriber:', error);
			}
		});
	}

	// Get an aircraft by ID, loading from localStorage if needed
	public getAircraft(aircraftId: string): Aircraft | null {
		// Check in-memory cache first
		if (this.aircraft.has(aircraftId)) {
			const cached = this.aircraft.get(aircraftId)!;
			// Return aircraft with current fixes
			return { ...cached.aircraft, fixes: cached.fixes };
		}

		// Try to load from localStorage
		const stored = this.loadAircraftFromStorage(aircraftId);
		if (stored) {
			this.aircraft.set(aircraftId, stored);
			return { ...stored.aircraft, fixes: stored.fixes };
		}

		return null;
	}

	// Get an aircraft by ID with automatic API fallback
	public async getAircraftWithFallback(aircraftId: string): Promise<Aircraft | null> {
		// Try to get from cache first
		let aircraft = this.getAircraft(aircraftId);

		// If not found in cache, fetch from API
		if (!aircraft) {
			console.log(`[REGISTRY] Aircraft ${aircraftId} not found in cache, fetching from API`);
			aircraft = await this.updateAircraftFromAPI(aircraftId);
		}

		return aircraft;
	}

	// Add or update an aircraft
	public setAircraft(aircraft: Aircraft): void {
		// Get existing fixes if any, or use the ones from the aircraft, or empty array
		const existingFixes = this.aircraft.get(aircraft.id)?.fixes || aircraft.fixes || [];
		const cached_at = Date.now();

		this.aircraft.set(aircraft.id, { aircraft, fixes: existingFixes, cached_at });
		this.saveAircraftToStorage({ aircraft, fixes: existingFixes, cached_at });

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

		// Clean up old fixes
		cached.fixes = this.cleanupOldFixes(cached.fixes);

		// Update the map
		this.aircraft.set(aircraftId, cached);
	}

	// Create or update aircraft from backend API data
	public async updateAircraftFromAPI(aircraftId: string): Promise<Aircraft | null> {
		try {
			const apiAircraft = await serverCall<Aircraft>(`/aircraft/${aircraftId}`);
			if (!apiAircraft) return null;

			this.setAircraft(apiAircraft);
			return this.getAircraft(aircraftId);
		} catch (error) {
			console.warn(`Failed to fetch aircraft ${aircraftId} from API:`, error);
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
			console.warn(`Failed to update aircraft from aircraft data:`, error);
			return null;
		}
	}

	// Add a fix to the appropriate aircraft
	public async addFixToAircraft(
		fix: Fix,
		allowApiFallback: boolean = true
	): Promise<Aircraft | null> {
		console.log('[REGISTRY] Adding fix to aircraft:', {
			aircraftId: fix.aircraft_id,
			deviceAddressHex: fix.device_address_hex,
			timestamp: fix.timestamp,
			position: { lat: fix.latitude, lng: fix.longitude }
		});

		const aircraftId = fix.aircraft_id;
		if (!aircraftId) {
			console.warn('[REGISTRY] No aircraft_id in fix, cannot add');
			return null;
		}

		let aircraft = this.getAircraft(aircraftId);
		if (!aircraft) {
			console.log(
				'[REGISTRY] Aircraft not found in cache for fix:',
				aircraftId,
				allowApiFallback ? 'attempting to fetch from API' : 'will check if API fetch needed'
			);

			// Check if the fix has a registration number
			const hasRegistration = fix.registration && fix.registration.trim() !== '';

			// Always try to fetch from API if:
			// 1. allowApiFallback is true, OR
			// 2. The fix doesn't have a registration (try to get complete data from backend)
			if (allowApiFallback || !hasRegistration) {
				try {
					console.log(
						`[REGISTRY] Fetching aircraft from API (allowApiFallback: ${allowApiFallback}, hasRegistration: ${hasRegistration})`
					);
					aircraft = await this.updateAircraftFromAPI(aircraftId);
				} catch (error) {
					console.warn('[REGISTRY] Failed to fetch aircraft from API for:', aircraftId, error);
				}
			}

			// If still no aircraft, create a minimal one
			if (!aircraft) {
				console.log('[REGISTRY] Creating minimal aircraft for fix:', aircraftId);
				aircraft = {
					id: aircraftId,
					device_address: fix.device_address_hex || '',
					address_type: '',
					address: fix.device_address_hex || '',
					aircraft_model: fix.model || '',
					registration: fix.registration || '',
					competition_number: '',
					tracked: false,
					identified: false,
					club_id: null,
					created_at: new Date().toISOString(),
					updated_at: new Date().toISOString(),
					from_ddb: false,
					fixes: []
				};
			}
		} else {
			console.log('[REGISTRY] Using existing aircraft:', {
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

		console.log('[REGISTRY] Fix added to aircraft. New fix count:', aircraft.fixes?.length || 0);

		// Notify subscribers about the fix
		this.notifySubscribers({
			type: 'fix_added',
			aircraft,
			fix
		});

		console.log('[REGISTRY] Notified subscribers about fix_added');

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

		// Clear localStorage
		if (browser) {
			const keys = [];
			for (let i = 0; i < localStorage.length; i++) {
				const key = localStorage.key(i);
				if (key && key.startsWith(this.storageKeyPrefix)) {
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

	// Private methods for localStorage management
	private saveAircraftToStorage(cached: AircraftWithFixesCache): void {
		if (!browser) return;

		try {
			const key = this.storageKeyPrefix + cached.aircraft.id;
			// Strip fixes entirely before saving to localStorage - they should never be cached
			const cacheWithoutFixes = {
				aircraft: cached.aircraft,
				cached_at: cached.cached_at
			};
			localStorage.setItem(key, JSON.stringify(cacheWithoutFixes));
		} catch (error) {
			console.warn('Failed to save aircraft to localStorage:', error);
		}
	}

	private loadAircraftFromStorage(aircraftId: string): AircraftWithFixesCache | null {
		if (!browser) return null;

		const key = this.storageKeyPrefix + aircraftId;
		const stored = localStorage.getItem(key);

		if (stored) {
			try {
				const data = JSON.parse(stored) as AircraftWithFixesCache;
				// Handle backward compatibility: if cached_at is missing, set it to 0 (will be refreshed)
				if (!data.cached_at) {
					data.cached_at = 0;
				}
				return data;
			} catch (e) {
				console.warn(`Failed to parse stored aircraft ${aircraftId}:`, e);
				// Remove corrupted data
				localStorage.removeItem(key);
			}
		}

		return null;
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
			console.log('[REGISTRY] No stale aircraft to refresh');
			return;
		}

		console.log(`[REGISTRY] Refreshing ${staleAircraftIds.length} stale aircraft`);

		// Refresh aircraft in parallel with rate limiting (max 5 at a time)
		const batchSize = 5;
		for (let i = 0; i < staleAircraftIds.length; i += batchSize) {
			const batch = staleAircraftIds.slice(i, i + batchSize);
			await Promise.allSettled(batch.map((aircraftId) => this.updateAircraftFromAPI(aircraftId)));
		}

		console.log('[REGISTRY] Finished refreshing stale aircraft');
	}

	// Start periodic refresh of stale aircraft
	private startPeriodicRefresh(): void {
		// Initial refresh on load
		this.loadAllAircraftFromStorage();
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

	// Load all aircraft from localStorage into memory
	private loadAllAircraftFromStorage(): void {
		if (!browser) return;

		console.log('[REGISTRY] Loading aircraft from localStorage');
		let count = 0;

		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			if (key && key.startsWith(this.storageKeyPrefix)) {
				const aircraftId = key.substring(this.storageKeyPrefix.length);
				const cached = this.loadAircraftFromStorage(aircraftId);
				if (cached) {
					this.aircraft.set(aircraftId, cached);
					count++;
				}
			}
		}

		console.log(`[REGISTRY] Loaded ${count} aircraft from localStorage`);
	}

	// Batch load recent fixes for an aircraft from API
	// Fetches fixes from the last 8 hours by default
	public async loadRecentFixesFromAPI(aircraftId: string, hoursBack: number = 8): Promise<Fix[]> {
		try {
			// Calculate timestamp for N hours ago in ISO 8601 UTC format
			const after = dayjs().utc().subtract(hoursBack, 'hours').toISOString();

			const response = await serverCall<{ fixes: Fix[] }>(`/aircraft/${aircraftId}/fixes`, {
				params: { after, per_page: 1000 }
			});
			if (response.fixes) {
				// Add fixes to aircraft
				for (const fix of response.fixes) {
					await this.addFixToAircraft(fix, false);
				}
				return response.fixes;
			}
			return [];
		} catch (error) {
			console.warn(`Failed to load recent fixes for aircraft ${aircraftId}:`, error);
			return [];
		}
	}
}
