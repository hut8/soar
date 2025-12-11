import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import type { Aircraft, WatchlistEntry, Fix } from '$lib/types';
import { AircraftRegistry } from '$lib/services/AircraftRegistry';
import { FixFeed } from '$lib/services/FixFeed';

// Store interfaces
export interface WatchlistStore {
	entries: WatchlistEntry[];
}

export interface AircraftRegistryStore {
	registry: AircraftRegistry;
	lastUpdate: number; // Timestamp of last update for reactivity
}

// Initial states
const initialWatchlistState: WatchlistStore = {
	entries: []
};

const initialAircraftRegistryState: AircraftRegistryStore = {
	registry: AircraftRegistry.getInstance(),
	lastUpdate: Date.now()
};

// FixFeed instance for managing websocket connections
const fixFeed = FixFeed.getInstance();

// WebSocket status store - delegates to FixFeed
export const websocketStatus = writable<{
	connected: boolean;
	reconnecting: boolean;
	error: string | null;
}>({
	connected: false,
	reconnecting: false,
	error: null
});

// Debug status store to help diagnose subscription issues
export const debugStatus = writable<{
	subscribedAircraft: string[];
	operationsPageActive: boolean;
	activeWatchlistEntries: string[];
	activeAreaSubscriptions: number;
}>({
	subscribedAircraft: [],
	operationsPageActive: false,
	activeWatchlistEntries: [],
	activeAreaSubscriptions: 0
});

// Subscribe to FixFeed events to update websocket status
if (browser) {
	fixFeed.subscribe((event) => {
		switch (event.type) {
			case 'connection_opened':
				websocketStatus.set({ connected: true, reconnecting: false, error: null });
				updateDebugStatus();
				break;
			case 'connection_closed':
				websocketStatus.set({
					connected: false,
					reconnecting: false,
					error: event.code !== 1000 ? `Connection lost (${event.code})` : null
				});
				updateDebugStatus();
				break;
			case 'connection_error':
				websocketStatus.update((status) => ({ ...status, error: 'Connection failed' }));
				updateDebugStatus();
				break;
			case 'reconnecting':
				websocketStatus.update((status) => ({
					...status,
					reconnecting: true,
					error: `Reconnecting... (${event.attempt})`
				}));
				updateDebugStatus();
				break;
			case 'subscription_added':
			case 'subscription_removed':
				updateDebugStatus();
				break;
		}
	});
}

// Create watchlist store
function createWatchlistStore() {
	const { subscribe, set, update } = writable<WatchlistStore>(initialWatchlistState);

	return {
		subscribe,

		// Add aircraft to watchlist
		add: (aircraftId: string) => {
			update((state) => {
				// Check if aircraft is already in watchlist
				const existingEntry = state.entries.find((entry) => entry.aircraftId === aircraftId);
				if (existingEntry) {
					console.log('Aircraft already in watchlist:', aircraftId);
					return state;
				}

				const newEntry: WatchlistEntry = {
					id: Date.now().toString(),
					aircraftId,
					active: true
				};
				const newEntries = [...state.entries, newEntry];
				saveWatchlistToStorage(newEntries);
				notifyWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Remove aircraft from watchlist
		remove: (id: string) => {
			update((state) => {
				const newEntries = state.entries.filter((entry) => entry.id !== id);
				saveWatchlistToStorage(newEntries);
				notifyWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Toggle aircraft active state
		toggleActive: (id: string) => {
			update((state) => {
				const newEntries = state.entries.map((entry) =>
					entry.id === id ? { ...entry, active: !entry.active } : entry
				);
				saveWatchlistToStorage(newEntries);
				notifyWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Load from localStorage
		loadFromStorage: () => {
			if (!browser) return;

			const saved = localStorage.getItem('watchlist');
			if (saved) {
				try {
					const rawEntries = JSON.parse(saved);
					const aircraftIdMap = new Map<string, WatchlistEntry>();

					// Handle both old format (with device objects) and new format (with aircraftId)
					for (const entry of rawEntries) {
						let normalizedEntry: WatchlistEntry | null = null;

						if (entry.aircraftId && entry.id) {
							// New format - just validate and use
							normalizedEntry = {
								id: entry.id,
								aircraftId: entry.aircraftId,
								active: entry.active
							};
						} else if (entry.device?.id && entry.id) {
							// Old format - convert to new format
							normalizedEntry = {
								id: entry.id,
								aircraftId: entry.device.id,
								active: entry.active
							};
						}

						// Deduplicate by aircraftId - keep the last entry for each aircraftId
						if (normalizedEntry && normalizedEntry.aircraftId) {
							aircraftIdMap.set(normalizedEntry.aircraftId, normalizedEntry);
						}
					}

					// Convert map back to array
					const deduplicatedEntries = Array.from(aircraftIdMap.values());

					// Check if deduplication was needed
					const originalCount = rawEntries.length;
					const finalCount = deduplicatedEntries.length;
					const hadDuplicates = originalCount > finalCount;

					if (hadDuplicates) {
						console.log(
							`Deduplicated watchlist: ${originalCount} → ${finalCount} entries (removed ${originalCount - finalCount} duplicates)`
						);
						// Save the deduplicated watchlist back to storage
						saveWatchlistToStorage(deduplicatedEntries);
					}

					console.log(`Loaded ${finalCount} valid watchlist entries from storage`);
					set({ entries: deduplicatedEntries });
					notifyWatchlistChange(deduplicatedEntries);
				} catch (e) {
					console.warn('Failed to load watchlist from localStorage:', e);
				}
			}
		},

		// Clear all entries
		clear: () => {
			const newEntries: WatchlistEntry[] = [];
			set({ entries: newEntries });
			saveWatchlistToStorage(newEntries);
			notifyWatchlistChange(newEntries);
		}
	};
}

// Create aircraft registry store
function createAircraftRegistryStore() {
	const { subscribe, set, update } = writable<AircraftRegistryStore>(initialAircraftRegistryState);

	return {
		subscribe,

		// Add fixes to appropriate devices
		addFixes: (newFixes: Fix[]) => {
			update((state) => {
				// Log each fix to console
				newFixes.forEach(async (fix) => {
					console.log('Received fix:', fix);
					// Add fix to the aircraft registry
					try {
						await state.registry.addFixToAircraft(fix);
					} catch (error) {
						console.warn('Failed to add fix to aircraft registry:', error);
					}
				});

				// Update timestamp to trigger reactivity
				return {
					registry: state.registry,
					lastUpdate: Date.now()
				};
			});
		},

		// Update device information from API
		updateDeviceFromAPI: async (aircraftId: string) => {
			update((state) => {
				state.registry.updateAircraftFromAPI(aircraftId);
				return {
					registry: state.registry,
					lastUpdate: Date.now()
				};
			});
		},

		// Get a specific aircraft
		getAircraft: (aircraftId: string): Aircraft | null => {
			let currentAircraft: Aircraft | null = null;
			subscribe((state) => {
				currentAircraft = state.registry.getAircraft(aircraftId);
			})();
			return currentAircraft;
		},

		// Get all aircraft with recent fixes
		getActiveAircraft: (): Aircraft[] => {
			let aircraft: Aircraft[] = [];
			subscribe((state) => {
				aircraft = state.registry.getAllAircraft().filter((ac: Aircraft) => {
					const fixes = ac.fixes || [];
					const latestFix = fixes.length > 0 ? fixes[0] : null;
					if (!latestFix) return false;
					// Consider active if last fix was within the last hour
					const oneHourAgo = Date.now() - 60 * 60 * 1000;
					return new Date(latestFix.timestamp).getTime() > oneHourAgo;
				});
			})();
			return aircraft;
		},

		// Get fixes for a specific aircraft
		getFixesForAircraft: (aircraftId: string): Fix[] => {
			let aircraftFixes: Fix[] = [];
			subscribe((state) => {
				const aircraft = state.registry.getAircraft(aircraftId);
				aircraftFixes = aircraft ? aircraft.fixes || [] : [];
			})();
			return aircraftFixes;
		},

		// Clear all aircraft
		clear: () => {
			AircraftRegistry.getInstance().clear();
			set({
				registry: AircraftRegistry.getInstance(),
				lastUpdate: Date.now()
			});
		}
	};
}

// Update debug status with current FixFeed state
function updateDebugStatus() {
	if (!browser) return;

	const connectionStatus = fixFeed.getConnectionStatus();
	debugStatus.update((current) => ({
		...current,
		subscribedAircraft: connectionStatus.subscribedAircraft,
		operationsPageActive: connectionStatus.operationsPageActive
	}));
}

// Notify FixFeed about watchlist changes
function notifyWatchlistChange(entries: WatchlistEntry[]) {
	if (!browser) return;

	const activeAircraftIds = entries
		.filter((entry) => entry.active && entry.aircraftId)
		.map((entry) => entry.aircraftId);

	// Update debug status with active watchlist entries
	debugStatus.update((current) => ({
		...current,
		activeWatchlistEntries: activeAircraftIds
	}));

	// Update FixFeed subscriptions based on watchlist
	fixFeed.subscribeToWatchlist(activeAircraftIds);

	// Update debug status after subscription change
	setTimeout(updateDebugStatus, 10); // Small delay to let subscription complete
}

// Save watchlist to localStorage
function saveWatchlistToStorage(entries: WatchlistEntry[]) {
	if (!browser) return;
	localStorage.setItem('watchlist', JSON.stringify(entries));
}

// Function to connect and listen for all live fixes (for operations page)
export function startLiveFixesFeed() {
	if (!browser) return;
	fixFeed.startLiveFixesFeed();
}

// Function to stop live fixes feed
export function stopLiveFixesFeed() {
	if (!browser) return;
	fixFeed.stopLiveFixesFeed();
}

// Create store instances
export const watchlist = createWatchlistStore();
export const aircraftRegistry = createAircraftRegistryStore();

// Backward compatibility - provide a way to get all recent fixes
export const fixes = {
	subscribe: aircraftRegistry.subscribe,
	addFixes: aircraftRegistry.addFixes,
	clear: aircraftRegistry.clear,
	// Get all recent fixes from all aircraft
	getAllRecentFixes: (): Fix[] => {
		const aircraft = aircraftRegistry.getActiveAircraft();
		const allFixes: Fix[] = [];
		aircraft.forEach((ac) => {
			allFixes.push(...(ac.fixes || []));
		});
		// Sort by timestamp, most recent first
		return allFixes.sort(
			(a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
		);
	}
};
