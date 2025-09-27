import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { WatchlistEntry, Fix, Device } from '$lib/types';

// Store interfaces
export interface WatchlistStore {
	entries: WatchlistEntry[];
}

export interface FixesStore {
	fixes: Fix[];
}

// Initial states
const initialWatchlistState: WatchlistStore = {
	entries: []
};

const initialFixesState: FixesStore = {
	fixes: []
};

// WebSocket connection state
let websocket: WebSocket | null = null;
let websocketUrl = '';
const currentlySubscribedDevices = new Set<string>();

// Create watchlist store
function createWatchlistStore() {
	const { subscribe, set, update } = writable<WatchlistStore>(initialWatchlistState);

	return {
		subscribe,

		// Add device to watchlist
		add: (device: Device) => {
			update((state) => {
				const newEntry: WatchlistEntry = {
					id: Date.now().toString(),
					device,
					active: true
				};
				const newEntries = [...state.entries, newEntry];
				saveWatchlistToStorage(newEntries);
				handleWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Remove device from watchlist
		remove: (id: string) => {
			update((state) => {
				const newEntries = state.entries.filter((entry) => entry.id !== id);
				saveWatchlistToStorage(newEntries);
				handleWatchlistChange(newEntries);
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
				handleWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Load from localStorage
		loadFromStorage: () => {
			if (!browser) return;

			const saved = localStorage.getItem('watchlist');
			if (saved) {
				try {
					const entries = (JSON.parse(saved) as WatchlistEntry[]).filter(
                        (entry) => entry.device?.id && entry.id
                    );
					set({ entries });
					handleWatchlistChange(entries);
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
			handleWatchlistChange(newEntries);
		}
	};
}

// Create fixes store
function createFixesStore() {
	const { subscribe, set, update } = writable<FixesStore>(initialFixesState);

	return {
		subscribe,

		// Add fixes to store
		addFixes: (newFixes: Fix[]) => {
			update((state) => {
				// Log each fix to console
				newFixes.forEach((fix) => {
					console.log('Received fix:', fix);
				});

				// Add to existing fixes (you might want to implement deduplication here)
				return { fixes: [...state.fixes, ...newFixes] };
			});
		},

		// Clear all fixes
		clear: () => {
			set(initialFixesState);
		},

		// Get fixes for a specific device
		getFixesForDevice: (deviceId: string) => {
			let currentFixes: Fix[] = [];
			subscribe((state) => {
				currentFixes = state.fixes.filter((fix) => fix.device_id === deviceId);
			})();
			return currentFixes;
		}
	};
}

// WebSocket management functions
function initializeWebSocket() {
	if (!browser) return;

	// Determine WebSocket URL based on current environment
	const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const host = window.location.host;
	websocketUrl = `${protocol}//${host}/fixes/live`;
}

function connectWebSocket() {
	if (!browser || websocket?.readyState === WebSocket.OPEN) return;

	try {
		websocket = new WebSocket(websocketUrl);

		websocket.onopen = () => {
			console.log('WebSocket connected to /fixes/live');
		};

		websocket.onmessage = (event) => {
			try {
				const fix = JSON.parse(event.data) as Fix;
				fixes.addFixes([fix]);
			} catch (e) {
				console.warn('Failed to parse WebSocket message:', e);
			}
		};

		websocket.onclose = () => {
			console.log('WebSocket disconnected');
			websocket = null;
			// Clear subscription tracking when connection is lost
			currentlySubscribedDevices.clear();
		};

		websocket.onerror = (error) => {
			console.error('WebSocket error:', error);
		};
	} catch (e) {
		console.error('Failed to create WebSocket connection:', e);
	}
}

function disconnectWebSocket() {
	if (websocket) {
		websocket.close();
		websocket = null;
	}
	// Clear subscription tracking when disconnecting
	currentlySubscribedDevices.clear();
}

async function subscribeToDevice(deviceId: string) {
	if (currentlySubscribedDevices.has(deviceId)) {
		return; // Already subscribed
	}

	if (!websocket || websocket.readyState !== WebSocket.OPEN) {
		connectWebSocket();
		// Wait a bit for connection to establish
		await new Promise((resolve) => setTimeout(resolve, 100));
	}

	if (websocket?.readyState === WebSocket.OPEN) {
		websocket.send(
			JSON.stringify({
				action: 'subscribe',
				device_id: deviceId
			})
		);

		// Track that we're now subscribed to this device
		currentlySubscribedDevices.add(deviceId);

		// Also fetch recent fixes from REST endpoint
		try {
			const response = await serverCall<{ fixes: Fix[] }>(`/fixes?device_id=${deviceId}&limit=100`);
			if (response.fixes) {
				fixes.addFixes(response.fixes);
			}
		} catch (error) {
			console.warn(`Failed to fetch fixes for device ${deviceId}:`, error);
		}
	}
}

async function unsubscribeFromDevice(deviceId: string) {
	if (!currentlySubscribedDevices.has(deviceId)) {
		return; // Not subscribed
	}

	if (websocket?.readyState === WebSocket.OPEN) {
		websocket.send(
			JSON.stringify({
				action: 'unsubscribe',
				device_id: deviceId
			})
		);
	}

	// Track that we're no longer subscribed to this device
	currentlySubscribedDevices.delete(deviceId);
}

// Handle watchlist changes - subscribe/unsubscribe as needed
async function handleWatchlistChange(entries: WatchlistEntry[]) {
	if (!browser) return;

	const desiredActiveDevices = new Set(
		entries.filter((entry) => entry.active && entry.device.id).map((entry) => entry.device.id!)
	);

	// Find devices to unsubscribe from (currently subscribed but not in desired set)
	const devicesToUnsubscribe = Array.from(currentlySubscribedDevices).filter(
		(deviceId) => !desiredActiveDevices.has(deviceId)
	);

	// Find devices to subscribe to (in desired set but not currently subscribed)
	const devicesToSubscribe = Array.from(desiredActiveDevices).filter(
		(deviceId) => !currentlySubscribedDevices.has(deviceId)
	);

	// Unsubscribe from devices no longer needed
	for (const deviceId of devicesToUnsubscribe) {
		await unsubscribeFromDevice(deviceId);
	}

	// Subscribe to new devices
	if (devicesToSubscribe.length > 0) {
		// Initialize WebSocket if needed
		initializeWebSocket();
		connectWebSocket();

		for (const deviceId of devicesToSubscribe) {
			await subscribeToDevice(deviceId);
		}
	}

	// If no devices are active, disconnect WebSocket
	if (desiredActiveDevices.size === 0) {
		disconnectWebSocket();
		currentlySubscribedDevices.clear();
	}
}

// Save watchlist to localStorage
function saveWatchlistToStorage(entries: WatchlistEntry[]) {
	if (!browser) return;
	localStorage.setItem('watchlist', JSON.stringify(entries));
}

// Create store instances
export const watchlist = createWatchlistStore();
export const fixes = createFixesStore();

// Initialize WebSocket URL when module loads
if (browser) {
	initializeWebSocket();
}
