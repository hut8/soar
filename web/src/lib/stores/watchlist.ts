import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { Device } from '$lib/types';
import type { WatchlistEntry, Fix } from '$lib/types';
import { DeviceRegistry } from '$lib/services/DeviceRegistry';
import { FixFeed } from '$lib/services/FixFeed';

// Store interfaces
export interface WatchlistStore {
	entries: WatchlistEntry[];
}

export interface DeviceRegistryStore {
	registry: DeviceRegistry;
	lastUpdate: number; // Timestamp of last update for reactivity
}

// Initial states
const initialWatchlistState: WatchlistStore = {
	entries: []
};

const initialDeviceRegistryState: DeviceRegistryStore = {
	registry: DeviceRegistry.getInstance(),
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
	subscribedDevices: string[];
	operationsPageActive: boolean;
	activeWatchlistEntries: string[];
	activeAreaSubscriptions: number;
}>({
	subscribedDevices: [],
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

		// Add device to watchlist
		add: (deviceId: string) => {
			update((state) => {
				// Check if device is already in watchlist
				const existingEntry = state.entries.find((entry) => entry.deviceId === deviceId);
				if (existingEntry) {
					console.log('Device already in watchlist:', deviceId);
					return state;
				}

				const newEntry: WatchlistEntry = {
					id: Date.now().toString(),
					deviceId,
					active: true
				};
				const newEntries = [...state.entries, newEntry];
				saveWatchlistToStorage(newEntries);
				notifyWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Remove device from watchlist
		remove: (id: string) => {
			update((state) => {
				const newEntries = state.entries.filter((entry) => entry.id !== id);
				saveWatchlistToStorage(newEntries);
				notifyWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Toggle device active state
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
					const deviceIdMap = new Map<string, WatchlistEntry>();

					// Handle both old format (with device objects) and new format (with deviceId)
					for (const entry of rawEntries) {
						let normalizedEntry: WatchlistEntry | null = null;

						if (entry.deviceId && entry.id) {
							// New format - just validate and use
							normalizedEntry = {
								id: entry.id,
								deviceId: entry.deviceId,
								active: entry.active
							};
						} else if (entry.device?.id && entry.id) {
							// Old format - convert to new format
							normalizedEntry = {
								id: entry.id,
								deviceId: entry.device.id,
								active: entry.active
							};
						}

						// Deduplicate by deviceId - keep the last entry for each deviceId
						if (normalizedEntry && normalizedEntry.deviceId) {
							deviceIdMap.set(normalizedEntry.deviceId, normalizedEntry);
						}
					}

					// Convert map back to array
					const deduplicatedEntries = Array.from(deviceIdMap.values());

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

// Create device registry store
function createDeviceRegistryStore() {
	const { subscribe, set, update } = writable<DeviceRegistryStore>(initialDeviceRegistryState);

	return {
		subscribe,

		// Add fixes to appropriate devices
		addFixes: (newFixes: Fix[]) => {
			update((state) => {
				// Log each fix to console
				newFixes.forEach(async (fix) => {
					console.log('Received fix:', fix);
					// Add fix to the device registry
					try {
						await state.registry.addFixToDevice(fix);
					} catch (error) {
						console.warn('Failed to add fix to device registry:', error);
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
		updateDeviceFromAPI: async (deviceId: string) => {
			update((state) => {
				state.registry.updateDeviceFromAPI(deviceId);
				return {
					registry: state.registry,
					lastUpdate: Date.now()
				};
			});
		},

		// Get a specific device
		getDevice: (deviceId: string): Device | null => {
			let currentDevice: Device | null = null;
			subscribe((state) => {
				currentDevice = state.registry.getDevice(deviceId);
			})();
			return currentDevice;
		},

		// Get all devices with recent fixes
		getActiveDevices: (): Device[] => {
			let devices: Device[] = [];
			subscribe((state) => {
				devices = state.registry.getAllDevices().filter((device) => {
					const latestFix = device.getLatestFix();
					if (!latestFix) return false;
					// Consider active if last fix was within the last hour
					const oneHourAgo = Date.now() - 60 * 60 * 1000;
					return new Date(latestFix.timestamp).getTime() > oneHourAgo;
				});
			})();
			return devices;
		},

		// Get fixes for a specific device
		getFixesForDevice: (deviceId: string): Fix[] => {
			let deviceFixes: Fix[] = [];
			subscribe((state) => {
				const device = state.registry.getDevice(deviceId);
				deviceFixes = device ? device.fixes : [];
			})();
			return deviceFixes;
		},

		// Clear all devices
		clear: () => {
			DeviceRegistry.getInstance().clear();
			set({
				registry: DeviceRegistry.getInstance(),
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
		subscribedDevices: connectionStatus.subscribedDevices,
		operationsPageActive: connectionStatus.operationsPageActive
	}));
}

// Notify FixFeed about watchlist changes
function notifyWatchlistChange(entries: WatchlistEntry[]) {
	if (!browser) return;

	const activeDeviceIds = entries
		.filter((entry) => entry.active && entry.deviceId)
		.map((entry) => entry.deviceId);

	// Update debug status with active watchlist entries
	debugStatus.update((current) => ({
		...current,
		activeWatchlistEntries: activeDeviceIds
	}));

	// Update FixFeed subscriptions based on watchlist
	fixFeed.subscribeToWatchlist(activeDeviceIds);

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
export const deviceRegistry = createDeviceRegistryStore();

// Backward compatibility - provide a way to get all recent fixes
export const fixes = {
	subscribe: deviceRegistry.subscribe,
	addFixes: deviceRegistry.addFixes,
	clear: deviceRegistry.clear,
	// Get all recent fixes from all devices
	getAllRecentFixes: (): Fix[] => {
		const devices = deviceRegistry.getActiveDevices();
		const allFixes: Fix[] = [];
		devices.forEach((device) => {
			allFixes.push(...device.fixes);
		});
		// Sort by timestamp, most recent first
		return allFixes.sort(
			(a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
		);
	}
};
